pub mod common_types;
pub mod lexer_utils;

// Обновление: Импортируем новую, декомпозированную структуру лексера
pub mod lexer {
    pub mod structures;
    pub mod core;
}

pub mod ast_nodes;
pub mod parsing_logic;
pub mod parser;
// Add other modules here as they are developed (e.g., semantic_analysis)

// The main entry point for the public compiler API will be exposed via this module tree structure.
