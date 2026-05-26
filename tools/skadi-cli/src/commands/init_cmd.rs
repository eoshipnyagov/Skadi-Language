use std::fs;
use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<(), String> {
    let project_type = if let Some(raw) = args.first() {
        super::new_cmd::normalize_type(raw)?
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
        let main = match project_type.as_str() {
            "game" => "new Int frame = 0\nloop {\n    output(frame)\n    frame = frame + 1\n    if frame >= 3 {\n        break\n    }\n}\n",
            "embedded" => "new Int tick = 0\nloop {\n    tick = tick + 1\n    if tick >= 5 {\n        break\n    }\n}\n",
            "gui" => "new Text title = \"Skadi GUI App\"\noutput(title)\n",
            _ => "output(\"Hello from Skadi!\")\n",
        };
        fs::write(&main_path, main)
            .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;
    }

    let toml_path = cwd.join("Skadi.toml");
    if !toml_path.exists() {
        let project_name = cwd
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("skadi_project");
        let toml = format!(
            "[package]\nname = \"{}\"\ntype = \"{}\"\nversion = \"0.1.0\"\nedition = \"v1\"\n\n[build]\nentry = \"src/main.skd\"\n",
            project_name, project_type
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


