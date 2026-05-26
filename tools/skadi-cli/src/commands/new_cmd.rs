use std::fs;
use std::path::{Path, PathBuf};

const TEMPLATE_TOML: &str = "[package]\nname = \"__NAME__\"\ntype = \"__TYPE__\"\nversion = \"0.1.0\"\nedition = \"v1\"\n\n[build]\nentry = \"src/main.skd\"\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let (project_type, name) = match args {
        [name] => ("console".to_string(), name.clone()),
        [project_type, name] => {
            let ty = normalize_type(project_type)?;
            (ty, name.clone())
        }
        _ => {
            return Err(
                "Usage: skadi new <name> | skadi new <type> <name>\nTypes: game, embedded, console, gui"
                    .to_string(),
            )
        }
    };

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
    fs::write(&main_path, template_main(project_type))
        .map_err(|e| format!("write {} failed: {e}", main_path.display()))?;

    let toml_path = root.join("Skadi.toml");
    let toml = TEMPLATE_TOML
        .replace("__NAME__", name)
        .replace("__TYPE__", project_type);
    fs::write(&toml_path, toml).map_err(|e| format!("write {} failed: {e}", toml_path.display()))?;

    let gitignore_path = root.join(".gitignore");
    fs::write(&gitignore_path, "build/\n*.c\n*.exe\n")
        .map_err(|e| format!("write {} failed: {e}", gitignore_path.display()))?;

    Ok(())
}

pub fn normalize_type(input: &str) -> Result<String, String> {
    match input {
        "game" | "embedded" | "console" | "gui" => Ok(input.to_string()),
        _ => Err(format!(
            "unknown project type '{}'. allowed: game, embedded, console, gui",
            input
        )),
    }
}

fn template_main(project_type: &str) -> &'static str {
    match project_type {
        "game" => "new Int frame = 0\nloop {\n    output(frame)\n    frame = frame + 1\n    if frame >= 3 {\n        break\n    }\n}\n",
        "embedded" => "new Int tick = 0\nloop {\n    tick = tick + 1\n    if tick >= 5 {\n        break\n    }\n}\n",
        "gui" => "new Text title = \"Skadi GUI App\"\noutput(title)\n",
        _ => "output(\"Hello from Skadi!\")\n",
    }
}
