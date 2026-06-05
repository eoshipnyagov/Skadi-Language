use crate::actions::{self, FormatState};

pub fn run(args: &[String]) -> Result<(), String> {
    let options = actions::parse_format_options(args).map_err(|e| e.to_string())?;
    let result = actions::run_format(&options).map_err(|e| e.to_string())?;
    let total = result.files.len();
    let mut changed = 0usize;

    for file in result.files {
        if result.check_only {
            match file.state {
                FormatState::Updated => {
                    changed += 1;
                    println!("needs format {}", file.path.display());
                }
                FormatState::Unchanged => {
                    println!("ok {}", file.path.display());
                }
            }
        } else {
            match file.state {
                FormatState::Updated => {
                    changed += 1;
                    println!("formatted {}", file.path.display());
                }
                FormatState::Unchanged => {
                    println!("already formatted {}", file.path.display());
                }
            }
        }
    }

    if result.check_only {
        if changed > 0 {
            return Err(format!(
                "format check failed: {} file(s) need formatting.",
                changed
            ));
        }
        println!("format check ok: {} file(s)", total);
        return Ok(());
    }

    println!("format ok: {} file(s), {} changed", total, changed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::actions::parse_format_options;

    #[test]
    fn parse_options_supports_check_and_paths() {
        let args = vec![
            "--check".to_string(),
            "src/main.skd".to_string(),
            "examples/demo.skd".to_string(),
        ];
        let parsed = parse_format_options(&args).expect("options should parse");
        assert!(parsed.check_only);
        assert_eq!(parsed.paths, vec!["src/main.skd", "examples/demo.skd"]);
    }

    #[test]
    fn parse_options_rejects_unknown_flag() {
        let err =
            parse_format_options(&["--wat".to_string()]).expect_err("unknown flag should fail");
        assert!(err.to_string().contains("unknown format option"));
    }
}
