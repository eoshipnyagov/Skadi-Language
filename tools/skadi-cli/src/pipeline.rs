use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};
use crate::targets::{candidate_invocations, resolve_profile};

const IMPORT_CONTRACT_HINT: &str =
    "v1 import contract: only `import \"./relative_path.skd\"` is supported (no module-name import, no alias).";

pub fn compile_to_c(entry_path: &Path) -> Result<String, String> {
    let source = load_source_with_imports(entry_path)
        .map_err(|e| format!("[SC-MOD-001] stage=module-import: {e}\nhint: use only path imports like import \"./file.skd\" and verify import graph paths/cycles."))?;
    let tokens = lex(&source).map_err(|e| {
        format!(
            "[SC-LEX-000] stage=lex: {e}\nhint: inspect the reported source position and remove unsupported characters/tokens."
        )
    })?;
    let program = parse_program(&tokens).map_err(|e| {
        format!(
            "[SC-PARSE-000] stage=parse: {e}\nhint: check statement/block structure near the reported token."
        )
    })?;
    semantic_analyze(&program).map_err(|e| {
        format!(
            "[SC-SEM-000] stage=semantic: {e}\nhint: align types/signatures and ensure on error is used only in allowed contexts."
        )
    })?;
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
    let abs = fs::canonicalize(path).map_err(|e| {
        format!(
            "import path resolution failed for '{}': {e}. {}",
            path.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;

    if stack.iter().any(|p| p == &abs) {
        let mut chain = stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        chain.push(abs.display().to_string());
        return Err(format!(
            "cyclic import detected: {}. {}",
            chain.join(" -> "),
            IMPORT_CONTRACT_HINT
        ));
    }

    if seen.contains(&abs) {
        return Ok(String::new());
    }

    stack.push(abs.clone());
    let source = fs::read_to_string(&abs).map_err(|e| {
        format!(
            "failed to read import file '{}': {e}. {}",
            abs.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;
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
    if rest.starts_with('"') && rest.contains("\" as ") {
        return Err(format!(
            "import alias is not supported in v1: '{}'. {}",
            line.trim(),
            IMPORT_CONTRACT_HINT
        ));
    }
    if !rest.starts_with('"') {
        return Err(format!(
            "module-name import is not supported in v1: '{}'. {}",
            line.trim(),
            IMPORT_CONTRACT_HINT
        ));
    }
    if !(rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2) {
        return Err(format!(
            "unsupported import syntax '{}'; expected: import \"./path/file.skd\". {}",
            line.trim(),
            IMPORT_CONTRACT_HINT
        ));
    }
    Ok(Some(rest[1..rest.len() - 1].to_string()))
}

pub fn compile_c_to_exe(c_path: &Path, exe_path: &Path, target: &str) -> Result<(), String> {
    let _profile = resolve_profile(target)?;

    let candidates = candidate_invocations(target, c_path, exe_path)?;
    let mut attempts: Vec<String> = Vec::new();
    for inv in candidates {
        let out = Command::new(&inv.program).args(&inv.args).output();
        match out {
            Ok(r) if r.status.success() => return Ok(()),
            Ok(r) => {
                let stderr = String::from_utf8_lossy(&r.stderr).trim().to_string();
                let stdout = String::from_utf8_lossy(&r.stdout).trim().to_string();
                let detail = if !stderr.is_empty() {
                    stderr
                } else if !stdout.is_empty() {
                    stdout
                } else {
                    "no compiler output".to_string()
                };
                attempts.push(format!("- {}: {}", inv.program, detail));
            }
            Err(e) => {
                attempts.push(format!("- {}: failed to run: {}", inv.program, e));
            }
        }
    }
    Err(format!(
        "[SC-CGEN-001] stage=codegen-native-compile: native C compilation failed for target '{}'. attempts:\n{}\nhint: install a supported C compiler or pass --cc <compiler>; run `skadi doctor` for toolchain diagnostics.",
        target,
        attempts.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::{compile_c_to_exe, compile_to_c, load_source_with_imports, parse_import_line};
    use crate::targets::detect_compiler;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
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

    fn has_host_compiler() -> bool {
        ["gcc", "clang", "cc", "cl"].iter().any(|c| detect_compiler(c))
    }

    #[test]
    fn parse_import_line_accepts_quoted_path() {
        let got = parse_import_line(r#"import "./lib.skd""#).expect("parse ok");
        assert_eq!(got.as_deref(), Some("./lib.skd"));
    }

    #[test]
    fn parse_import_line_rejects_unquoted_path() {
        let err = parse_import_line("import lib").expect_err("must reject");
        assert!(err.contains("module-name import is not supported"));
    }

    #[test]
    fn parse_import_line_rejects_alias_form() {
        let err = parse_import_line(r#"import "./lib.skd" as lib"#).expect_err("must reject");
        assert!(err.contains("import alias is not supported"));
    }

    #[test]
    fn parse_import_line_rejects_unterminated_quote() {
        let err = parse_import_line("import \"./lib.skd").expect_err("must reject");
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

    #[test]
    fn load_source_with_imports_detects_self_cycle() {
        let root = temp_case_dir("imports_self_cycle");
        let a = root.join("a.skd");
        fs::write(&a, "import \"./a.skd\"\nnew Int x = 1\n").expect("write a");

        let err = load_source_with_imports(&a).expect_err("self-cycle expected");
        assert!(err.contains("cyclic import detected"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_source_with_imports_fails_fast_on_missing_file() {
        let root = temp_case_dir("imports_missing");
        let entry = root.join("main.skd");
        fs::write(&entry, "import \"./missing.skd\"\nnew Int x = 1\n").expect("write entry");

        let err = load_source_with_imports(&entry).expect_err("missing import should fail");
        assert!(err.contains("import path resolution failed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_source_with_imports_deduplicates_diamond_imports() {
        let root = temp_case_dir("imports_diamond");
        let main = root.join("main.skd");
        let left = root.join("left.skd");
        let right = root.join("right.skd");
        let common = root.join("common.skd");
        fs::write(&common, "fn shared() Int {\n    return 1\n}\n").expect("write common");
        fs::write(&left, "import \"./common.skd\"\nnew Int l = shared()\n").expect("write left");
        fs::write(&right, "import \"./common.skd\"\nnew Int r = shared()\n").expect("write right");
        fs::write(
            &main,
            "import \"./left.skd\"\nimport \"./right.skd\"\nnew Int x = l + r\n",
        )
        .expect("write main");

        let merged = load_source_with_imports(&main).expect("merge");
        assert_eq!(merged.matches("fn shared() Int").count(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_source_with_imports_deduplicates_repeated_direct_imports() {
        let root = temp_case_dir("imports_repeat_direct");
        let main = root.join("main.skd");
        let common = root.join("common.skd");
        fs::write(&common, "fn shared() Int {\n    return 42\n}\n").expect("write common");
        fs::write(
            &main,
            "import \"./common.skd\"\nimport \"./common.skd\"\nnew Int x = shared()\n",
        )
        .expect("write main");

        let merged = load_source_with_imports(&main).expect("merge");
        assert_eq!(merged.matches("fn shared() Int").count(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_source_with_imports_preserves_deterministic_import_order() {
        let root = temp_case_dir("imports_order");
        let main = root.join("main.skd");
        let one = root.join("one.skd");
        let two = root.join("two.skd");
        fs::write(&one, "new Int marker_one = 1\n").expect("write one");
        fs::write(&two, "new Int marker_two = 2\n").expect("write two");
        fs::write(
            &main,
            "import \"./one.skd\"\nimport \"./two.skd\"\nnew Int m = marker_one + marker_two\n",
        )
        .expect("write main");

        let merged = load_source_with_imports(&main).expect("merge");
        let p1 = merged.find("marker_one").expect("marker one present");
        let p2 = merged.find("marker_two").expect("marker two present");
        assert!(p1 < p2, "import order must be stable and line-ordered");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_chain_imports_compile_and_run() {
        if !has_host_compiler() {
            eprintln!("Skipping e2e_chain_imports_compile_and_run: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("imports_e2e_chain");
        let entry = root.join("main.skd");
        let math = root.join("math.skd");
        let util = root.join("util.skd");
        fs::write(&util, "fn twice(Int x) Int {\n    return x + x\n}\n").expect("write util");
        fs::write(&math, "import \"./util.skd\"\nfn plus_two(Int x) Int {\n    return x + 2\n}\n").expect("write math");
        fs::write(
            &entry,
            "import \"./math.skd\"\nnew Int a = twice(2)\nnew Int b = plus_two(a)\noutput(b)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_diamond_imports_structs_compile_and_run() {
        if !has_host_compiler() {
            eprintln!("Skipping e2e_diamond_imports_structs_compile_and_run: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("imports_e2e_diamond");
        let entry = root.join("main.skd");
        let left = root.join("left.skd");
        let right = root.join("right.skd");
        let shared = root.join("shared.skd");
        fs::write(&shared, "struct Acc {\n    Int value\n}\n\nfn base() Int {\n    return 3\n}\n").expect("write shared");
        fs::write(&left, "import \"./shared.skd\"\nfn l() Int {\n    return base() + 1\n}\n").expect("write left");
        fs::write(&right, "import \"./shared.skd\"\nfn r() Int {\n    return base() + 2\n}\n").expect("write right");
        fs::write(
            &entry,
            "import \"./left.skd\"\nimport \"./right.skd\"\nnew Acc a = {value = l() + r()}\noutput(a.value)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_multifile_feature_mix_compile_and_run() {
        if !has_host_compiler() {
            eprintln!("Skipping e2e_multifile_feature_mix_compile_and_run: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("imports_e2e_feature_mix");
        let entry = root.join("main.skd");
        let ops = root.join("ops.skd");
        let util = root.join("util.skd");

        fs::write(
            &util,
            "fn next(Int x) Int {\n    return x + 1\n}\n",
        )
        .expect("write util");
        fs::write(
            &ops,
            "import \"./util.skd\"\nfn score3(Int a, Int b, Int c) Int {\n    return next(a) + next(b) + next(c)\n}\n",
        )
        .expect("write ops");
        fs::write(
            &entry,
            "import \"./ops.skd\"\nnew i32 List xs = [1, 2, 3]\nnew Int s = score3(xs[0], xs[1], xs[2])\nwhen s {\n    is 9 {\n        output(s)\n    }\n    else {\n        output(0)\n    }\n}\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_deep_chain_imports_compile_and_run() {
        if !has_host_compiler() {
            eprintln!("Skipping e2e_deep_chain_imports_compile_and_run: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("imports_e2e_deep_chain");
        let entry = root.join("main.skd");

        fs::write(
            root.join("m11.skd"),
            "fn step11(Int x) Int {\n    return x + 11\n}\n",
        )
        .expect("write m11");
        for i in (0..=10).rev() {
            let next = i + 1;
            let content = format!(
                "import \"./m{next}.skd\"\nfn step{i}(Int x) Int {{\n    return step{next}(x + {i})\n}}\n"
            );
            fs::write(root.join(format!("m{i}.skd")), content).expect("write chain module");
        }
        fs::write(
            &entry,
            "import \"./m0.skd\"\nnew Int out = step0(1)\noutput(out)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_wide_diamond_imports_compile_and_run() {
        if !has_host_compiler() {
            eprintln!("Skipping e2e_wide_diamond_imports_compile_and_run: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("imports_e2e_wide_diamond");
        let entry = root.join("main.skd");
        fs::write(
            root.join("shared.skd"),
            "fn base(Int x) Int {\n    return x + 1\n}\n",
        )
        .expect("write shared");

        let mut imports = String::new();
        let mut sum_expr = String::new();
        for i in 0..8 {
            let name = format!("leaf{i}");
            fs::write(
                root.join(format!("{name}.skd")),
                format!("import \"./shared.skd\"\nfn {name}() Int {{\n    return base({i})\n}}\n"),
            )
            .expect("write leaf");
            imports.push_str(&format!("import \"./{name}.skd\"\n"));
            if i > 0 {
                sum_expr.push_str(" + ");
            }
            sum_expr.push_str(&format!("{name}()"));
        }
        fs::write(
            &entry,
            format!("{imports}new Int total = {sum_expr}\noutput(total)\n"),
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_module_name_import_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_module_name");
        let entry = root.join("main.skd");
        fs::write(&entry, "import lib\nnew Int x = 1\n").expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("stage=module-import"));
        assert!(err.contains("hint:"));
        assert!(err.contains("module-name import is not supported"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_alias_import_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_alias");
        let entry = root.join("main.skd");
        fs::write(&entry, "import \"./lib.skd\" as lib\nnew Int x = 1\n").expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("import alias is not supported"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_missing_import_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_missing");
        let entry = root.join("main.skd");
        fs::write(&entry, "import \"./missing.skd\"\nnew Int x = 1\n").expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("import path resolution failed"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_cycle_import_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_cycle");
        let a = root.join("a.skd");
        let b = root.join("b.skd");
        fs::write(&a, "import \"./b.skd\"\nnew Int x = 1\n").expect("write a");
        fs::write(&b, "import \"./a.skd\"\nnew Int y = 2\n").expect("write b");

        let err = compile_to_c(&a).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("cyclic import detected"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_self_cycle_import_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_self_cycle");
        let a = root.join("a.skd");
        fs::write(&a, "import \"./a.skd\"\nnew Int x = 1\n").expect("write a");

        let err = compile_to_c(&a).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("cyclic import detected"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_invalid_import_syntax_fails_with_contract_diagnostic() {
        let root = temp_case_dir("imports_neg_invalid_syntax");
        let entry = root.join("main.skd");
        fs::write(&entry, "import \"./lib.skd\nnew Int x = 1\n").expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("unsupported import syntax"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_parser_error_surfaces_with_parse_code() {
        let root = temp_case_dir("mut_parse_code");
        let entry = root.join("main.skd");
        fs::write(
            &entry,
            "new Int x = 1\nx = 1 +\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-PARSE-000]"));
        assert!(err.contains("stage=parse"));
        assert!(err.contains("hint:"));
        assert!(err.contains("SC-PARSE-"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_semantic_on_error_non_danger_surfaces_with_sem_code() {
        let root = temp_case_dir("mut_sem_on_error");
        let entry = root.join("main.skd");
        fs::write(
            &entry,
            "new Int x = 1\nx = read(\"a.txt\") on error {\n    x = 0\n}\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-SEM-000]"));
        assert!(err.contains("stage=semantic"));
        assert!(err.contains("hint:"));
        assert!(err.contains("SC-SEM-040"));
        assert!(err.contains("on error requires danger fn call"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_semantic_index_type_surfaces_with_sem_code() {
        let root = temp_case_dir("mut_sem_index_type");
        let entry = root.join("main.skd");
        fs::write(
            &entry,
            "new i32 List xs = [1, 2]\nnew i32 v = xs[\"bad\"]\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-SEM-000]"));
        assert!(err.contains("stage=semantic"));
        assert!(err.contains("hint:"));
        assert!(err.contains("SC-SEM-020"));
        assert!(err.contains("index access requires Int index"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn compile_c_to_exe_reports_attempt_matrix_on_failure() {
        if !has_host_compiler() {
            eprintln!("Skipping compile_c_to_exe_reports_attempt_matrix_on_failure: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("cgen_attempt_matrix");
        let c_path = root.join("broken.c");
        let exe_path = if cfg!(windows) { root.join("broken.exe") } else { root.join("broken") };
        fs::write(&c_path, "int main(void) { BROKEN_TOKEN return 0; }\n").expect("write c");

        let err = compile_c_to_exe(&c_path, &exe_path, "host").expect_err("compile must fail");
        assert!(err.contains("[SC-CGEN-001]"));
        assert!(err.contains("stage=codegen-native-compile"));
        assert!(err.contains("hint:"));
        assert!(err.contains("attempts:"));
        assert!(
            err.contains("- gcc:") || err.contains("- clang:") || err.contains("- cc:"),
            "must include compiler attempt lines, got: {}",
            err
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_codegen_gap_surfaces_as_sc_cgen_001() {
        if !has_host_compiler() {
            eprintln!("Skipping mutation_negative_codegen_gap_surfaces_as_sc_cgen_001: no host C compiler in PATH.");
            return;
        }

        let root = temp_case_dir("mut_codegen_gap");
        let entry = root.join("main.skd");
        fs::write(
            &entry,
            "new i32 List xs = [1, 2]\nnew Int v = 0\nv = xs.pop() on error {\n    v = -1\n}\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile_to_c should pass this known gap");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) { root.join("out.exe") } else { root.join("out") };
        fs::write(&c_path, c).expect("write C file");

        let err = compile_c_to_exe(&c_path, &exe_path, "host").expect_err("native compile must fail");
        assert!(err.contains("[SC-CGEN-001]"));
        assert!(err.contains("stage=codegen-native-compile"));
        assert!(err.contains("hint:"));
        assert!(err.contains("attempts:"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_lex_error_surfaces_with_lex_code() {
        let root = temp_case_dir("mut_lex_code");
        let entry = root.join("main.skd");
        fs::write(&entry, "new Int x = 1\n@\n").expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-LEX-000]"));
        assert!(err.contains("stage=lex"));
        assert!(err.contains("hint:"));

        let _ = fs::remove_dir_all(root);
    }
}
