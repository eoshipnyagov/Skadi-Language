use crate::actions;

pub fn run(args: &[String]) -> Result<(), String> {
    let options = actions::parse_build_options(args).map_err(|e| e.to_string())?;
    let result = actions::run_project(&options).map_err(|e| e.to_string())?;
    if let Some(selected) = result.build.requested_compiler.clone() {
        println!(
            "build ok [{}] (cc={}): {}",
            result.build.target,
            selected,
            result.build.exe_path.display()
        );
    } else {
        println!(
            "build ok [{}]: {}",
            result.build.target,
            result.build.exe_path.display()
        );
    }
    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprint!("{}", result.stderr);
    }
    Ok(())
}
