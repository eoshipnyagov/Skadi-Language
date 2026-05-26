use std::fs;
use std::path::PathBuf;

use crate::project::load_project;
use crate::templates::{example_templates, normalize_project_type, PROJECT_TYPES};

pub fn run(args: &[String]) -> Result<(), String> {
    match parse_examples_args(args)? {
        ExamplesAction::List => {
            println!("available example sets: {}", PROJECT_TYPES.join(", "));
            Ok(())
        }
        ExamplesAction::Generate { force_type } => {
            let project = load_project()?;
            let project_type = force_type.unwrap_or(project.project_type);
            let examples_dir = project.root.join("examples");
            fs::create_dir_all(&examples_dir)
                .map_err(|e| format!("create {} failed: {e}", examples_dir.display()))?;

            let files = example_templates(&project_type);
            for (name, content) in files {
                let path: PathBuf = examples_dir.join(name);
                if path.exists() {
                    continue;
                }
                fs::write(&path, content).map_err(|e| format!("write {} failed: {e}", path.display()))?;
            }

            println!("examples added for type '{}': {}", project_type, examples_dir.display());
            Ok(())
        }
    }
}

#[derive(Debug)]
enum ExamplesAction {
    List,
    Generate { force_type: Option<String> },
}

fn parse_examples_args(args: &[String]) -> Result<ExamplesAction, String> {
    if args.len() == 1 && args[0] == "--list" {
        return Ok(ExamplesAction::List);
    }
    if args.is_empty() {
        return Ok(ExamplesAction::Generate { force_type: None });
    }
    if args.len() == 2 && args[0] == "--type" {
        let ty = normalize_project_type(&args[1])?;
        return Ok(ExamplesAction::Generate {
            force_type: Some(ty),
        });
    }
    Err("Usage: skadi examples [--list] [--type <game|embedded|console|gui>]".to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_examples_args, ExamplesAction};

    #[test]
    fn parse_examples_args_list() {
        let args = vec!["--list".to_string()];
        let a = parse_examples_args(&args).expect("parse");
        assert!(matches!(a, ExamplesAction::List));
    }

    #[test]
    fn parse_examples_args_default_generate() {
        let a = parse_examples_args(&[]).expect("parse");
        assert!(matches!(a, ExamplesAction::Generate { force_type: None }));
    }

    #[test]
    fn parse_examples_args_generate_type() {
        let args = vec!["--type".to_string(), "gui".to_string()];
        let a = parse_examples_args(&args).expect("parse");
        match a {
            ExamplesAction::Generate { force_type } => assert_eq!(force_type.as_deref(), Some("gui")),
            _ => panic!("expected generate"),
        }
    }

    #[test]
    fn parse_examples_args_rejects_invalid_usage() {
        let args = vec!["--foo".to_string()];
        let err = parse_examples_args(&args).expect_err("must reject");
        assert!(err.contains("Usage: skadi examples"));
    }

    #[test]
    fn parse_examples_args_rejects_unknown_type() {
        let args = vec!["--type".to_string(), "web".to_string()];
        let err = parse_examples_args(&args).expect_err("must reject");
        assert!(err.contains("unknown project type"));
    }
}
