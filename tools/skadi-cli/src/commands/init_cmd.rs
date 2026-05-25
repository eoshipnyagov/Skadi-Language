use std::fs;
use std::path::PathBuf;

pub fn run(_args: &[String]) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd failed: {e}"))?;
    let src = cwd.join("src");
    if !src.exists() {
        fs::create_dir_all(&src).map_err(|e| format!("create {} failed: {e}", src.display()))?;
    }

    let main_path = src.join("main.scadi");
    if !main_path.exists() {
        fs::write(&main_path, "output(\"Hello from Skadi!\")\n")
            .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;
    }

    let toml_path = cwd.join("Skadi.toml");
    if !toml_path.exists() {
        let project_name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("skadi_project");
        let toml = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"v1\"\n\n[build]\nentry = \"src/main.scadi\"\n",
            project_name
        );
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
