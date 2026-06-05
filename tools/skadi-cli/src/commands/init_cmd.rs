use crate::actions;

pub fn run(_args: &[String]) -> Result<(), String> {
    let result = actions::init_current_project().map_err(|e| e.to_string())?;
    println!("Initialized Skadi project in {}", result.root.display());
    Ok(())
}
