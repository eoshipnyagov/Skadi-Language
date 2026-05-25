pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        Some("list") => {
            println!("host");
            println!("x86_64-unknown-linux-gnu");
            println!("x86_64-pc-windows-msvc");
            println!("aarch64-unknown-linux-gnu");
            Ok(())
        }
        _ => Err("Usage: skadi target list".to_string()),
    }
}
