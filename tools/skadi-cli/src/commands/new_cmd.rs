use crate::actions;

pub fn run(args: &[String]) -> Result<(), String> {
    let Some(input) = args.first() else {
        return Err("Usage: skadi new <project_name>".to_string());
    };

    let result = actions::create_new_project(input).map_err(|e| e.to_string())?;
    println!("Created Skadi project: {}", result.root.display());
    Ok(())
}
