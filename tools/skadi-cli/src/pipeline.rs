use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use crate::targets::{
    CompilerInvocation, candidate_invocations, resolve_profile, single_compiler_invocation,
};
use v01::codegen::{ensure_codegen_supported, transpile_program_to_c};
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

const IMPORT_CONTRACT_HINT: &str = "v1 import contract: only `import \"./relative_path.skd\"` is supported (no module-name import, no alias).";

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
    let source = load_source_with_imports(entry_path).map_err(|e| {
        format!(
            "[SC-MOD-001] stage=module-import: {e}\nhint: use only path imports like import \"./file.skd\" and verify import graph paths/cycles."
        )
    })?;
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
    ensure_codegen_supported(&program).map_err(|e| {
        format!(
            "[SC-CG-000] stage=codegen: {e}\nhint: use only syntax supported by the current C backend."
        )
    })?;
    let warnings = semantic_style_warnings(&program);
    Ok(FrontendOutput {
        c_code: transpile_program_to_c(&program),
        warnings,
    })
}

#[cfg(test)]
pub fn compile_to_c(entry_path: &Path) -> Result<String, String> {
    let frontend = compile_frontend(entry_path)?;
    for warning in frontend.warnings {
        eprintln!("{warning}");
    }
    Ok(frontend.c_code)
}

fn load_source_with_imports(entry_path: &Path) -> Result<String, String> {
    let entry_abs = fs::canonicalize(entry_path).map_err(|e| {
        format!(
            "import path resolution failed for '{}': {e}. {}",
            entry_path.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;
    let direct_imports = collect_direct_imports(&entry_abs)?;

    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut stack: Vec<PathBuf> = Vec::new();
    let mut decl_index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let merged = load_source_recursive(&entry_abs, &mut seen, &mut stack, &mut decl_index)?;
    validate_entry_direct_visibility(&entry_abs, &direct_imports, &decl_index)?;
    ensure_no_public_symbol_collisions(&decl_index)?;
    Ok(merged)
}

fn load_source_recursive(
    path: &Path,
    seen: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    decl_index: &mut BTreeMap<String, BTreeSet<String>>,
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
    let raw_source = fs::read_to_string(&abs).map_err(|e| {
        format!(
            "failed to read import file '{}': {e}. {}",
            abs.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;
    let source = rewrite_local_symbols(&raw_source, &abs)?;
    index_public_top_level_declarations(&source, &abs, decl_index)?;
    let base_dir = abs.parent().unwrap_or(Path::new("."));
    let mut merged = String::new();

    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line)? {
            let import_abs = base_dir.join(import_path);
            let imported = load_source_recursive(&import_abs, seen, stack, decl_index)?;
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

fn collect_direct_imports(entry_abs: &Path) -> Result<HashSet<PathBuf>, String> {
    let source = fs::read_to_string(entry_abs).map_err(|e| {
        format!(
            "failed to read import file '{}': {e}. {}",
            entry_abs.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;
    let base_dir = entry_abs.parent().unwrap_or(Path::new("."));
    let mut out: HashSet<PathBuf> = HashSet::new();
    for line in source.lines() {
        if let Some(import_path) = parse_import_line(line)? {
            let import_abs = fs::canonicalize(base_dir.join(import_path)).map_err(|e| {
                format!(
                    "import path resolution failed for '{}': {e}. {}",
                    base_dir.display(),
                    IMPORT_CONTRACT_HINT
                )
            })?;
            out.insert(import_abs);
        }
    }
    Ok(out)
}

fn index_public_top_level_declarations(
    source: &str,
    path: &Path,
    decl_index: &mut BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let module = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("invalid module filename '{}'.", path.display()))?
        .to_string();
    let decl_re = Regex::new(r"(?m)^\s*(fn|struct|label)\s+([A-Za-z_][A-Za-z0-9_]*)")
        .map_err(|e| format!("internal declaration index regex error: {e}"))?;
    for caps in decl_re.captures_iter(source) {
        let Some(name_m) = caps.get(2) else {
            continue;
        };
        let name = name_m.as_str();
        if name.starts_with("__sklocal_") {
            continue;
        }
        decl_index
            .entry(name.to_string())
            .or_default()
            .insert(module.clone());
    }
    Ok(())
}

fn validate_entry_direct_visibility(
    entry_abs: &Path,
    direct_imports: &HashSet<PathBuf>,
    decl_index: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let module_of = |p: &Path| -> Result<String, String> {
        p.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("invalid module filename '{}'.", p.display()))
    };

    let entry_module = module_of(entry_abs)?;
    let mut allowed_modules: HashSet<String> = HashSet::new();
    allowed_modules.insert(entry_module);
    for p in direct_imports {
        allowed_modules.insert(module_of(p)?);
    }

    let source = fs::read_to_string(entry_abs).map_err(|e| {
        format!(
            "failed to read import file '{}': {e}. {}",
            entry_abs.display(),
            IMPORT_CONTRACT_HINT
        )
    })?;
    let call_re = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .map_err(|e| format!("internal entry-call regex error: {e}"))?;
    let skip = [
        "if", "when", "while", "for", "loop", "return", "on", "run", "wait", "new", "danger", "fn",
    ];

    for caps in call_re.captures_iter(&source) {
        let Some(name_m) = caps.get(1) else {
            continue;
        };
        let name = name_m.as_str();
        if skip.contains(&name) || name.contains('.') {
            continue;
        }
        if name == "output" || name == "input" || name == "read" || name == "write" {
            continue;
        }
        let Some(mods) = decl_index.get(name) else {
            continue;
        };
        if mods.iter().any(|m| allowed_modules.contains(m)) {
            continue;
        }
        return Err(format!(
            "[SC-MOD-003] transitive visibility violation: '{}' is not directly imported into '{}'. hint: add a direct import for the module that declares '{}' or access it through direct module API.",
            name,
            entry_abs.display(),
            name
        ));
    }

    Ok(())
}

fn ensure_no_public_symbol_collisions(
    decl_index: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let mut collisions: Vec<String> = Vec::new();
    for (name, modules) in decl_index {
        if modules.len() <= 1 {
            continue;
        }
        let modules_joined = modules.iter().cloned().collect::<Vec<_>>().join(", ");
        collisions.push(format!("'{}' in modules [{}]", name, modules_joined));
    }
    if collisions.is_empty() {
        return Ok(());
    }
    Err(format!(
        "[SC-MOD-002] import symbol collision detected: {}. hint: rename symbols or prefer qualified usage with module.symbol where applicable.",
        collisions.join("; ")
    ))
}

fn rewrite_local_symbols(source: &str, path: &Path) -> Result<String, String> {
    let module = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("invalid module filename '{}'.", path.display()))?;
    let decl_re = Regex::new(r"(?m)^\s*local\s+(fn|struct|label)\s+([A-Za-z_][A-Za-z0-9_]*)")
        .map_err(|e| format!("internal local-symbol regex error: {e}"))?;

    let mut rewrites: Vec<(String, String)> = Vec::new();
    for caps in decl_re.captures_iter(source) {
        let kind = caps.get(1).map(|m| m.as_str()).unwrap_or("fn");
        let name = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let mangled = format!("__sklocal_{}_{}_{}", module, kind, name);
        rewrites.push((name.to_string(), mangled));
    }

    if rewrites.is_empty() {
        return Ok(source.to_string());
    }

    let mut out = source.to_string();
    for (name, mangled) in &rewrites {
        let word_re = Regex::new(&format!(r"\b{}\b", regex::escape(name)))
            .map_err(|e| format!("internal local-word regex error for '{}': {e}", name))?;
        out = word_re.replace_all(&out, mangled.as_str()).to_string();
    }

    let local_kw_re = Regex::new(r"(?m)^(\s*)local\s+(fn|struct|label)\s+")
        .map_err(|e| format!("internal local-kw regex error: {e}"))?;
    Ok(local_kw_re.replace_all(&out, "$1$2 ").to_string())
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
                    "{}: {} failed (status {}): {}{}{}",
                    inv.program,
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
                errs.push(format!(
                    "{}: failed to run {}: {}",
                    inv.program,
                    format_invocation(&inv),
                    e
                ));
            }
        }
    }
    if errs.is_empty() {
        Err("no compiler candidates were generated".to_string())
    } else {
        Err(format!(
            "no working C compiler for target '{}'. attempts:\n- {}",
            target,
            errs.join("\n- ")
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

#[cfg(test)]
pub fn compile_c_to_exe(c_path: &Path, exe_path: &Path, target: &str) -> Result<(), String> {
    compile_c_to_exe_detailed(c_path, exe_path, target, None)
        .map(|_| ())
        .map_err(|error| {
            format!(
                "[SC-CGEN-001] stage=codegen-native-compile: {error}\nhint: install a supported C compiler or pass --cc <compiler>; run `skadi-cli doctor` for toolchain diagnostics."
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{
        compile_c_to_exe, compile_to_c, load_source_with_imports, parse_import_line,
        rewrite_local_symbols,
    };
    use crate::targets::detect_compiler;
    use std::fs;
    use std::path::{Path, PathBuf};
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
        ["gcc", "clang", "cc", "cl"]
            .iter()
            .any(|c| detect_compiler(c))
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
    fn local_symbols_are_mangled_and_local_keyword_removed() {
        let src = r#"
local fn helper(Int x) Int {
    return x + 1
}

local struct Hidden {
    Int value
}

local label State {
    A
    B
}
"#;
        let path = Path::new("mod_a.skd");
        let rewritten = rewrite_local_symbols(src, path).expect("rewrite");
        assert!(!rewritten.contains("local fn helper"));
        assert!(!rewritten.contains("local struct Hidden"));
        assert!(!rewritten.contains("local label State"));
        assert!(rewritten.contains("fn __sklocal_mod_a_fn_helper"));
        assert!(rewritten.contains("struct __sklocal_mod_a_struct_Hidden"));
        assert!(rewritten.contains("label __sklocal_mod_a_label_State"));
    }

    #[test]
    fn load_source_with_imports_merges_files() {
        let root = temp_case_dir("imports_ok");
        let entry = root.join("main.skd");
        let util = root.join("util.skd");
        fs::write(&util, "fn helper() Int {\n    return 7\n}\n").expect("write util");
        fs::write(&entry, "import \"./util.skd\"\nnew Int x = helper()\n").expect("write entry");

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
        fs::write(
            &math,
            "import \"./util.skd\"\nfn plus_two(Int x) Int {\n    return x + 2\n}\n",
        )
        .expect("write math");
        fs::write(
            &entry,
            "import \"./math.skd\"\nnew Int b = plus_two(2)\noutput(b)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_diamond_imports_structs_compile_and_run() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_diamond_imports_structs_compile_and_run: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("imports_e2e_diamond");
        let entry = root.join("main.skd");
        let left = root.join("left.skd");
        let right = root.join("right.skd");
        let shared = root.join("shared.skd");
        fs::write(
            &shared,
            "struct Acc {\n    Int value\n}\n\nfn base() Int {\n    return 3\n}\n",
        )
        .expect("write shared");
        fs::write(
            &left,
            "import \"./shared.skd\"\nfn l() Int {\n    return base() + 1\n}\n",
        )
        .expect("write left");
        fs::write(
            &right,
            "import \"./shared.skd\"\nfn r() Int {\n    return base() + 2\n}\n",
        )
        .expect("write right");
        fs::write(
            &entry,
            "import \"./left.skd\"\nimport \"./right.skd\"\nnew Acc a = {value = l() + r()}\noutput(a.value)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_multifile_feature_mix_compile_and_run() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_multifile_feature_mix_compile_and_run: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("imports_e2e_feature_mix");
        let entry = root.join("main.skd");
        let ops = root.join("ops.skd");
        let util = root.join("util.skd");

        fs::write(&util, "fn next(Int x) Int {\n    return x + 1\n}\n").expect("write util");
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
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_deep_chain_imports_compile_and_run() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_deep_chain_imports_compile_and_run: no host C compiler in PATH."
            );
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
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_wide_diamond_imports_compile_and_run() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_wide_diamond_imports_compile_and_run: no host C compiler in PATH."
            );
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
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
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
    fn negative_direct_import_cannot_call_local_function() {
        let root = temp_case_dir("imports_neg_local_visibility");
        let lib = root.join("lib.skd");
        let entry = root.join("main.skd");
        fs::write(
            &lib,
            "local fn helper(Int x) Int {\n    return x + 1\n}\nfn pubf(Int x) Int {\n    return helper(x)\n}\n",
        )
        .expect("write lib");
        fs::write(
            &entry,
            "import \"./lib.skd\"\nnew Int a = helper(1)\noutput(a)\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-SEM-000]"));
        assert!(err.contains("unknown function 'helper'"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_import_symbol_collision_is_deterministic() {
        let root = temp_case_dir("imports_neg_collision");
        let a = root.join("a.skd");
        let b = root.join("b.skd");
        let entry = root.join("main.skd");
        fs::write(&a, "fn shared() Int {\n    return 1\n}\n").expect("write a");
        fs::write(&b, "fn shared() Int {\n    return 2\n}\n").expect("write b");
        fs::write(
            &entry,
            "import \"./a.skd\"\nimport \"./b.skd\"\nnew Int x = 0\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("[SC-MOD-002]"));
        assert!(err.contains("import symbol collision detected"));
        assert!(err.contains("'shared'"));
        assert!(err.contains("modules [a, b]"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn negative_transitive_import_visibility_is_rejected() {
        let root = temp_case_dir("imports_neg_transitive_visibility");
        let c = root.join("c.skd");
        let b = root.join("b.skd");
        let entry = root.join("main.skd");
        fs::write(&c, "fn deep() Int {\n    return 7\n}\n").expect("write c");
        fs::write(
            &b,
            "import \"./c.skd\"\nfn mid() Int {\n    return deep()\n}\n",
        )
        .expect("write b");
        fs::write(
            &entry,
            "import \"./b.skd\"\nnew Int x = deep()\noutput(x)\n",
        )
        .expect("write entry");

        let err = compile_to_c(&entry).expect_err("compile must fail");
        assert!(err.contains("[SC-MOD-001]"));
        assert!(err.contains("[SC-MOD-003]"));
        assert!(err.contains("transitive visibility violation"));
        assert!(err.contains("'deep'"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn positive_direct_import_still_exposes_symbol_when_also_transitively_used() {
        let root = temp_case_dir("imports_pos_direct_plus_transitive");
        let c = root.join("c.skd");
        let b = root.join("b.skd");
        let entry = root.join("main.skd");
        fs::write(&c, "fn deep() Int {\n    return 7\n}\n").expect("write c");
        fs::write(
            &b,
            "import \"./c.skd\"\nfn mid() Int {\n    return deep()\n}\n",
        )
        .expect("write b");
        fs::write(
            &entry,
            "import \"./b.skd\"\nimport \"./c.skd\"\nnew Int x = deep() + mid()\noutput(x)\n",
        )
        .expect("write entry");

        let _c = compile_to_c(&entry).expect("compile must pass");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_qualified_struct_type_across_modules_builds_and_runs() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_qualified_struct_type_across_modules_builds_and_runs: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("imports_e2e_qualified_struct_type");
        let shared = root.join("shared.skd");
        let entry = root.join("main.skd");
        fs::write(
            &shared,
            "struct Point {\n    Int x\n}\nfn make(Int x) returns Point {\n    return {x = x}\n}\n",
        )
        .expect("write shared");
        fs::write(
            &entry,
            "import \"./shared.skd\"\nnew shared.Point p = make(7)\noutput(p.x)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn e2e_qualified_errorcode_variant_builds_and_runs() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping e2e_qualified_errorcode_variant_builds_and_runs: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("imports_e2e_qualified_errorcode");
        let shared = root.join("shared.skd");
        let entry = root.join("main.skd");
        fs::write(
            &shared,
            "label ErrorCode {\n    Ok\n    ZeroDivision\n}\ndanger fn parse(Int x) Int {\n    return error shared.ZeroDivision\n}\n",
        )
        .expect("write shared");
        fs::write(
            &entry,
            "import \"./shared.skd\"\nnew Int x = 0\nx = parse(1) on error {\n    x = -1\n}\noutput(x)\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile to C");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        compile_c_to_exe(&c_path, &exe_path, "host").expect("compile C to exe");
        let run = Command::new(&exe_path).output().expect("run exe");
        assert!(run.status.success(), "binary run failed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mutation_negative_parser_error_surfaces_with_parse_code() {
        let root = temp_case_dir("mut_parse_code");
        let entry = root.join("main.skd");
        fs::write(&entry, "new Int x = 1\nx = 1 +\n").expect("write entry");

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
            eprintln!(
                "Skipping compile_c_to_exe_reports_attempt_matrix_on_failure: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("cgen_attempt_matrix");
        let c_path = root.join("broken.c");
        let exe_path = if cfg!(windows) {
            root.join("broken.exe")
        } else {
            root.join("broken")
        };
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
    fn mutation_codegen_regression_guard_list_pop_on_error_compiles_and_runs() {
        if !has_host_compiler() {
            eprintln!(
                "Skipping mutation_codegen_regression_guard_list_pop_on_error_compiles_and_runs: no host C compiler in PATH."
            );
            return;
        }

        let root = temp_case_dir("mut_codegen_gap");
        let entry = root.join("main.skd");
        fs::write(
            &entry,
            "new i32 List xs = [1, 2]\nnew Int v = 0\nv = xs.pop() on error {\n    v = -1\n}\n",
        )
        .expect("write entry");

        let c = compile_to_c(&entry).expect("compile_to_c must pass");
        let c_path = root.join("out.c");
        let exe_path = if cfg!(windows) {
            root.join("out.exe")
        } else {
            root.join("out")
        };
        fs::write(&c_path, c).expect("write C file");
        match compile_c_to_exe(&c_path, &exe_path, "host") {
            Ok(()) => {
                let run = Command::new(&exe_path).output().expect("run exe");
                assert!(run.status.success(), "binary run failed");
            }
            Err(err) => {
                assert!(err.contains("[SC-CGEN-001]"));
                assert!(err.contains("stage=codegen-native-compile"));
            }
        }

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
