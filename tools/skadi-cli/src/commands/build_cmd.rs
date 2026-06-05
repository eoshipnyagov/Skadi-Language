use crate::actions;

pub fn run(args: &[String]) -> Result<(), String> {
    let options = actions::parse_build_options(args).map_err(|e| e.to_string())?;
    let result = actions::run_build(&options).map_err(|e| e.to_string())?;
    if let Some(selected) = result.requested_compiler {
        println!(
            "build ok [{}] (cc={}): {}",
            result.target,
            selected,
            result.exe_path.display()
        );
    } else {
        println!(
            "build ok [{}]: {}",
            result.target,
            result.exe_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::actions::parse_build_options;

    #[test]
    fn parse_target_and_cc() {
        let args = vec![
            "--target".to_string(),
            "host".to_string(),
            "--cc".to_string(),
            "gcc".to_string(),
        ];
        let options = parse_build_options(&args).expect("args should parse");
        assert_eq!(options.target, "host");
        assert_eq!(options.cc.as_deref(), Some("gcc"));
    }

    #[test]
    fn parse_missing_cc_value() {
        let args = vec!["--cc".to_string()];
        let err = parse_build_options(&args).expect_err("should fail");
        assert!(err.to_string().contains("--cc requires value"));
    }

    #[test]
    fn parse_unknown_option() {
        let args = vec!["--zzz".to_string()];
        let err = parse_build_options(&args).expect_err("should fail");
        assert!(err.to_string().contains("unknown build option"));
    }
}
