use std::fs;
use std::path::Path;
use std::process::Command;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};
use crate::targets::{candidate_invocations, resolve_profile, single_compiler_invocation};

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

pub fn compile_c_to_exe(
    c_path: &Path,
    exe_path: &Path,
    target: &str,
    preferred_compiler: Option<&str>,
) -> Result<(), String> {
    let _profile = resolve_profile(target)?;

    let candidates = if let Some(cc) = preferred_compiler {
        vec![single_compiler_invocation(target, cc, c_path, exe_path)?]
    } else {
        candidate_invocations(target, c_path, exe_path)?
    };

    let mut errs: Vec<String> = Vec::new();
    for inv in candidates {
        let out = Command::new(&inv.program).args(&inv.args).output();
        match out {
            Ok(r) if r.status.success() => return Ok(()),
            Ok(r) => {
                errs.push(format!(
                    "{} failed (status {}): {}",
                    inv.program,
                    r.status,
                    String::from_utf8_lossy(&r.stderr).trim()
                ));
            }
            Err(e) => {
                errs.push(format!("failed to run {}: {}", inv.program, e));
            }
        }
    }
    if errs.is_empty() {
        Err("no compiler candidates were generated".to_string())
    } else {
        Err(format!(
            "no working C compiler for target '{}': {}",
            target,
            errs.join(" | ")
        ))
    }
}
