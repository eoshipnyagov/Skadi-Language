use std::fs;
use std::process::Command;
use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::{semantic_analyze, semantic_style_warnings};

fn print_usage() {
    println!("Skadi compiler (current toolchain)");
    println!("Usage:");
    println!("  cargo run -- --input <file.skd> [--print-c] [--emit-c <out.c>] [--emit-exe <out.exe>]");
    println!("  cargo run -- <file.skd> [--print-c] [--emit-c <out.c>] [--emit-exe <out.exe>]");
    println!();
    println!("Options:");
    println!("  --input <path>      Source .skd file");
    println!("  --emit-c <path>     Write generated C code to file");
    println!("  --emit-exe <path>   Build native executable via gcc/clang");
    println!("  --print-c           Print generated C to stdout");
    println!("  --help              Show this help");
}

fn compile_c_to_exe(c_path: &str, exe_path: &str) -> Result<(), String> {
    let candidates = ["gcc", "clang"];
    let mut last_err: Option<String> = None;

    for compiler in candidates {
        let output = Command::new(compiler).arg(c_path).arg("-o").arg(exe_path).output();
        match output {
            Ok(out) => {
                if out.status.success() {
                    return Ok(());
                }
                let stderr = String::from_utf8_lossy(&out.stderr);
                last_err = Some(format!(
                    "{} failed with status {}: {}",
                    compiler,
                    out.status,
                    stderr.trim()
                ));
            }
            Err(e) => {
                last_err = Some(format!("failed to run {}: {}", compiler, e));
            }
        }
    }

    Err(last_err.unwrap_or_else(|| "no C compiler found (tried gcc, clang)".to_string()))
}

fn main() {
    let mut input_file = "examples/example_meteostation.txt".to_string();
    let mut emit_c_path: Option<String> = None;
    let mut emit_exe_path: Option<String> = None;
    let mut print_c = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--emit-c" => {
                if i + 1 >= args.len() {
                    eprintln!("Missing value for --emit-c");
                    return;
                }
                emit_c_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--emit-exe" => {
                if i + 1 >= args.len() {
                    eprintln!("Missing value for --emit-exe");
                    return;
                }
                emit_exe_path = Some(args[i + 1].clone());
                i += 2;
            }
            "--print-c" => {
                print_c = true;
                i += 1;
            }
            "--help" => {
                print_usage();
                return;
            }
            "--input" => {
                if i + 1 >= args.len() {
                    eprintln!("Missing value for --input");
                    return;
                }
                input_file = args[i + 1].clone();
                i += 2;
            }
            other if other.starts_with("--") => {
                eprintln!("Unknown option: {}", other);
                return;
            }
            path => {
                input_file = path.to_string();
                i += 1;
            }
        }
    }

    let source_code = match fs::read_to_string(&input_file) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", input_file, e);
            return;
        }
    };

    match lex(&source_code) {
        Ok(tokens) => {
            println!("Lexing completed successfully. Token count: {}", tokens.len());
            match parse_program(&tokens) {
                Ok(program) => {
                    println!("Parsing completed successfully. Statement count: {}", program.statements.len());
                    match semantic_analyze(&program) {
                        Ok(()) => {
                            println!("Semantic analysis completed successfully.");
                            for warning in semantic_style_warnings(&program) {
                                eprintln!("{}", warning);
                            }
                            let c_code = transpile_program_to_c(&program);
                            println!("C transpilation completed. Output size: {} bytes", c_code.len());

                            if print_c {
                                println!("\n----- GENERATED C -----\n{}\n-----------------------", c_code);
                            }

                            if let Some(path) = emit_c_path {
                                match fs::write(&path, c_code.as_bytes()) {
                                    Ok(_) => println!("C output written to {}", path),
                                    Err(e) => eprintln!("Failed to write C output to '{}': {}", path, e),
                                }
                            }

                            if let Some(exe_path) = emit_exe_path {
                                let temp_c_path = format!("{}.skd_tmp.c", exe_path);
                                match fs::write(&temp_c_path, c_code.as_bytes()) {
                                    Ok(_) => {
                                        match compile_c_to_exe(&temp_c_path, &exe_path) {
                                            Ok(()) => println!("Executable built: {}", exe_path),
                                            Err(e) => eprintln!("Failed to build executable '{}': {}", exe_path, e),
                                        }
                                        if let Err(e) = fs::remove_file(&temp_c_path) {
                                            eprintln!(
                                                "Warning: failed to remove temporary C file '{}': {}",
                                                temp_c_path, e
                                            );
                                        }
                                    }
                                    Err(e) => eprintln!(
                                        "Failed to write temporary C file '{}': {}",
                                        temp_c_path, e
                                    ),
                                }
                            }
                        }
                        Err(e) => eprintln!("Semantic analysis failed: {}", e),
                    }
                }
                Err(e) => {
                    eprintln!("Parsing failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Lexing failed: {}", e);
        }
    }
}



