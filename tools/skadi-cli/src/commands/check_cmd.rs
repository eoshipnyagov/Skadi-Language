use crate::actions;

pub fn run(_args: &[String]) -> Result<(), String> {
    let result = actions::run_check().map_err(|e| e.to_string())?;
    println!("check ok: {}", result.entry.display());
    Ok(())
}
