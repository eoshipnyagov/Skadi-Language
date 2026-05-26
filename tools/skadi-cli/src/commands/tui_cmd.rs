use std::io::{self, Write};

pub fn run(_args: &[String]) -> Result<(), String> {
    println!("Skadi TUI (minimal)\n");
    println!("0) Exit");
    println!("1) New project");
    println!("2) Init current directory");
    println!("3) Check project");
    println!("4) Build project");
    println!("5) Run project");
    print!("Select [0-5]: ");
    io::stdout().flush().map_err(|e| format!("flush failed: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("read failed: {e}"))?;

    let result = match input.trim() {
        "0" => return Ok(()),
        "1" => {
            print!("Project name: ");
            io::stdout().flush().map_err(|e| format!("flush failed: {e}"))?;
            let mut name = String::new();
            io::stdin()
                .read_line(&mut name)
                .map_err(|e| format!("read failed: {e}"))?;
            let name = name.trim();
            if name.is_empty() {
                return Err("project name cannot be empty".to_string());
            }
            super::new_cmd::run(&[name.to_string()])
        }
        "2" => super::init_cmd::run(&[]),
        "3" => super::check_cmd::run(&[]),
        "4" => super::build_cmd::run(&[]),
        "5" => super::run_cmd::run(&[]),
        _ => Err("unknown selection".to_string()),
    };

    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            render_error_table(&err);
            Err(err)
        }
    }
}

fn render_error_table(err: &str) {
    if let Some(diag) = parse_diagnostic(err) {
        println!();
        println!("+----------+------------+------+-----+--------------------------------------+");
        println!(
            "| {:8} | {:10} | {:4} | {:3} | {:36} |",
            diag.kind,
            diag.code.as_deref().unwrap_or("-"),
            diag.line.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
            diag.col.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()),
            truncate(&diag.message, 36)
        );
        println!("+----------+------------+------+-----+--------------------------------------+");
        return;
    }

    println!();
    println!("+----------+------------+------+-----+--------------------------------------+");
    println!(
        "| {:8} | {:10} | {:4} | {:3} | {:36} |",
        "General",
        "-",
        "-",
        "-",
        truncate(err, 36)
    );
    println!("+----------+------------+------+-----+--------------------------------------+");
}

struct ParsedDiagnostic {
    kind: String,
    code: Option<String>,
    line: Option<u32>,
    col: Option<u32>,
    message: String,
}

fn parse_diagnostic(input: &str) -> Option<ParsedDiagnostic> {
    let (kind, rest) = if let Some(x) = input.strip_prefix("Semantic error at ") {
        ("Semantic".to_string(), x)
    } else if let Some(x) = input.strip_prefix("Parse error at ") {
        ("Parse".to_string(), x)
    } else if let Some(x) = input.strip_prefix("Lex error at ") {
        ("Lex".to_string(), x)
    } else {
        return None;
    };

    let line = extract_after(rest, "line ")
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<u32>().ok());
    let col = extract_after(rest, "col ")
        .and_then(|s| s.split(':').next())
        .and_then(|s| s.trim().parse::<u32>().ok());

    let code = extract_between(rest, "[", "]");
    let message = if let Some(pos) = rest.find("] ") {
        rest[(pos + 2)..].trim().to_string()
    } else if let Some(pos) = rest.find(": ") {
        rest[(pos + 2)..].trim().to_string()
    } else {
        rest.trim().to_string()
    };

    Some(ParsedDiagnostic {
        kind,
        code,
        line,
        col,
        message,
    })
}

fn extract_after<'a>(s: &'a str, needle: &str) -> Option<&'a str> {
    let idx = s.find(needle)?;
    Some(&s[(idx + needle.len())..])
}

fn extract_between(s: &str, left: &str, right: &str) -> Option<String> {
    let start = s.find(left)? + left.len();
    let tail = &s[start..];
    let end = tail.find(right)?;
    Some(tail[..end].to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max.saturating_sub(3) {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    out
}
