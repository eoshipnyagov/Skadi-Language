use std::fs;
use std::path::{Path, PathBuf};

pub struct ProjectConfig {
    pub root: PathBuf,
    pub name: String,
    pub entry: PathBuf,
    pub int_width: String,
    pub native_sources: Vec<String>,
    pub native_libraries: Vec<String>,
    pub native_library_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestConfig {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
    pub int_width: String,
    pub native_sources: Vec<String>,
    pub native_libraries: Vec<String>,
    pub native_library_paths: Vec<String>,
}

const TEMPLATE_MAIN: &str = "new Text greeting = \"Hello from Skadi\"\n\noutput(greeting)\n\nnew Angle quarter_turn = 90deg\n\noutput(\"Quarter turn: \", rad_to_deg(quarter_turn), \" degrees\")\n";

pub fn load_project_at(root: &Path) -> Result<ProjectConfig, String> {
    let manifest = load_manifest_config_at(root)?;
    let entry = root.join(&manifest.entry);

    Ok(ProjectConfig {
        root: root.to_path_buf(),
        name: manifest.name,
        entry,
        int_width: manifest.int_width,
        native_sources: manifest.native_sources,
        native_libraries: manifest.native_libraries,
        native_library_paths: manifest.native_library_paths,
    })
}

pub fn load_manifest_config_at(root: &Path) -> Result<ManifestConfig, String> {
    let manifest = root.join("Skadi.toml");
    let content = fs::read_to_string(&manifest)
        .map_err(|e| format!("failed to read {}: {e}", manifest.display()))?;

    let config = ManifestConfig {
        name: extract_string_value(&content, "name").unwrap_or_else(|| {
            root.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("skadi_project")
                .to_string()
        }),
        version: extract_string_value(&content, "version").unwrap_or_else(|| "0.1.0".to_string()),
        edition: extract_string_value(&content, "edition").unwrap_or_else(|| "v1".to_string()),
        entry: extract_string_value(&content, "entry")
            .unwrap_or_else(|| "src/main.skd".to_string()),
        int_width: extract_section_string_value(&content, "numeric", "int")
            .unwrap_or_else(|| "target".to_string()),
        native_sources: extract_section_string_array(&content, "native", "sources")?,
        native_libraries: extract_section_string_array(&content, "native", "libraries")?,
        native_library_paths: extract_section_string_array(&content, "native", "library_paths")?,
    };
    validate_manifest_config(&config)?;
    Ok(config)
}

pub fn save_manifest_config_at(root: &Path, manifest: &ManifestConfig) -> Result<(), String> {
    validate_manifest_config(manifest)?;
    let manifest_path = root.join("Skadi.toml");
    fs::write(&manifest_path, render_manifest_config(manifest))
        .map_err(|e| format!("write {} failed: {e}", manifest_path.display()))
}

pub fn ensure_entry_file_at(root: &Path, entry: &str) -> Result<PathBuf, String> {
    let entry_path = root.join(entry);
    if let Some(parent) = entry_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create {} failed: {e}", parent.display()))?;
    }
    if !entry_path.exists() {
        fs::write(&entry_path, TEMPLATE_MAIN)
            .map_err(|e| format!("write {} failed: {e}", entry_path.display()))?;
    }
    Ok(entry_path)
}

fn extract_string_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(key) {
            continue;
        }
        let mut parts = trimmed.splitn(2, '=');
        let _left = parts.next()?;
        let right = parts.next()?.trim();
        if right.starts_with('\"') && right.ends_with('\"') && right.len() >= 2 {
            return Some(right[1..right.len() - 1].to_string());
        }
    }
    None
}

fn extract_section_string_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut current_section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }
        if current_section != section || !trimmed.starts_with(key) {
            continue;
        }
        let (_, right) = trimmed.split_once('=')?;
        let right = right.trim();
        if right.starts_with('"') && right.ends_with('"') && right.len() >= 2 {
            return Some(right[1..right.len() - 1].to_string());
        }
    }
    None
}

fn extract_section_string_array(
    content: &str,
    section: &str,
    key: &str,
) -> Result<Vec<String>, String> {
    let mut current_section = "";
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }
        if current_section != section {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() != key {
            continue;
        }
        let value = right.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return Err(format!(
                "manifest field '{section}.{key}' must be a one-line string array"
            ));
        }
        let inner = value[1..value.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Vec::new());
        }
        let mut values = Vec::new();
        for item in inner.split(',') {
            let item = item.trim();
            if item.len() < 2 || !item.starts_with('"') || !item.ends_with('"') {
                return Err(format!(
                    "manifest field '{section}.{key}' accepts only quoted string values"
                ));
            }
            values.push(item[1..item.len() - 1].to_string());
        }
        return Ok(values);
    }
    Ok(Vec::new())
}

pub fn ensure_build_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join("build");
    fs::create_dir_all(&dir).map_err(|e| format!("create {} failed: {e}", dir.display()))?;
    Ok(dir)
}

pub fn create_project(root: &Path, name: &str) -> Result<(), String> {
    fs::create_dir_all(root.join("src")).map_err(|e| format!("create dirs failed: {e}"))?;

    let main_path = root.join("src/main.skd");
    fs::write(&main_path, TEMPLATE_MAIN)
        .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;

    let toml_path = root.join("Skadi.toml");
    let manifest = ManifestConfig {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        edition: "v1".to_string(),
        entry: "src/main.skd".to_string(),
        int_width: "target".to_string(),
        native_sources: Vec::new(),
        native_libraries: Vec::new(),
        native_library_paths: Vec::new(),
    };
    fs::write(&toml_path, render_manifest_config(&manifest))
        .map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;

    let gitignore_path = root.join(".gitignore");
    fs::write(&gitignore_path, "build/\n*.c\n*.exe\n")
        .map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;

    Ok(())
}

pub fn init_project(root: &Path) -> Result<(), String> {
    let src = root.join("src");
    if !src.exists() {
        fs::create_dir_all(&src).map_err(|e| format!("create {} failed: {e}", src.display()))?;
    }

    let main_path = src.join("main.skd");
    if !main_path.exists() {
        fs::write(&main_path, TEMPLATE_MAIN)
            .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;
    }

    let toml_path = root.join("Skadi.toml");
    if !toml_path.exists() {
        let project_name = root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("skadi_project");
        let manifest = ManifestConfig {
            name: project_name.to_string(),
            version: "0.1.0".to_string(),
            edition: "v1".to_string(),
            entry: "src/main.skd".to_string(),
            int_width: "target".to_string(),
            native_sources: Vec::new(),
            native_libraries: Vec::new(),
            native_library_paths: Vec::new(),
        };
        fs::write(&toml_path, render_manifest_config(&manifest))
            .map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;
    }

    let gitignore_path = root.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(&gitignore_path, "build/\n*.c\n*.exe\n")
            .map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;
    }

    Ok(())
}

fn render_manifest_config(manifest: &ManifestConfig) -> String {
    format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"{}\"\n\n[build]\nentry = \"{}\"\n\n[numeric]\nint = \"{}\"\n\n[native]\nsources = {}\nlibraries = {}\nlibrary_paths = {}\n",
        manifest.name,
        manifest.version,
        manifest.edition,
        manifest.entry,
        manifest.int_width,
        render_string_array(&manifest.native_sources),
        render_string_array(&manifest.native_libraries),
        render_string_array(&manifest.native_library_paths),
    )
}

fn render_string_array(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| format!("\"{value}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validate_manifest_config(manifest: &ManifestConfig) -> Result<(), String> {
    if manifest.name.trim().is_empty() {
        return Err("manifest field 'name' cannot be empty".to_string());
    }
    if manifest.version.trim().is_empty() {
        return Err("manifest field 'version' cannot be empty".to_string());
    }
    if manifest.edition.trim().is_empty() {
        return Err("manifest field 'edition' cannot be empty".to_string());
    }
    if manifest.entry.trim().is_empty() {
        return Err("manifest field 'entry' cannot be empty".to_string());
    }
    if !matches!(
        manifest.int_width.trim(),
        "target" | "i8" | "i16" | "i32" | "i64"
    ) {
        return Err(
            "manifest field 'numeric.int' must be target, i8, i16, i32, or i64".to_string(),
        );
    }
    for source in &manifest.native_sources {
        validate_native_relative_path(source, "native.sources")?;
        if !source.ends_with(".c") {
            return Err(format!(
                "manifest field 'native.sources' accepts only .c files, got '{source}'"
            ));
        }
    }
    for path in &manifest.native_library_paths {
        validate_native_relative_path(path, "native.library_paths")?;
    }
    for library in &manifest.native_libraries {
        if library.is_empty()
            || !library.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            return Err(format!(
                "manifest field 'native.libraries' contains invalid library name '{library}'"
            ));
        }
    }
    Ok(())
}

fn validate_native_relative_path(value: &str, field: &str) -> Result<(), String> {
    let path = Path::new(value);
    let bytes = value.as_bytes();
    let has_windows_drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    let has_portable_root = value.starts_with('/') || value.starts_with('\\');
    let has_parent = value.split(['/', '\\']).any(|component| component == "..");
    if value.trim().is_empty()
        || path.is_absolute()
        || has_windows_drive
        || has_portable_root
        || has_parent
    {
        return Err(format!(
            "manifest field '{field}' requires a project-relative path and must not contain '..', got '{value}'"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        ManifestConfig, ensure_entry_file_at, init_project, load_manifest_config_at,
        save_manifest_config_at, validate_manifest_config,
    };

    fn unique_temp_dir(stem: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_millis();
        let dir = std::env::temp_dir().join(format!("skadi_project_{stem}_{stamp}"));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn manifest_roundtrip_preserves_canonical_fields() {
        let temp = unique_temp_dir("manifest");
        init_project(&temp).expect("init");

        let updated = ManifestConfig {
            name: "demo".to_string(),
            version: "1.2.3".to_string(),
            edition: "v1".to_string(),
            entry: "src/app.skd".to_string(),
            int_width: "i16".to_string(),
            native_sources: vec!["native/helper.c".to_string()],
            native_libraries: vec!["helper".to_string()],
            native_library_paths: vec!["native/lib".to_string()],
        };
        save_manifest_config_at(&temp, &updated).expect("save");
        let loaded = load_manifest_config_at(&temp).expect("load");
        assert_eq!(loaded, updated);

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn ensure_entry_file_creates_missing_parent_and_file() {
        let temp = unique_temp_dir("entry_file");
        let entry = ensure_entry_file_at(&temp, "src/nested/app.skd").expect("entry");
        assert!(entry.exists());
        let source = std::fs::read_to_string(&entry).expect("entry source");
        assert!(source.contains("Hello"));

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn native_manifest_rejects_unsafe_paths_and_flag_like_library_names() {
        let base = ManifestConfig {
            name: "demo".to_string(),
            version: "0.1.0".to_string(),
            edition: "v1".to_string(),
            entry: "src/main.skd".to_string(),
            int_width: "target".to_string(),
            native_sources: Vec::new(),
            native_libraries: Vec::new(),
            native_library_paths: Vec::new(),
        };

        let mut absolute = base.clone();
        absolute.native_sources = vec!["/outside/helper.c".to_string()];
        assert!(
            validate_manifest_config(&absolute)
                .expect_err("absolute path must fail")
                .contains("project-relative")
        );

        let mut parent = base.clone();
        parent.native_sources = vec!["native/../helper.c".to_string()];
        assert!(
            validate_manifest_config(&parent)
                .expect_err("parent traversal must fail")
                .contains("must not contain '..'")
        );

        let mut flags = base;
        flags.native_libraries = vec!["-Wl=unsafe".to_string()];
        assert!(
            validate_manifest_config(&flags)
                .expect_err("linker flag must fail")
                .contains("invalid library name")
        );
    }
}
