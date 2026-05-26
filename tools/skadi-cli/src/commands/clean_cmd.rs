use std::fs;
use std::path::PathBuf;

pub fn run(args: &[String]) -> Result<(), String> {
    let mut deep = false;
    for arg in args {
        match arg.as_str() {
            "--all" | "--deep" => deep = true,
            _ => return Err(format!("unknown clean option: {arg}. supported: --all")),
        }
    }

    let root = std::env::current_dir().map_err(|e| format!("cwd failed: {e}"))?;
    let mut removed: Vec<PathBuf> = Vec::new();

    remove_dir_if_exists(root.join("build"), &mut removed)?;
    remove_globbed_files(&root, "bench_*.exe", &mut removed)?;
    remove_globbed_files(&root, "*.scadi.c", &mut removed)?;

    if deep {
        remove_dir_if_exists(root.join("target"), &mut removed)?;
        remove_dir_if_exists(root.join("tools").join("skadi-cli").join("target"), &mut removed)?;
    }

    if removed.is_empty() {
        println!("clean: nothing to remove");
    } else {
        for path in removed {
            println!("clean: removed {}", path.display());
        }
    }
    Ok(())
}

fn remove_dir_if_exists(path: PathBuf, removed: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| format!("remove {} failed: {e}", path.display()))?;
        removed.push(path);
    }
    Ok(())
}

fn remove_globbed_files(root: &std::path::Path, suffix_pattern: &str, removed: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(root).map_err(|e| format!("read_dir {} failed: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry failed: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|x| x.to_str()) else {
            continue;
        };
        if wildcard_match(name, suffix_pattern) {
            fs::remove_file(&path).map_err(|e| format!("remove {} failed: {e}", path.display()))?;
            removed.push(path);
        }
    }
    Ok(())
}

fn wildcard_match(name: &str, pattern: &str) -> bool {
    if pattern == "bench_*.exe" {
        return name.starts_with("bench_") && name.ends_with(".exe");
    }
    if pattern == "*.scadi.c" {
        return name.ends_with(".scadi.c");
    }
    false
}
