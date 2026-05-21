use std::fs;
use v01::codegen::transpile_program_to_c;
use v01::lexer::lex;
use v01::parser::parse_program;
use v01::semantic_analysis::semantic_analyze;

fn main() {
    let mut input_file = "example_meteostation.txt".to_string();
    let mut emit_c_path: Option<String> = None;
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
            "--print-c" => {
                print_c = true;
                i += 1;
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
