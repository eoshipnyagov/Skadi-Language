use std::fs;
use std::path::PathBuf;

use crate::templates::{main_template, manifest_content, normalize_project_type};

pub fn run(args: &[String]) -> Result<(), String> {
    let project_type = if let Some(raw) = args.first() {
        normalize_project_type(raw)?
    } else {
        "console".to_string()
    };

    let cwd = std::env::current_dir().map_err(|e| format!("cwd failed: {e}"))?;
    let src = cwd.join("src");
    if !src.exists() {
        fs::create_dir_all(&src).map_err(|e| format!("create {} failed: {e}", src.display()))?;
    }

    let main_path = src.join("main.skd");
    if !main_path.exists() {
        fs::write(&main_path, main_template(&project_type))
            .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;
    }

    let toml_path = cwd.join("Skadi.toml");
    if !toml_path.exists() {
        let project_name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("skadi_project");
        let toml = manifest_content(project_name, &project_type);
        fs::write(&toml_path, toml).map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;
    }

    let gitignore_path = PathBuf::from(".gitignore");
    if !gitignore_path.exists() {
        fs::write(&gitignore_path, "build/\n*.c\n*.exe\n")
            .map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;
    }

    println!("Initialized Skadi project in {}", cwd.display());
    Ok(())
}


