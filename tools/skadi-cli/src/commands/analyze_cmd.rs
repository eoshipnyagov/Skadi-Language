use serde_json::{Value, json};

use crate::actions::{self, ActionError, AnalysisFact, DiagnosticSummary, FailureSource};

pub fn run(args: &[String]) -> Result<(), String> {
    let json_output = match args {
        [] => false,
        [flag] if matches!(flag.as_str(), "--help" | "-h") => {
            println!("skadi-cli analyze [--json]");
            println!("Explain lifecycle and blocking behavior for the current project.");
            println!("Use --json for the stable skadi.analysis.v1 tool contract.");
            return Ok(());
        }
        [flag] if flag == "--json" => true,
        _ => {
            return Err(if args.iter().any(|arg| arg == "--json") {
                json!({
                    "schema": "skadi.analysis.v1",
                    "ok": false,
                    "source": "usage",
                    "message": "usage: skadi-cli analyze [--json]",
                    "diagnostics": []
                })
                .to_string()
            } else {
                "usage: skadi-cli analyze [--json]".to_string()
            });
        }
    };

    match actions::run_check() {
        Ok(result) if json_output => {
            let facts = result.analysis.iter().map(fact_json).collect::<Vec<_>>();
            let warnings = result
                .warnings
                .iter()
                .map(diagnostic_json)
                .collect::<Vec<_>>();
            println!(
                "{}",
                json!({
                    "schema": "skadi.analysis.v1",
                    "ok": true,
                    "project": {
                        "root": result.project.cwd,
                        "entry": result.entry,
                    },
                    "warnings": warnings,
                    "facts": facts,
                })
            );
            Ok(())
        }
        Ok(result) => {
            println!("analysis ok: {}", result.entry.display());
            println!(
                "facts: {} | warnings: {}",
                result.analysis.len(),
                result.warnings.len()
            );
            for fact in result.analysis {
                let subject = fact
                    .subject
                    .as_deref()
                    .map(|name| format!(" subject='{name}'"))
                    .unwrap_or_default();
                println!(
                    "[{}] {} {}:{} {}{}",
                    fact.level_name(),
                    fact.code,
                    fact.span.start_line,
                    fact.span.start_col,
                    fact.summary,
                    subject
                );
                println!("  why: {}", fact.explanation);
                println!("  next: {}", fact.action);
            }
            Ok(())
        }
        Err(error) if json_output => Err(error_json(&error).to_string()),
        Err(error) => Err(error.to_string()),
    }
}

fn fact_json(fact: &AnalysisFact) -> Value {
    json!({
        "id": fact.id,
        "code": fact.code,
        "kind": fact.kind_name(),
        "level": fact.level_name(),
        "span": {
            "start": { "line": fact.span.start_line, "col": fact.span.start_col },
            "end": { "line": fact.span.end_line, "col": fact.span.end_col },
        },
        "context": fact.context,
        "subject": fact.subject.as_ref().map(|name| json!({
            "id": fact.subject_id,
            "kind": fact.subject_kind.map(|kind| kind.as_str()),
            "name": name,
        })),
        "summary": fact.summary,
        "explanation": fact.explanation,
        "action": fact.action,
    })
}

fn diagnostic_json(diagnostic: &DiagnosticSummary) -> Value {
    json!({
        "stage": diagnostic.stage,
        "code": diagnostic.code,
        "line": diagnostic.line,
        "col": diagnostic.col,
        "message": diagnostic.message,
        "warning": diagnostic.is_warning,
    })
}

fn error_json(error: &ActionError) -> Value {
    json!({
        "schema": "skadi.analysis.v1",
        "ok": false,
        "source": failure_source_name(error.source),
        "message": error.message,
        "diagnostics": error.diagnostics.iter().map(diagnostic_json).collect::<Vec<_>>(),
        "facts": [],
    })
}

fn failure_source_name(source: FailureSource) -> &'static str {
    match source {
        FailureSource::Frontend => "frontend",
        FailureSource::Toolchain => "toolchain",
        FailureSource::Runtime => "runtime",
        FailureSource::Project => "project",
        FailureSource::Io => "io",
        FailureSource::Usage => "usage",
    }
}
