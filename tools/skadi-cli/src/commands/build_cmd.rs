use std::fs;
use std::path::PathBuf;

use crate::pipeline::{compile_c_to_exe, compile_to_c};
use crate::project::{ensure_build_dir, load_project};
use crate::targets::{os_install_hint, resolve_profile, shell_probe_hint, target_hint, OutputKind};

fn parse_build_args(args: &[String]) -> Result<(String, Option<String>), String> {
    let mut target = "host".to_string();
    let mut cc: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                if i + 1 >= args.len() {
                    return Err("--target requires value".to_string());
                }
                target = args[i + 1].clone();
                i += 2;
            }
            "--cc" => {
                if i + 1 >= args.len() {
                    return Err("--cc requires value".to_string());
                }
                cc = Some(args[i + 1].clone());
                i += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown build option: {other}"));
            }
            _ => i += 1,
        }
    }
    Ok((target, cc))
}

pub fn run(args: &[String]) -> Result<(), String> {
    let (target, cc) = parse_build_args(args)?;

    let profile = resolve_profile(&target)?;
    let project = load_project()?;
    let c_code = compile_to_c(&project.entry)?;
    let build_dir = ensure_build_dir(&project.root)?;
    let c_path = build_dir.join(format!("{}.c", project.name));
    fs::write(&c_path, c_code).map_err(|e| format!("write {} failed: {e}", c_path.display()))?;

    let exe_name = match profile.output_kind {
        OutputKind::WindowsExe => format!("{}.exe", project.name),
        OutputKind::LinuxElf => project.name.clone(),
    };
    let exe_path: PathBuf = build_dir.join(exe_name);
    if let Err(e) = compile_c_to_exe(&c_path, &exe_path, &target, cc.as_deref()) {
        let compiler_info = cc
            .as_ref()
            .map(|v| format!("requested compiler: {v}"))
            .unwrap_or_else(|| "auto-detect order: gcc -> clang -> cc (and cl on Windows host)".to_string());
        return Err(format!(
            "{}\n{}\nhint: {}\ninstall: {}\nprobe: try 'gcc --version', 'clang --version', and '{}'",
            e,
            compiler_info,
            target_hint(profile.triple),
            os_install_hint(),
            shell_probe_hint(),
        ));
    }
    if let Some(selected) = cc {
        println!("build ok [{}] (cc={}): {}", profile.triple, selected, exe_path.display());
    } else {
        println!("build ok [{}]: {}", profile.triple, exe_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_build_args;

    #[test]
    fn parse_target_and_cc() {
        let args = vec![
            "--target".to_string(),
            "host".to_string(),
            "--cc".to_string(),
            "gcc".to_string(),
        ];
        let (target, cc) = parse_build_args(&args).expect("args should parse");
        assert_eq!(target, "host");
        assert_eq!(cc.as_deref(), Some("gcc"));
    }

    #[test]
    fn parse_missing_cc_value() {
        let args = vec!["--cc".to_string()];
        let err = parse_build_args(&args).expect_err("should fail");
        assert!(err.contains("--cc requires value"));
    }

    #[test]
    fn parse_unknown_option() {
        let args = vec!["--zzz".to_string()];
        let err = parse_build_args(&args).expect_err("should fail");
        assert!(err.contains("unknown build option"));
    }
}
