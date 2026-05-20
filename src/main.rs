use std::fs;
use v01::lexer::core::lex;

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
        }
        Err(e) => {
            eprintln!("Lexing failed: {}", e);
        }
    }
}
