use std::fs;
use std::path::{Path, PathBuf};

const TEMPLATE_MAIN: &str = "output(\"Hello from Skadi!\")\n";
const TEMPLATE_TOML: &str = "[package]\nname = \"__NAME__\"\nversion = \"0.1.0\"\nedition = \"v1\"\n\n[build]\nentry = \"src/main.skd\"\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let Some(input) = args.first() else {
        return Err("Usage: skadi new <project_name>".to_string());
    };

    let root = PathBuf::from(input);
    if root.exists() {
        return Err(format!("Directory already exists: {}", root.display()));
    }
    let project_name = root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "invalid project path/name".to_string())?
        .to_string();

    create_project(&root, &project_name)?;
    println!("Created Skadi project: {}", root.display());
    Ok(())
}

pub fn create_project(root: &Path, name: &str) -> Result<(), String> {
    fs::create_dir_all(root.join("src")).map_err(|e| format!("create dirs failed: {e}"))?;

    let main_path = root.join("src/main.skd");
    fs::write(&main_path, TEMPLATE_MAIN).map_err(|e| format!("write {} failed: {e}", main_path.display()))?;

    let toml_path = root.join("Skadi.toml");
    let toml = TEMPLATE_TOML.replace("__NAME__", name);
    fs::write(&toml_path, toml).map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;

    let gitignore_path = root.join(".gitignore");
    fs::write(&gitignore_path, "build/\n*.c\n*.exe\n").map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;

    Ok(())
}


