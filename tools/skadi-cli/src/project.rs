use std::fs;
use std::path::{Path, PathBuf};

pub struct ProjectConfig {
    pub root: PathBuf,
    pub name: String,
    pub project_type: String,
    pub entry: PathBuf,
}

pub fn load_project() -> Result<ProjectConfig, String> {
    let root = std::env::current_dir().map_err(|e| format!("cwd failed: {e}"))?;
    let manifest = root.join("Skadi.toml");
    let content = fs::read_to_string(&manifest)
        .map_err(|e| format!("failed to read {}: {e}", manifest.display()))?;

    let name = extract_string_value(&content, "name").unwrap_or_else(|| {
        root.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("skadi_project")
            .to_string()
    });
    let project_type = extract_string_value(&content, "type").unwrap_or_else(|| "console".to_string());
    let entry_str = extract_string_value(&content, "entry").unwrap_or_else(|| "src/main.skd".to_string());
    let entry = root.join(entry_str);

    Ok(ProjectConfig { root, name, project_type, entry })
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


