use std::io::{self, Write};

pub fn run(_args: &[String]) -> Result<(), String> {
    println!("Skadi TUI (minimal)\n");
    println!("1) New project");
    println!("2) Init current directory");
    println!("3) Build (planned)");
    print!("Select [1-3]: ");
    io::stdout().flush().map_err(|e| format!("flush failed: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("read failed: {e}"))?;

    match input.trim() {
        "1" => {
            print!("Project name: ");
            io::stdout().flush().map_err(|e| format!("flush failed: {e}"))?;
            let mut name = String::new();
            io::stdin()
                .read_line(&mut name)
                .map_err(|e| format!("read failed: {e}"))?;
            let name = name.trim();
            if name.is_empty() {
                return Err("project name cannot be empty".to_string());
            }
            super::new_cmd::run(&[name.to_string()])
        }
        "2" => super::init_cmd::run(&[]),
        "3" => {
            println!("Build flow in TUI is planned.");
            Ok(())
        }
        _ => Err("unknown selection".to_string()),
    }
}
