use std::collections::HashSet;

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};

pub fn transpile_program_to_c(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("#include <stdio.h>\n\n");
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");

    for stmt in &program.statements {
        if let Statement::FunctionDef { .. } = stmt {
            emit_function(stmt, &mut out);
            out.push('\n');
        }
    }

    out.push_str("int main(void) {\n");
    let mut declared = HashSet::new();
    for stmt in &program.statements {
        if !matches!(stmt, Statement::FunctionDef { .. }) {
            emit_statement(stmt, &mut out, 1, &mut declared);
        }
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    out
}

fn emit_function(stmt: &Statement, out: &mut String) {
    if let Statement::FunctionDef {
        name,
        params,
        body,
        returns,
        ..
    } = stmt
    {
        out.push_str(map_skadi_type_to_c(returns.as_deref()));
        out.push(' ');
        out.push_str(name);
        out.push('(');
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(map_skadi_type_to_c(p.param_type.as_deref()));
            out.push(' ');
            out.push_str(&p.name);
        }
        out.push_str(") {\n");
        let mut declared: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
        emit_block(body, out, 1, &mut declared);
        out.push_str("    return 0;\n");
        out.push_str("}\n");
    }
}

fn emit_block(block: &BlockStatement, out: &mut String, indent: usize, declared: &mut HashSet<String>) {
    for stmt in &block.statements {
        emit_statement(stmt, out, indent, declared);
    }
}

fn emit_statement(stmt: &Statement, out: &mut String, indent: usize, declared: &mut HashSet<String>) {
    let pad = "    ".repeat(indent);
    match stmt {
        Statement::Assignment { target, value } => {
            let expr = emit_expr(value);
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" = ");
            out.push_str(&expr);
            out.push_str(";\n");
        }
        Statement::IfStatement { condition, then_block, else_block } => {
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(&emit_expr(condition));
            out.push_str(") {\n");
            let mut then_decl = declared.clone();
            emit_block(then_block, out, indent + 1, &mut then_decl);
            out.push_str(&pad);
            out.push_str("}");
            if let Some(else_block) = else_block {
                out.push_str(" else {\n");
                let mut else_decl = declared.clone();
                emit_block(else_block, out, indent + 1, &mut else_decl);
                out.push_str(&pad);
                out.push_str("}");
            }
            out.push('\n');
        }
        Statement::WhileLoop { condition, body } => {
            out.push_str(&pad);
            out.push_str("while (");
            out.push_str(&emit_expr(condition));
            out.push_str(") {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::LoopStatement { body } => {
            out.push_str(&pad);
            out.push_str("while (1) {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ForLoop { initialization, condition, body, .. } => {
            // Transitional lowering for `for item in collection`.
            if let (Some(init), Some(coll)) = (initialization, condition) {
                let var_name = match init.as_ref() {
                    Expression::VariableReference(v) => v.clone(),
                    _ => "item".to_string(),
                };
                out.push_str(&pad);
                out.push_str("/* TODO(v1): lower Skadi List iteration semantics */\n");
                out.push_str(&pad);
                out.push_str("for (int __i = 0; __i < 1; ++__i) {\n");
                out.push_str(&"    ".repeat(indent + 1));
                out.push_str("int ");
                out.push_str(&var_name);
                out.push_str(" = ");
                out.push_str(&emit_expr(coll));
                out.push_str(";\n");
                let mut inner = declared.clone();
                inner.insert(var_name);
                emit_block(body, out, indent + 1, &mut inner);
                out.push_str(&pad);
                out.push_str("}\n");
            } else {
                out.push_str(&pad);
                out.push_str("/* TODO(v1): unsupported for-loop form */\n");
            }
        }
        Statement::FunctionDef { .. } => {}
        Statement::LabelDecl { name, .. } => {
            out.push_str(&pad);
            out.push_str("/* label ");
            out.push_str(name);
            out.push_str(" */\n");
        }
        Statement::StructDecl { name } => {
            out.push_str(&pad);
            out.push_str("/* struct ");
            out.push_str(name);
            out.push_str(" TODO(v1): C struct lowering */\n");
        }
        Statement::OnBlock { trigger } => {
            out.push_str(&pad);
            out.push_str("/* on ");
            out.push_str(trigger);
            out.push_str(" TODO(v1): runtime binding */\n");
        }
        Statement::DangerAssignOnError {
            target,
            call_name,
            args,
            on_error,
        } => {
            out.push_str(&pad);
            out.push_str("/* TODO(v1): danger call lowering */\n");
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(call_name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_expr(a));
            }
            if !args.is_empty() {
                out.push_str(", ");
            }
            out.push('&');
            out.push_str(target);
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::DangerCallOnError {
            call_name,
            args,
            on_error,
        } => {
            out.push_str(&pad);
            out.push_str("/* TODO(v1): danger call lowering */\n");
            out.push_str(&pad);
            out.push_str("if (");
            out.push_str(call_name);
            out.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&emit_expr(a));
            }
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ReturnStatement { value } => {
            out.push_str(&pad);
            out.push_str("return");
            if let Some(expr) = value {
                out.push(' ');
                out.push_str(&emit_expr(expr));
            }
            out.push_str(";\n");
        }
        Statement::WhenBlock { .. } => {
            out.push_str(&pad);
            out.push_str("/* TODO(v1): when lowering */\n");
        }
        Statement::VarDecl { name, value, declared_type, .. } => {
            out.push_str(&pad);
            out.push_str(map_skadi_type_to_c(declared_type.as_deref()));
            out.push(' ');
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&emit_expr(value));
            out.push_str(";\n");
            declared.insert(name.clone());
        }
        Statement::BlockStatement { statements } | Statement::OnErrorBlock { statements } => {
            let mut inner = declared.clone();
            for s in statements {
                emit_statement(s, out, indent, &mut inner);
            }
        }
    }
}

fn map_skadi_type_to_c(skadi_type: Option<&str>) -> &'static str {
    match skadi_type.unwrap_or("Int") {
        "Int" | "i64" => "int64_t",
        "Float" | "f64" => "double",
        "bool" => "bool",
        _ => "int64_t",
    }
}

fn emit_expr(expr: &Expression) -> String {
    match expr {
        Expression::LiteralInt(v) => v.to_string(),
        Expression::LiteralFloat(v) => v.to_string(),
        Expression::VariableReference(name) => name.clone(),
        Expression::BinaryOp { op, left, right } => {
            let l = emit_expr(left);
            if op == "neg" {
                return format!("(-{})", l);
            }
            if op == "not" {
                return format!("(!{})", l);
            }
            if let Some(r) = right {
                let rr = emit_expr(r);
                let c_op = match op.as_str() {
                    "and" => "&&",
                    "or" => "||",
                    "xor" => "^",
                    "div" => "/",
                    "mod" => "%",
                    other => other,
                };
                format!("({} {} {})", l, c_op, rr)
            } else {
                format!("({})", l)
            }
        }
        Expression::StructConstruction { .. } => "0 /* TODO(v1): struct literal */".to_string(),
    }
}
