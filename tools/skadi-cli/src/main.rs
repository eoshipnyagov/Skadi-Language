mod actions;
mod commands;
mod pipeline;
mod project;
mod targets;
mod tui;

use std::env;

fn help_text() -> String {
    [
        concat!("skadi-cli v", env!("CARGO_PKG_VERSION")),
        "Canonical CLI workflow for Skadi v1.2.",
        "Usage:",
        "  skadi-cli <command> [args]",
        "",
        "Commands:",
        "  new <name>         Create a new Skadi project",
        "  init               Initialize Skadi project in current directory",
        "  check              Run frontend checks",
        "  analyze [--json]   Explain lifecycle and blocking behavior",
        "  build [--target] [--cc]  Build project",
        "  run [--target] [--cc]    Build and run project",
        "  quick-run <file.skd> [-- <args>]  Run one file without a manifest",
        "  target list        List supported targets",
        "  tui                Full-screen interactive workflow",
        "  format [--check] [path ...]  Format Skadi source files",
        "  doctor             Verify toolchain environment",
        "",
        "Options:",
        "  -V, --version      Print release version",
        "  -h, --help         Show this help",
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
        "analyze" => commands::analyze_cmd::run(&args[2..]),
        "build" => commands::build_cmd::run(&args[2..]),
        "run" => commands::run_cmd::run(&args[2..]),
        "quick-run" => commands::quick_run_cmd::run(&args[2..]),
        "target" => commands::target_cmd::run(&args[2..]),
        "tui" => commands::tui_cmd::run(&args[2..]),
        "format" => commands::format_cmd::run(&args[2..]),
        "doctor" => commands::doctor_cmd::run(&args[2..]),
        "--version" | "-V" => {
            println!("skadi-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => Err(format!("unknown command: {cmd}. Use 'skadi-cli help'.")),
    };

    if let Err(err) = result {
        if cmd == "analyze" && args[2..].iter().any(|arg| arg == "--json") {
            println!("{err}");
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::help_text;

    #[test]
    fn help_text_mentions_release_identity_and_format_status() {
        let help = help_text();
        assert!(help.contains(concat!("skadi-cli v", env!("CARGO_PKG_VERSION"))));
        assert!(help.contains("Canonical CLI workflow for Skadi v1.2."));
        assert!(help.contains("skadi-cli <command> [args]"));
        assert!(help.contains("-V, --version"));
        assert!(help.contains("format [--check] [path ...]  Format Skadi source files"));
        assert!(help.contains("quick-run <file.skd> [-- <args>]"));
        assert!(help.contains("analyze [--json]"));
        assert!(help.contains("tui                Full-screen interactive workflow"));
    }
}
