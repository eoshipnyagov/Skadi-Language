use std::fs;
use std::path::Path;
use std::process::Command;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

pub fn compile_to_c(entry_path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("failed to read {}: {e}", entry_path.display()))?;
    let tokens = lex(&source).map_err(|e| format!("lex failed: {e}"))?;
    let program = parse_program(&tokens).map_err(|e| format!("parse failed: {e}"))?;
    semantic_analyze(&program).map_err(|e| format!("semantic failed: {e}"))?;
    for warning in semantic_style_warnings(&program) {
        eprintln!("{warning}");
    }
    Ok(transpile_program_to_c(&program))
}

pub fn compile_c_to_exe(c_path: &Path, exe_path: &Path, target: &str) -> Result<(), String> {
    if target != "host" {
        return Err(format!("target '{target}' is not implemented yet (only host)."));
    }

    let mut last_err = String::new();

    let mut candidates: Vec<(&str, Vec<String>)> = vec![
        (
            "gcc",
            vec![
                c_path.display().to_string(),
                "-o".to_string(),
                exe_path.display().to_string(),
            ],
        ),
        (
            "clang",
            vec![
                c_path.display().to_string(),
                "-o".to_string(),
                exe_path.display().to_string(),
            ],
        ),
        (
            "cc",
            vec![
                c_path.display().to_string(),
                "-o".to_string(),
                exe_path.display().to_string(),
            ],
        ),
    ];

    if cfg!(windows) {
        candidates.push((
            "cl",
            vec![
                "/nologo".to_string(),
                c_path.display().to_string(),
                format!("/Fe:{}", exe_path.display()),
            ],
        ));
    }

    for (compiler, args) in candidates {
        let out = Command::new(compiler).args(&args).output();
        match out {
            Ok(r) if r.status.success() => return Ok(()),
            Ok(r) => {
                last_err = format!(
                    "{} failed: {}",
                    compiler,
                    String::from_utf8_lossy(&r.stderr).trim()
                );
            }
            Err(e) => {
                last_err = format!("failed to run {}: {}", compiler, e);
            }
        }
    }
    Err(last_err)
}
