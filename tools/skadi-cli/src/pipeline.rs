use std::fs;
use std::path::Path;
use std::process::Command;

use crate::targets::{
    CompilerInvocation, candidate_invocations, resolve_profile, single_compiler_invocation,
};
use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

pub struct FrontendOutput {
    pub c_code: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ToolchainOutput {
    pub invocation: CompilerInvocation,
    pub status: String,
    pub stdout: String,
    pub stderr: String,
}

pub fn compile_frontend(entry_path: &Path) -> Result<FrontendOutput, String> {
    let source = fs::read_to_string(entry_path)
        .map_err(|e| format!("failed to read {}: {e}", entry_path.display()))?;
    let tokens = lex(&source).map_err(|e| format!("lex failed: {e}"))?;
    let program = parse_program(&tokens).map_err(|e| format!("parse failed: {e}"))?;
    semantic_analyze(&program).map_err(|e| format!("semantic failed: {e}"))?;
    ensure_codegen_supported(&program).map_err(|e| format!("codegen failed: {e}"))?;
    let warnings = semantic_style_warnings(&program);
    Ok(FrontendOutput {
        c_code: transpile_program_to_c(&program),
        warnings,
    })
}

pub fn compile_c_to_exe_detailed(
    c_path: &Path,
    exe_path: &Path,
    target: &str,
    preferred_compiler: Option<&str>,
) -> Result<ToolchainOutput, String> {
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
            Ok(r) if r.status.success() => {
                return Ok(ToolchainOutput {
                    invocation: inv,
                    status: r.status.to_string(),
                    stdout: String::from_utf8_lossy(&r.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&r.stderr).to_string(),
                });
            }
            Ok(r) => {
                let stdout = String::from_utf8_lossy(&r.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&r.stderr).trim().to_string();
                errs.push(format!(
                    "{} failed (status {}): {}{}{}",
                    format_invocation(&inv),
                    r.status,
                    stderr,
                    if !stdout.is_empty() && !stderr.is_empty() {
                        " | stdout: "
                    } else if !stdout.is_empty() {
                        "stdout: "
                    } else {
                        ""
                    },
                    stdout
                ));
            }
            Err(e) => {
                errs.push(format!("failed to run {}: {}", format_invocation(&inv), e));
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

fn format_invocation(invocation: &CompilerInvocation) -> String {
    if invocation.args.is_empty() {
        invocation.program.clone()
    } else {
        format!("{} {}", invocation.program, invocation.args.join(" "))
    }
}
