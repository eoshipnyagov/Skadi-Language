use std::fs;
use std::path::PathBuf;

use v01::codegen::ensure_codegen_supported;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn example_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn read_example(rel: &str) -> String {
    fs::read_to_string(example_path(rel)).expect("example file should be readable")
}

fn parse_ok(rel: &str) -> v01::ast_nodes::Program {
    let src = read_example(rel);
    let tokens = lex(&src).expect("lex should succeed");
    parse_program(&tokens).expect("parse should succeed")
}

#[test]
fn positive_memory_examples_pass_frontend_and_stop_at_backend_gate() {
    let examples = [
        "examples/memory/positive/01_loaded_text_asset.skd",
        "examples/memory/positive/02_local_scratch_preview.skd",
        "examples/memory/positive/03_sensor_batch_external_memory.skd",
        "examples/memory/positive/04_explicit_recovery.skd",
    ];

    for rel in examples {
        let program = parse_ok(rel);
        semantic_analyze(&program).unwrap_or_else(|err| {
            panic!("semantic analysis should pass for {rel}: {err}");
        });
        let err = ensure_codegen_supported(&program).expect_err("backend gate should fail");
        assert!(
            err.contains("SC-CG-201"),
            "expected SC-CG-201 for {rel}, got: {err}"
        );
    }
}

#[test]
fn memory_style_pitfall_example_emits_collapsed_name_warning() {
    let program = parse_ok("examples/memory/pitfalls/01_collapsed_field_names.skd");
    semantic_analyze(&program).expect("semantic analysis should pass");
    let warnings = semantic_style_warnings(&program);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("avoid collapsed field init")),
        "expected collapsed field init warning, got: {warnings:?}"
    );
}

#[test]
fn negative_memory_examples_fail_with_expected_diagnostics() {
    let examples = [
        (
            "examples/memory/negative/01_local_memory_return_escape.skd",
            "SC-SEM-061",
            "local Memory",
        ),
        (
            "examples/memory/negative/02_in_block_clear.skd",
            "SC-SEM-060",
            "forbidden in-block clear",
        ),
        (
            "examples/memory/negative/03_memory_in_struct.skd",
            "SC-SEM-062",
            "struct field type",
        ),
        (
            "examples/memory/negative/04_memory_list.skd",
            "SC-SEM-062",
            "variable declaration type",
        ),
        (
            "examples/memory/negative/05_memory_copy_assignment.skd",
            "SC-SEM-062",
            "cannot be reassigned or copied",
        ),
        (
            "examples/memory/negative/06_use_after_clear.skd",
            "SC-SEM-061",
            "use-after-clear",
        ),
        (
            "examples/memory/negative/07_store_into_longer_lived_owner.skd",
            "SC-SEM-060",
            "longer-lived owner",
        ),
    ];

    for (rel, code, marker) in examples {
        let program = parse_ok(rel);
        let err = semantic_analyze(&program).expect_err("semantic analysis should fail");
        assert!(err.contains(code), "expected {code} for {rel}, got: {err}");
        assert!(
            err.contains(marker),
            "expected marker '{marker}' for {rel}, got: {err}"
        );
    }
}
