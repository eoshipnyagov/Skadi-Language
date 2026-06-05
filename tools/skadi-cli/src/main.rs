mod actions;
mod commands;
mod pipeline;
mod project;
mod targets;
mod tui;

use std::env;

fn help_text() -> String {
    [
        "skadi-cli v1.1",
        "Canonical CLI workflow for Skadi v1.1.",
        "Usage:",
        "  skadi <command> [args]",
        "",
        "Commands:",
        "  new <name>         Create a new Skadi project",
        "  init               Initialize Skadi project in current directory",
        "  check              Run frontend checks",
        "  build [--target] [--cc]  Build project",
        "  run [--target] [--cc]    Build and run project",
        "  target list        List supported targets",
        "  tui                Full-screen interactive workflow",
        "  format [--check] [path ...]  Format Skadi source files",
        "  doctor             Verify toolchain environment",
    ]
    .join("\n")
}

fn print_help() {
    println!("{}", help_text());
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

#[cfg(test)]
mod tests {
    use super::help_text;

    #[test]
    fn help_text_mentions_v1_1_and_format_status() {
        let help = help_text();
        assert!(help.contains("skadi-cli v1.1"));
        assert!(help.contains("Canonical CLI workflow for Skadi v1.1."));
        assert!(help.contains("format [--check] [path ...]  Format Skadi source files"));
        assert!(help.contains("tui                Full-screen interactive workflow"));
    }
}
