use std::fs;
use std::path::Path;

use crate::actions;

const HELP: &str = "skadi-cli debug

Build a project with Skadi probes and start an interactive debug session.

Usage:
  skadi-cli debug [--break <file.skd:line>]... [--cc compiler] [-- <args ...>]

Options:
  -b, --break <file.skd:line>  Stop at an executable statement; repeat as needed
  --cc <compiler>              Select the host C compiler
  -h, --help                   Show this help

With no breakpoints, execution pauses at the first statement.
Session commands: continue (c), step (s), quit (q).";

pub fn run(args: &[String]) -> Result<(), String> {
    if matches!(args, [arg] if arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let options = actions::parse_debug_options(args).map_err(|error| error.to_string())?;
    let prepared = actions::prepare_debug(&options).map_err(|error| error.to_string())?;
    println!(
        "debug build ok [{}]: {}",
        prepared.build.target,
        prepared.build.exe_path.display()
    );
    if prepared.starts_paused {
        println!("start: pause at the first executable statement");
    } else {
        for breakpoint in &prepared.breakpoints {
            println!(
                "breakpoint {}:{}:{} -> {} ({})",
                display_source(&prepared.build.project.cwd, &breakpoint.source_path),
                breakpoint.line,
                breakpoint.col,
                breakpoint.statement_id,
                breakpoint.statement_kind
            );
        }
    }
    println!("debug commands: continue (c), step (s), quit (q)");
    let result = actions::execute_debug(&prepared).map_err(|error| error.to_string())?;
    println!("debug session completed: {}", result.exit_status);
    Ok(())
}

fn display_source(project_root: &Path, source: &Path) -> String {
    let canonical_root = fs::canonicalize(project_root).unwrap_or_else(|_| project_root.into());
    let value = source
        .strip_prefix(&canonical_root)
        .unwrap_or(source)
        .to_string_lossy()
        .replace('\\', "/");
    value.strip_prefix("//?/").unwrap_or(&value).to_string()
}

#[cfg(test)]
mod tests {
    use crate::actions::parse_debug_options;

    #[test]
    fn parses_repeated_breakpoints_and_program_arguments() {
        let args = vec![
            "--break".to_string(),
            "src/main.skd:3".to_string(),
            "-b".to_string(),
            "src/worker.skd:8".to_string(),
            "--".to_string(),
            "hello".to_string(),
        ];
        let options = parse_debug_options(&args).expect("debug options should parse");
        assert_eq!(options.breakpoints.len(), 2);
        assert_eq!(options.breakpoints[0].line, 3);
        assert_eq!(options.program_args, vec!["hello"]);
    }

    #[test]
    fn parses_windows_breakpoint_path_from_the_final_colon() {
        let args = vec!["--break".to_string(), r"C:\work\main.skd:12".to_string()];
        let options = parse_debug_options(&args).expect("Windows path should parse");
        assert_eq!(options.breakpoints[0].line, 12);
        assert_eq!(
            options.breakpoints[0].path.to_string_lossy(),
            r"C:\work\main.skd"
        );
    }

    #[test]
    fn rejects_zero_breakpoint_line() {
        let args = vec!["--break".to_string(), "src/main.skd:0".to_string()];
        let error = parse_debug_options(&args).expect_err("zero line should fail");
        assert!(error.to_string().contains("greater than zero"));
    }
}
