// src/ast_nodes.rs
use std::collections::HashMap;

/// Represents a single location in the source file.
#[derive(Debug, Clone)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

impl Default for Location {
    fn default() -> Self {
        Location { line: 1, column: 1 }
    }
}

// --- Core Nodes (Structural Contracts) ---

/// The root of the Abstract Syntax Tree.
#[derive(Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new() -> Self {
        Program {
            statements: Vec::new(),
        }
    }
}

/// A general container for all top-level executable structures (Function definitions, etc.)
#[derive(Debug)]
pub struct FunctionParam {
    pub name: String,
    pub param_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub field_type: String,
    pub name: String,
}

#[derive(Debug)]
pub struct StructMethod {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub body: Box<BlockStatement>,
    pub returns: Option<String>,
    pub is_danger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForLoopStyle {
    ForIn,
    IterateAs,
    LegacyCStyle,
}

#[derive(Debug, Clone)]
pub struct LegacyForParts {
    pub initialization: String,
    pub condition: String,
    pub update: String,
}

#[derive(Debug)]
pub enum Statement {
    VarDecl {
        name: String,
        value: Box<Expression>,
        is_fixed: bool,
        declared_type: Option<String>,
        loc: Location,
    },
    MemoryDecl {
        name: String,
        size_spec: String,
        on_error: Option<Box<BlockStatement>>,
        loc: Location,
    },
    Assignment {
        target: String,
        value: Box<Expression>,
        loc: Location,
    },
    IncDec {
        target: String,
        is_increment: bool,
        loc: Location,
    },
    FieldAssignment {
        object: String,
        field: String,
        value: Box<Expression>,
        loc: Location,
    },
    FunctionDef {
        name: String,
        params: Vec<FunctionParam>,
        body: Box<BlockStatement>,
        returns: Option<String>, // Type of return
        is_danger: bool,
        loc: Location,
    },
    IfStatement {
        condition: Box<Expression>,
        then_block: Box<BlockStatement>,
        else_block: Option<Box<BlockStatement>>,
        loc: Location,
    },
    ForLoop {
        initialization: Option<Box<Expression>>,
        condition: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        legacy_parts: Option<LegacyForParts>,
        style: ForLoopStyle,
        body: Box<BlockStatement>,
        loc: Location,
    },
    WhenBlock {
        when_expression: Box<Expression>,
        cases: Vec<(Vec<Expression>, Box<BlockStatement>)>,
        else_block: Option<Box<BlockStatement>>,
        loc: Location,
    },
    WhileLoop {
        condition: Box<Expression>,
        body: Box<BlockStatement>,
        loc: Location,
    },
    LoopStatement {
        body: Box<BlockStatement>,
        loc: Location,
    },
    BreakStatement {
        loc: Location,
    },
    ContinueStatement {
        loc: Location,
    },
    PassStatement {
        loc: Location,
    },
    LabelDecl {
        name: String,
        variants: Vec<String>,
        loc: Location,
    },
    StructDecl {
        name: String,
        fields: Vec<StructField>,
        methods: Vec<StructMethod>,
        loc: Location,
    },
    OnBlock {
        trigger: String,
        target: Option<String>,
        body: Box<BlockStatement>,
        loc: Location,
    },
    DangerAssignOnError {
        target: String,
        call_name: String,
        args: Vec<Expression>,
        on_error: Box<BlockStatement>,
        loc: Location,
    },
    DangerCallOnError {
        call_name: String,
        args: Vec<Expression>,
        on_error: Box<BlockStatement>,
        loc: Location,
    },
    ListPush {
        list_name: String,
        value: Box<Expression>,
        loc: Location,
    },
    ListPopOnError {
        target: String,
        list_name: String,
        on_error: Box<BlockStatement>,
        loc: Location,
    },
    PlaceIn {
        memory_name: String,
        on_error: Option<Box<BlockStatement>>,
        body: Box<BlockStatement>,
        loc: Location,
    },
    MemoryClear {
        memory_name: String,
        loc: Location,
    },
    StopTask {
        task_name: String,
        loc: Location,
    },
    ReturnError {
        code: String,
        loc: Location,
    },
    ReturnStatement {
        value: Option<Box<Expression>>,
        loc: Location,
    },
    ExpressionStatement {
        expr: Box<Expression>,
        loc: Location,
    },
    BlockStatement {
        statements: Vec<Statement>,
        loc: Location,
    },
    OnErrorBlock {
        statements: Vec<Statement>,
        loc: Location,
    }, // For 'on error' context
}

/// Represents a sequence of statements executed together (function body, if/else block).
#[derive(Debug)]
pub struct BlockStatement {
    pub statements: Vec<Statement>,
}

impl From<Vec<Statement>> for Box<BlockStatement> {
    fn from(statements: Vec<Statement>) -> Self {
        Box::new(BlockStatement { statements })
    }
}

/// Represents an expression, which can be anything that evaluates to a value (variable, literal, call).
#[derive(Debug)]
pub enum Expression {
    LiteralInt(i64),   // Simple integer literal
    LiteralFloat(f32), // Floating point literal
    LiteralBool(bool),
    LiteralString(String),
    ListLiteral(Vec<Expression>),
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    VariableReference(String), // Usage of a defined variable name
    MemberAccess {
        base: String,
        field: String,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
    RunTask {
        call_name: String,
        args: Vec<Expression>,
    },
    WaitTask {
        task_name: String,
    },
    Stopping,
    BinaryOp {
        op: String, // Operator (+, -, etc.)
        left: Box<Expression>,
        right: Option<Box<Expression>>,
    },
    StructConstruction {
        fields: HashMap<String, Box<Expression>>,
    }, // e.g., WeatherData { temperature = ... }
}

// --- Scope Management (Symbol Table Implementation) ---
#[derive(Debug)]
pub struct ScopeManager {
    scope_stack: Vec<HashMap<String, &'static str>>,
}

impl ScopeManager {
    /// Creates a new scope manager initialized with an optional global scope provided by the parent context.
    pub fn new(parent_scope: Option<&HashMap<String, &'static str>>) -> Self {
        let base_scope = parent_scope.cloned().unwrap_or_default();
        ScopeManager {
            // Initialize stack with the global/parent scope if present, otherwise start fresh.
            scope_stack: vec![base_scope],
        }
    }

    /// Enters a new lexical scope (e.g., entering function body or 'if' block).
    /// Returns a mutable reference to the newly added, empty scope for immediate definition use (e.g., `let`).
    pub fn enter_scope(&mut self) -> &mut HashMap<String, &'static str> {
        self.scope_stack.push(HashMap::new());
        self.scope_stack
            .last_mut()
            .expect("scope stack is never empty")
    }

    /// Exits the current lexical scope, discarding variables defined within it.
    pub fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        } else {
            // Safety check: Never pop the base global scope
            eprintln!("Warning: Attempted to pop the global scope.");
        }
    }

    /// Defines a symbol (variable/function) in the *currently active* lexical scope. Returns an error if already present.
    pub fn define_symbol(
        &mut self,
        name: &str,
        kind_type: &'static str,
    ) -> Result<(), &'static str> {
        let current = self
            .scope_stack
            .last_mut()
            .expect("scope stack is never empty");
        if current.contains_key(name) {
            return Err("Symbol already defined in this scope (re-declaration detected).");
        }
        current.insert(name.to_string(), kind_type);
        Ok(())
    }

    /// Looks up a symbol name, searching from the *local* scope outwards through all active scopes until found.
    pub fn lookup(&self, name: &str) -> Option<&'static str> {
        for scope in self.scope_stack.iter().rev() {
            // Search reverse (top-down/local to global)
            if let Some(type_info) = scope.get(name) {
                return Some(type_info);
            }
        }
        None
    }
}
