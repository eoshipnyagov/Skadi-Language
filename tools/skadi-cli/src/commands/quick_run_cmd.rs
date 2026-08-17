use crate::actions;

const HELP: &str = "skadi-cli quick-run
Build and run one .skd file without Skadi.toml.

Usage:
  skadi-cli quick-run <file.skd> [--cc compiler] [-- <args ...>]

Options:
  --cc <compiler>  Select the host C compiler
  -h, --help       Show this help

Program arguments must follow '--'. Temporary build artifacts are removed after execution.";

pub fn run(args: &[String]) -> Result<(), String> {
    if matches!(args, [arg] if arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return Ok(());
    }

    let options = actions::parse_quick_run_options(args).map_err(|e| e.to_string())?;
    let prepared = actions::prepare_quick_run(&options).map_err(|e| e.to_string())?;
    eprintln!(
        "quick-run [{}] (cc={}): {}",
        prepared.target,
        prepared.selected_compiler,
        prepared.source.display()
    );
    let result = actions::execute_quick_run(&prepared).map_err(|e| e.to_string())?;
    eprintln!(
        "quick-run completed [{}] (cc={}, status={}): {}",
        result.target,
        result.selected_compiler,
        result.exit_status,
        result.source.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::actions::parse_quick_run_options;

    #[test]
    fn parses_source_compiler_and_program_arguments() {
        let args = vec![
            "hello.skd".to_string(),
            "--cc".to_string(),
            "clang".to_string(),
            "--".to_string(),
            "first".to_string(),
            "--second".to_string(),
        ];
        let options = parse_quick_run_options(&args).expect("quick-run args should parse");
        assert_eq!(options.source.to_string_lossy(), "hello.skd");
        assert_eq!(options.build.target, "host");
        assert_eq!(options.build.cc.as_deref(), Some("clang"));
        assert_eq!(options.program_args, ["first", "--second"]);
    }

    #[test]
    fn requires_separator_before_program_arguments() {
        let args = vec!["hello.skd".to_string(), "extra".to_string()];
        let error = parse_quick_run_options(&args).expect_err("extra path should fail");
        assert!(error.to_string().contains("after '--'"));
    }
}
