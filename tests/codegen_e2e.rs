use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn find_c_compiler() -> Option<&'static str> {
    for c in ["clang", "gcc", "cc"] {
        if Command::new(c).arg("--version").output().is_ok() {
            return Some(c);
        }
    }
    None
}

#[test]
fn e2e_skadi_to_c_binary_builds() {
    let Some(compiler) = find_c_compiler() else {
        eprintln!("Skipping e2e C build test: no clang/gcc/cc in PATH.");
        return;
    };

    let src = r#"
new Int x = 2
new Int y = x + 3
"#;

    let tokens = lex(src).expect("lex should succeed");
    let program = parse_program(&tokens).expect("parse should succeed");
    semantic_analyze(&program).expect("semantic should pass");
    let c = transpile_program_to_c(&program);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let mut c_path: PathBuf = std::env::temp_dir();
    c_path.push(format!("scadi_e2e_{stamp}.c"));
    let mut exe_path: PathBuf = std::env::temp_dir();
    exe_path.push(format!("scadi_e2e_{stamp}"));
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }

    fs::write(&c_path, c).expect("write C source");

    let compile = Command::new(compiler)
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("run C compiler");
    assert!(
        compile.status.success(),
        "C compile failed: {}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&exe_path).output().expect("run compiled binary");
    assert!(
        run.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );

    let _ = fs::remove_file(c_path);
    let _ = fs::remove_file(exe_path);
}
