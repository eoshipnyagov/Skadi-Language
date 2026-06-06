// ================================================
// Skadi Common Types (Rust)
// File: src/common_types.rs
// ----------------------------------------------------------------

/// Represents the category and type of token found in the source code.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    KeywordFn,           // fn
    KeywordStruct,       // struct
    KeywordLabel,        // label
    KeywordIf,           // if
    KeywordElse,         // else
    KeywordWhen,         // when
    KeywordIs,           // is
    KeywordFor,          // for
    KeywordIn,           // in
    KeywordWhile,        // while
    KeywordLoop,         // loop
    KeywordBreak,        // break
    KeywordContinue,     // continue
    KeywordPass,         // pass
    KeywordReturn,       // return
    KeywordNew,          // new
    KeywordFixed,        // fixed (or const)
    KeywordConst,        // const
    KeywordHide,         // hide
    KeywordLocal,        // local
    KeywordMy,           // my
    KeywordDirect,       // direct
    KeywordAllowDrop,    // allow drop
    KeywordOnError,      // on error
    KeywordOnInterrupt,  // on interrupt
    KeywordOnErrorBlock, // Used for the block context (on error { ... })

    // Primitive Types
    TypeInt,    // Integer literals (123)
    TypeFloat,  // Floating point literals (3.14)
    TypeBool,   // true, false
    TypeChar,   // char literal ('a')
    TypeString, // String or Text literals ("...")

    // Structural Types
    Identifier,          // General identifiers (variables, function names)
    KeywordOperatorName, // Special tokens like "fn", "struct" (Kept for robust matching)

    // Operators - Separating classes is critical for parsing logic.
    OpAssignment,  // = (e.g., x = 5)
    OpArithmetic,  // +, -, *, /, %, ^ (Primary arithmetic operations)
    OpComparison,  // ==, !=, >=, <= (Relational checks)
    OpLogical,     // &&, || (Boolean conjunction/disjunction)
    OpPunctuation, // :, ., ,, (, ), [, ] (Structural separators and flow operators)
    OpIncDec,      // ++, -- (statement-only increment/decrement)

    // Special Markers
    Whitespace, // Tokenized whitespace (to be filtered out later)
    NewLine,    // A newline character
}

/// Represents a single token found during lexical analysis.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: u32,
    pub col: u32,
}

impl Token {
    pub fn kind(&self) -> TokenKind {
        self.kind.clone()
    }
}
