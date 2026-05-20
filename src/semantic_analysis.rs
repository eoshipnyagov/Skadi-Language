// src/semantic_analysis.rs
use std::collections::{HashMap, HashSet};
use crate::ast_nodes::{Program, Statement, Expression, ScopeManager};

/// Executes the full semantic analysis pass on a given Program AST root.
pub fn semantic_analyze(program: &Program) -> Result<(), String> {
    println!("==========================================");
    println!("--- Starting Skadi Semantic Analysis ---");
    println!("------------------------------------------");

    // Symbol table tracks defined identifiers (variables/functions) and their scope, type, and memory footprint.
    let mut global_scope: HashMap<String, &'static str> = HashMap::new(); 
    let mut current_memory_budget: u64 = 0;

    for stmt in &program.statements {
        println!("Analyzing statement...");
        // We start a new scope for each top-level statement/function definition.
        match analyze_statement(stmt, &mut global_scope, &mut current_memory_budget) {
            Ok(_) => println!("[OK] Statement analyzed successfully."),
            Err(e) => {
                return Err(format!("Semantic Failure in statement: {}", e)); 
            }
        }
    }

    println!("\n==========================================");
    if current_memory_budget > 0 {
         println!("SUCCESS: Semantic analysis completed. Estimated final budget requirement: {} bytes.", current_memory_budget);
    } else {
         println!("SUCCESS: Semantic analysis completed with a clean scope and minimal memory usage.");
    }

    Ok(())
}

/// Recursively analyzes an individual statement within its given scope context.
fn analyze_statement(stmt: &Statement, scope: &mut HashMap<String, &'static str>, budget: &mut u64) -> Result<(), String> {
    match stmt {
        // 1. Variable Declaration/Assignment
        Statement::VarDecl { name, value, is_fixed } => {
            if scope.contains_key(name.as_str()) {
                return Err("Variable re-declaration in this scope.");
            }
            let value_type = analyze_expression(value, scope);
            scope.insert(name.as_str(), "TypePlaceholder"); // Should store the resolved type string
            println!("  - Declared variable '{}' of inferred type: {}", name, value_type);
            // Crude budget estimation: assuming minimal storage cost for now.
            *budget += 8; 
            Ok(())
        }

        Statement::Assignment { target, value } => {
             if !scope.contains_key(target.as_str()) {
                return Err("Cannot assign to undeclared variable or field.");
            }
            let value_type = analyze_expression(value, scope);
            println!("  - Assigned value to '{}'. Value type: {}", target, value_type);
            Ok(())
        }

        // 2. Function Definition (Skipped complex analysis for brevity)
        Statement::FunctionDef { name, params, body, returns, is_danger } => {
            println!("  - Defining function '{}' (Danger: {}).", name, is_danger);
            let new_scope = ScopeManager{ parent_scope: scope.clone() }; 
            // We pass 'global_scope' here to simulate nested context management
            if let Err(e) = analyze_block(body, &mut new_scope, budget) {
                return Err(format!("Error in function '{}': {}", name, e));
            }

            println!("  - Function '{}' analysis OK.", name);
            // We assume successful function definition adds it to the global scope.
            scope.insert(name.as_str(), "Function"); 
            Ok(())
        }


        // 3. Control Flow: If/When
        Statement::IfStatement { condition, then_block, else_block } => {
            let cond_type = analyze_expression(condition, &mut HashMap::new());
            println!("  - IF statement analyzed. Condition type: {}", cond_type);

            // Analyze THEN block (simulating temporary scope entry)
            if let Err(e) = analyze_block(then_block, &mut ScopeManager{ parent_scope: global_scope.clone() }, budget) {
                 return Err(format!("Error in IF block: {}", e));
            }
            
            // If ELSE exists, analyze it too
            if let Some(el) = else_block {
                println!("  - Analyzing optional ELSE block.");
                 if let Err(e) = analyze_block(el, &mut ScopeManager{ parent_scope: global_scope.clone() }, budget) {
                     return Err(format!("Error in ELSE block: {}", e));
                 }
            }
            Ok(())
        }
        
        // 4. Generic Blocks (e.g., Function body, 'on error' context)
        Statement::BlockStatement { statements } => {
             if let Err(e) = analyze_block(Box::new(BlockStatement{statements: statements.clone()}), &mut ScopeManager{ parent_scope: global_scope.clone() }, budget) {
                 return Err(format!("Error in block statement: {}", e));
            }
            Ok(())
        }

    }
}


/// Analyzes a block of code by iterating through its statements and updating the scope/budget.
fn analyze_block(block: &Box<BlockStatement>, scope: &mut ScopeManager, budget: u64) -> Result<(), String> {
    // The new local scope tracks variables defined *only* within this block.
    let mut local_scope = ScopeManager{ parent_scope: scope.scope_stack.clone() };

    for stmt in &block.statements {
        match analyze_statement(stmt, &mut local_scope.scope, budget) {
            Ok(_) => {}, 
            Err(e) => return Err(format!("Semantic Error processing statement: {}", e)), // Propagate error upwards immediately
        }
    }
    Ok(())
}

/// Analyzes an expression and returns the inferred/expected type of that expression.
fn analyze_expression(expr: &Box<Expression>, scope: &mut HashMap<String, &'static str>) -> String {
     match expr.as_ref() {
         Expression::LiteralInt(_) => "Int".to_string(),
         Expression::LiteralFloat(_) => "Float".to_string(),
         Expression::VariableReference(name) => scope.get(name).cloned().unwrap_or("Unknown").to_string(),
         // BinaryOp, Call, StructConstruction require deeper type resolution logic here...
         _ => { 
             if expr.is_struct() { "Struct".to_string() } else { "AnyType".to_string() }
         }
     }
}


// Helper to check if the expression is a struct construction (for clearer logging)
fn is_struct(expr: &Expression) -> bool {
    match expr {
        Expression::StructConstruction { .. } => true,
        _ => false,
    }
}