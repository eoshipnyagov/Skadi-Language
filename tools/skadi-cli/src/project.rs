use std::fs;
use std::path::{Path, PathBuf};

pub struct ProjectConfig {
    pub root: PathBuf,
    pub name: String,
    pub entry: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestConfig {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub entry: String,
}

const TEMPLATE_MAIN: &str = "new Text greeting = concat(\"Hello\", \" from Skadi v1.1\")\noutput(greeting)\nnew Float quarter_turn = deg_to_rad(90)\noutput(quarter_turn)\n";

pub fn load_project_at(root: &Path) -> Result<ProjectConfig, String> {
    let manifest = load_manifest_config_at(root)?;
    let entry = root.join(&manifest.entry);

    Ok(ProjectConfig {
        root: root.to_path_buf(),
        name: manifest.name,
        entry,
    })
}

pub fn load_manifest_config_at(root: &Path) -> Result<ManifestConfig, String> {
    let manifest = root.join("Skadi.toml");
    let content = fs::read_to_string(&manifest)
        .map_err(|e| format!("failed to read {}: {e}", manifest.display()))?;

    Ok(ManifestConfig {
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
    })
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
        "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"{}\"\n\n[build]\nentry = \"{}\"\n",
        manifest.name, manifest.version, manifest.edition, manifest.entry
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        ManifestConfig, ensure_entry_file_at, init_project, load_manifest_config_at,
        save_manifest_config_at,
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
}
