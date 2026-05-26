use std::fs;
use std::path::{Path, PathBuf};

use crate::templates::{main_template, manifest_content, normalize_project_type};

pub fn run(args: &[String]) -> Result<(), String> {
    let (project_type, name) = parse_new_args(args)?;

    let root = PathBuf::from(name);
    if root.exists() {
        return Err(format!("Directory already exists: {}", root.display()));
    }
    let project_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "invalid project path/name".to_string())?
        .to_string();

    create_project(&root, &project_name, &project_type)?;
    println!("Created Skadi project [{}]: {}", project_type, root.display());
    Ok(())
}

pub fn create_project(root: &Path, name: &str, project_type: &str) -> Result<(), String> {
    fs::create_dir_all(root.join("src")).map_err(|e| format!("create dirs failed: {e}"))?;

    let main_path = root.join("src/main.skd");
    fs::write(&main_path, main_template(project_type))
        .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;

    let toml_path = root.join("Skadi.toml");
    let toml = manifest_content(name, project_type);
    fs::write(&toml_path, toml).map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;

    let gitignore_path = root.join(".gitignore");
    fs::write(&gitignore_path, "build/\n*.c\n*.exe\n")
        .map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;

    Ok(())
}

fn parse_new_args(args: &[String]) -> Result<(String, String), String> {
    match args {
        [name] => Ok(("console".to_string(), name.clone())),
        [project_type, name] => {
            let ty = normalize_project_type(project_type)?;
            Ok((ty, name.clone()))
        }
        _ => Err(
            "Usage: skadi new <name> | skadi new <type> <name>\nTypes: game, embedded, console, gui"
                .to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_new_args;

    #[test]
    fn parse_new_args_default_console() {
        let args = vec!["demo".to_string()];
        let (ty, name) = parse_new_args(&args).expect("parse");
        assert_eq!(ty, "console");
        assert_eq!(name, "demo");
    }

    #[test]
    fn parse_new_args_typed() {
        let args = vec!["game".to_string(), "demo".to_string()];
        let (ty, name) = parse_new_args(&args).expect("parse");
        assert_eq!(ty, "game");
        assert_eq!(name, "demo");
    }

    #[test]
    fn parse_new_args_rejects_invalid_type() {
        let args = vec!["web".to_string(), "demo".to_string()];
        let err = parse_new_args(&args).expect_err("must reject");
        assert!(err.contains("unknown project type"));
    }

    #[test]
    fn parse_new_args_rejects_bad_arity() {
        let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let err = parse_new_args(&args).expect_err("must reject");
        assert!(err.contains("Usage: skadi new"));
    }
}
