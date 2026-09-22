use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn semantic_ok(source: &str) -> v01::ast_nodes::Program {
    let tokens = lex(source).expect("lex File source");
    let program = parse_program(&tokens).expect("parse File source");
    semantic_analyze(&program).expect("semantic File source");
    program
}

fn semantic_err(source: &str) -> String {
    let tokens = lex(source).expect("lex invalid File source");
    let program = parse_program(&tokens).expect("parse invalid File source");
    semantic_analyze(&program).expect_err("expected File semantic error")
}

#[test]
fn file_resource_supports_fallible_open_read_write_and_close() {
    let program = semantic_ok(
        r#"
fn round_trip(Path path) returns Int {
    new File output_file = fs.open(path, FileMode.Write) on error {
        return -1
    }
    output_file.write("Skadi file resource") on error {
        return -2
    }
    output_file.close() on error {
        return -3
    }

    new File input_file = fs.open(path, FileMode.Read) on error {
        return -4
    }
    new Text data = input_file.read_all() on error {
        return -5
    }
    output(data)
    return 0
}
"#,
    );

    let c = transpile_program_to_c(&program);
    assert!(
        c.contains("typedef struct { FILE *handle; SkFileMode mode; } SkFile;"),
        "{c}"
    );
    assert!(
        c.contains("sk_file_open(path, FileMode_Write, &output_file)"),
        "{c}"
    );
    assert!(
        c.contains("sk_file_write(&output_file, \"Skadi file resource\")"),
        "{c}"
    );
    assert!(c.contains("sk_file_close(&output_file)"), "{c}");
    assert!(c.contains("sk_file_read_all(&input_file, &data)"), "{c}");
    assert!(c.contains("sk_file_destroy(&input_file);"), "{c}");
    assert!(c.contains("sk_free_text((void*)data);"), "{c}");
}

#[test]
fn file_open_and_methods_require_on_error() {
    let open = semantic_err(
        r#"
new File file = fs.open("data.txt", FileMode.Read)
"#,
    );
    assert!(
        open.contains("fs.open is fallible and requires a typed declaration"),
        "{open}"
    );

    let read = semantic_err(
        r#"
fn read_file(Path path) returns Int {
    new File file = fs.open(path, FileMode.Read) on error {
        return -1
    }
    new Text data = file.read_all()
    return 0
}
"#,
    );
    assert!(
        read.contains("File method 'file.read_all' is fallible and requires 'on error'"),
        "{read}"
    );
}

#[test]
fn file_modes_are_nominal_and_file_view_is_read_only() {
    let mode = semantic_err(
        r#"
new File file = fs.open("data.txt", 1) on error {
    return
}
"#,
    );
    assert!(mode.contains("fs.open mode expects FileMode"), "{mode}");

    let view = semantic_err(
        r#"
fn inspect(view File file) returns Int {
    new Text data = file.read_all() on error {
        return -1
    }
    return 0
}
"#,
    );
    assert!(view.contains("view parameter 'file' cannot call"), "{view}");
}

#[test]
fn closed_file_cannot_be_used_again() {
    let error = semantic_err(
        r#"
fn inspect(Path path) returns Int {
    new File file = fs.open(path, FileMode.Read) on error {
        return -1
    }
    file.close() on error {
        return -2
    }
    new Text data = file.read_all() on error {
        return -3
    }
    return 0
}
"#,
    );
    assert!(
        error.contains("resource 'file' is already closed"),
        "{error}"
    );
}

#[test]
fn owning_files_cannot_be_nested_in_containers() {
    let error = semantic_err("new File List files = []\n");
    assert!(
        error.contains("cannot contain an owning resource"),
        "{error}"
    );
}

#[test]
fn file_resource_round_trip_runs_natively() {
    let compiler = ["gcc", "clang", "cc"]
        .into_iter()
        .find(|candidate| Command::new(candidate).arg("--version").output().is_ok());
    let Some(compiler) = compiler else {
        eprintln!("Skipping File runtime test: no C compiler in PATH.");
        return;
    };

    let program = semantic_ok(
        r#"
fn round_trip() returns Int {
    new File output_file = fs.open("round-trip.txt", FileMode.Write) on error {
        return -1
    }
    output_file.write("Skadi file resource") on error {
        return -2
    }
    output_file.close() on error {
        return -3
    }
    new File input_file = fs.open("round-trip.txt", FileMode.Read) on error {
        return -4
    }
    new Text data = input_file.read_all() on error {
        return -5
    }
    output(data)
    return 0
}

new Int status = round_trip()
"#,
    );
    let c = transpile_program_to_c(&program);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_millis();
    let work = std::env::temp_dir().join(format!("skadi_file_resource_{stamp}"));
    fs::create_dir_all(&work).expect("create File runtime directory");
    let c_path = work.join("file_resource.c");
    let mut exe_path = work.join("file_resource");
    if cfg!(windows) {
        exe_path.set_extension("exe");
    }
    fs::write(&c_path, c).expect("write generated C");

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
    let run = Command::new(&exe_path)
        .current_dir(&work)
        .output()
        .expect("run File resource binary");
    assert!(run.status.success(), "runtime failed: {run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "Skadi file resource"
    );

    fs::remove_dir_all(work).expect("remove File runtime directory");
}
