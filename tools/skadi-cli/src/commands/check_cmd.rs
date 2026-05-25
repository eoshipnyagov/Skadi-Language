use crate::pipeline::compile_to_c;
use crate::project::load_project;

pub fn run(_args: &[String]) -> Result<(), String> {
    let project = load_project()?;
    let _c = compile_to_c(&project.entry)?;
    println!("check ok: {}", project.entry.display());
    Ok(())
}
