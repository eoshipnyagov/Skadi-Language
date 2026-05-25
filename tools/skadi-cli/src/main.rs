mod commands;
mod pipeline;
mod project;

use std::env;

fn print_help() {
    println!("skadi-cli v0.1");
    println!("Usage:");
    println!("  skadi <command> [args]");
    println!();
    println!("Commands:");
    println!("  new <name>         Create a new Skadi project");
    println!("  init               Initialize Skadi project in current directory");
    println!("  check              Run frontend checks");
    println!("  build [--target]   Build project");
    println!("  run                Build and run project");
    println!("  target list        List supported targets");
    println!("  tui                Interactive mode");
    println!("  format             Format source files (planned)");
    println!("  doctor             Verify toolchain environment (planned)");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(cmd) = args.get(1).map(|s| s.as_str()) else {
        print_help();
        return;
    };

    let result = match cmd {
        "new" => commands::new_cmd::run(&args[2..]),
        "init" => commands::init_cmd::run(&args[2..]),
        "check" => commands::check_cmd::run(&args[2..]),
        "build" => commands::build_cmd::run(&args[2..]),
        "run" => commands::run_cmd::run(&args[2..]),
        "target" => commands::target_cmd::run(&args[2..]),
        "tui" => commands::tui_cmd::run(&args[2..]),
        "format" => commands::format_cmd::run(&args[2..]),
        "doctor" => commands::doctor_cmd::run(&args[2..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => Err(format!("Unknown command: {cmd}. Use 'skadi help'.")),
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
