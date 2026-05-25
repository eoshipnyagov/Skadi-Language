use std::process::Command;

use crate::commands::build_cmd;
use crate::project::{ensure_build_dir, load_project};

pub fn run(args: &[String]) -> Result<(), String> {
    build_cmd::run(args)?;
    let project = load_project()?;
    let build_dir = ensure_build_dir(&project.root)?;
    let exe_name = if cfg!(windows) {
        format!("{}.exe", project.name)
    } else {
        project.name
    };
    let exe_path = build_dir.join(exe_name);
    let status = Command::new(&exe_path)
        .status()
        .map_err(|e| format!("failed to run {}: {e}", exe_path.display()))?;
    if !status.success() {
        return Err(format!("program exited with status {status}"));
    }
    Ok(())
}
