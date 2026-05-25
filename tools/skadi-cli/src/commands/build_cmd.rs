use std::fs;
use std::path::PathBuf;

use crate::pipeline::{compile_c_to_exe, compile_to_c};
use crate::project::{ensure_build_dir, load_project};

pub fn run(args: &[String]) -> Result<(), String> {
    let mut target = "host".to_string();
    let mut i = 0usize;
    while i < args.len() {
        if args[i] == "--target" {
            if i + 1 >= args.len() {
                return Err("--target requires value".to_string());
            }
            target = args[i + 1].clone();
            i += 2;
            continue;
        }
        i += 1;
    }

    let project = load_project()?;
    let c_code = compile_to_c(&project.entry)?;
    let build_dir = ensure_build_dir(&project.root)?;
    let c_path = build_dir.join(format!("{}.c", project.name));
    fs::write(&c_path, c_code).map_err(|e| format!("write {} failed: {e}", c_path.display()))?;

    let exe_name = if cfg!(windows) {
        format!("{}.exe", project.name)
    } else {
        project.name.clone()
    };
    let exe_path: PathBuf = build_dir.join(exe_name);
    compile_c_to_exe(&c_path, &exe_path, &target)?;
    println!("build ok: {}", exe_path.display());
    Ok(())
}
