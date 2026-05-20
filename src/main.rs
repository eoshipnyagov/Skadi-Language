use std::fs;
use v01::lexer::core::lex;
use v01::parser::parse_program;

fn main() {
    let source_file = "example_meteostation.txt";
    let source_code = match fs::read_to_string(source_file) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Failed to read '{}': {}", source_file, e);
            return;
        }
    };

    match lex(&source_code) {
        Ok(tokens) => {
            println!("Lexing completed successfully. Token count: {}", tokens.len());
            match parse_program(&tokens) {
                Ok(program) => {
                    println!("Parsing completed successfully. Statement count: {}", program.statements.len());
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
