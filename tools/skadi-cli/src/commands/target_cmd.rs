use crate::targets::builtin_profiles;

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            for p in builtin_profiles() {
                println!("{}    {}", p.triple, p.description);
            }
            Ok(())
        }
        _ => Err("Usage: skadi target list".to_string()),
    }
}
