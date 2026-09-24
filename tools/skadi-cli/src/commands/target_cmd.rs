use crate::actions;

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            let result = actions::list_targets();
            for p in result.targets {
                println!("{}    {}\n    {}", p.triple, p.description, p.capabilities);
            }
            Ok(())
        }
        _ => Err("Usage: skadi target list".to_string()),
    }
}
