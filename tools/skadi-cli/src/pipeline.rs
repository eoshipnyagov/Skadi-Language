use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};
use crate::targets::{candidate_invocations, resolve_profile};

pub fn compile_to_c(entry_path: &Path) -> Result<String, String> {
    let source = load_source_with_imports(entry_path)?;
    let tokens = lex(&source).map_err(|e| format!("lex failed: {e}"))?;
    let program = parse_program(&tokens).map_err(|e| format!("parse failed: {e}"))?;
    semantic_analyze(&program).map_err(|e| format!("semantic failed: {e}"))?;
    for warning in semantic_style_warnings(&program) {
        eprintln!("{warning}");
    }
    Ok(transpile_program_to_c(&program))
}

fn load_source_with_imports(entry_path: &Path) -> Result<String, String> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    load_source_recursive(entry_path, &mut seen, &mut stack)
}

fn load_source_recursive(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
) -> Result<String, String> {
    let abs = fs::canonicalize(path)
        .map_err(|e| format!("failed to resolve {}: {e}", path.display()))?;

    if stack.iter().any(|p| p == &abs) {
        let mut chain = stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        chain.push(abs.display().to_string());
        return Err(format!("cyclic import detected: {}", chain.join(" -> ")));
    }

    if seen.contains(&abs) {
        return Ok(String::new());
    }

    stack.push(abs.clone());
    let source = fs::read_to_string(&abs)
        .map_err(|e| format!("failed to read {}: {e}", abs.display()))?;
    let base_dir = abs.parent().unwrap_or(Path::new("."));
    let mut merged = String::new();

    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line)? {
            let import_abs = base_dir.join(import_path);
            let imported = load_source_recursive(&import_abs, seen, stack)?;
            if !imported.is_empty() {
                merged.push_str(&imported);
                if !imported.ends_with('\n') {
                    merged.push('\n');
                }
            }
            continue;
        }
        merged.push_str(line);
        merged.push('\n');
    }

    stack.pop();
    seen.insert(abs);
    Ok(merged)
}

fn parse_import_line(line: &str) -> Result<Option<String>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return Ok(None);
    }
    if !trimmed.starts_with("import ") {
        return Ok(None);
    }
    let rest = trimmed["import ".len()..].trim();
    if !(rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2) {
        return Err(format!(
            "unsupported import syntax '{}'; expected: import \"./path/file.skd\"",
            line.trim()
        ));
    }
    Ok(Some(rest[1..rest.len() - 1].to_string()))
}

pub fn compile_c_to_exe(c_path: &Path, exe_path: &Path, target: &str) -> Result<(), String> {
    let _profile = resolve_profile(target)?;

    let mut last_err = String::new();
    let candidates = candidate_invocations(target, c_path, exe_path)?;
    for inv in candidates {
        let out = Command::new(&inv.program).args(&inv.args).output();
        match out {
            Ok(r) if r.status.success() => return Ok(()),
            Ok(r) => {
                last_err = format!(
                    "{} failed: {}",
                    inv.program,
                    String::from_utf8_lossy(&r.stderr).trim()
                );
            }
            Err(e) => {
                last_err = format!("failed to run {}: {}", inv.program, e);
            }
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::{load_source_with_imports, parse_import_line};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_case_dir(stem: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("skadi_cli_{stem}_{stamp}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    #[test]
    fn parse_import_line_accepts_quoted_path() {
        let got = parse_import_line(r#"import "./lib.skd""#).expect("parse ok");
        assert_eq!(got.as_deref(), Some("./lib.skd"));
    }

    #[test]
    fn parse_import_line_rejects_unquoted_path() {
        let err = parse_import_line("import lib").expect_err("must reject");
        assert!(err.contains("unsupported import syntax"));
    }

    #[test]
    fn load_source_with_imports_merges_files() {
        let root = temp_case_dir("imports_ok");
        let entry = root.join("main.skd");
        let util = root.join("util.skd");
        fs::write(&util, "fn helper() Int {\n    return 7\n}\n").expect("write util");
        fs::write(
            &entry,
            "import \"./util.skd\"\nnew Int x = helper()\n",
        )
        .expect("write entry");

        let merged = load_source_with_imports(&entry).expect("merge");
        assert!(merged.contains("fn helper() Int"));
        assert!(merged.contains("new Int x = helper()"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_source_with_imports_detects_cycle() {
        let root = temp_case_dir("imports_cycle");
        let a = root.join("a.skd");
        let b = root.join("b.skd");
        fs::write(&a, "import \"./b.skd\"\nnew Int x = 1\n").expect("write a");
        fs::write(&b, "import \"./a.skd\"\nnew Int y = 2\n").expect("write b");

        let err = load_source_with_imports(&a).expect_err("cycle expected");
        assert!(err.contains("cyclic import detected"));

        let _ = fs::remove_dir_all(root);
    }
}
