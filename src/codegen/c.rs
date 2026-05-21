use std::collections::HashSet;

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};

struct FunctionContext {
    is_danger: bool,
    return_type: Option<String>,
}

pub fn transpile_program_to_c(program: &Program) -> String {
    let mut out = String::new();
    let needs_list_runtime = program_uses_for_loop(program);
    out.push_str("#include <stdio.h>\n\n");
    if needs_list_runtime {
        out.push_str("#include <stddef.h>\n");
    }
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");
    if needs_list_runtime {
        out.push_str(
            "typedef struct {\n    int64_t *data;\n    size_t len;\n} SkadiListInt;\n\n",
        );
    }
    emit_error_code_enum(program, &mut out);

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
            emit_statement(stmt, &mut out, 1, &mut declared, None);
        }
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    out
}

fn program_uses_for_loop(program: &Program) -> bool {
    fn block_has_for(block: &BlockStatement) -> bool {
        block.statements.iter().any(statement_has_for)
    }
    fn statement_has_for(stmt: &Statement) -> bool {
        match stmt {
            Statement::ForLoop { .. } => true,
            Statement::FunctionDef { body, .. } => block_has_for(body),
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                block_has_for(then_block)
                    || else_block
                        .as_ref()
                        .map(|b| block_has_for(b))
                        .unwrap_or(false)
            }
            Statement::WhenBlock { cases, else_block, .. } => {
                cases.iter().any(|(_, b)| block_has_for(b))
                    || else_block
                        .as_ref()
                        .map(|b| block_has_for(b))
                        .unwrap_or(false)
            }
            Statement::WhileLoop { body, .. } | Statement::LoopStatement { body } => block_has_for(body),
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. } => block_has_for(on_error),
            Statement::BlockStatement { statements } | Statement::OnErrorBlock { statements } => {
                statements.iter().any(statement_has_for)
            }
            _ => false,
        }
    }
    program.statements.iter().any(statement_has_for)
}

fn emit_error_code_enum(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        if let Statement::LabelDecl { name, variants } = stmt {
            if name == "ErrorCode" && !variants.is_empty() {
                out.push_str("typedef enum ErrorCode {\n");
                for (i, v) in variants.iter().enumerate() {
                    if i == 0 {
                        out.push_str(&format!("    ErrorCode_{} = 0,\n", v));
                    } else {
                        out.push_str(&format!("    ErrorCode_{} = {},\n", v, i));
                    }
                }
                out.push_str("} ErrorCode;\n\n");
                break;
            }
        }
    }
}

fn emit_function(stmt: &Statement, out: &mut String) {
    if let Statement::FunctionDef {
        name,
        params,
        body,
        returns,
        is_danger,
        ..
    } = stmt
    {
        if *is_danger {
            out.push_str("int");
        } else {
            out.push_str(map_skadi_type_to_c(returns.as_deref()));
        }
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
        if *is_danger && let Some(ret_ty) = returns.as_deref() {
            if !params.is_empty() {
                out.push_str(", ");
            }
            out.push_str(map_skadi_type_to_c(Some(ret_ty)));
            out.push_str(" *out");
        }
        out.push_str(") {\n");
        let mut declared: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
        let fn_ctx = FunctionContext {
            is_danger: *is_danger,
            return_type: returns.clone(),
        };
        emit_block(body, out, 1, &mut declared, Some(&fn_ctx));
        out.push_str("    return 0;\n");
        out.push_str("}\n");
    }
}

fn emit_block(
    block: &BlockStatement,
    out: &mut String,
    indent: usize,
    declared: &mut HashSet<String>,
    fn_ctx: Option<&FunctionContext>,
) {
    for stmt in &block.statements {
        emit_statement(stmt, out, indent, declared, fn_ctx);
    }
}

fn emit_statement(
    stmt: &Statement,
    out: &mut String,
    indent: usize,
    declared: &mut HashSet<String>,
    fn_ctx: Option<&FunctionContext>,
) {
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
            emit_block(then_block, out, indent + 1, &mut then_decl, fn_ctx);
            out.push_str(&pad);
            out.push_str("}");
            if let Some(else_block) = else_block {
                out.push_str(" else {\n");
                let mut else_decl = declared.clone();
                emit_block(else_block, out, indent + 1, &mut else_decl, fn_ctx);
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
            emit_block(body, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::LoopStatement { body } => {
            out.push_str(&pad);
            out.push_str("while (1) {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ForLoop { initialization, condition, body, .. } => {
            if let (Some(init), Some(coll)) = (initialization, condition) {
                let var_name = match init.as_ref() {
                    Expression::VariableReference(v) => v.clone(),
                    _ => "item".to_string(),
                };
                let coll_expr = emit_expr(coll);
                out.push_str(&pad);
                out.push_str("for (size_t __i = 0; __i < ");
                out.push_str(&coll_expr);
                out.push_str(".len; ++__i) {\n");
                out.push_str(&"    ".repeat(indent + 1));
                out.push_str("int64_t ");
                out.push_str(&var_name);
                out.push_str(" = ");
                out.push_str(&coll_expr);
                out.push_str(".data[__i]");
                out.push_str(";\n");
                let mut inner = declared.clone();
                inner.insert(var_name);
                emit_block(body, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            } else {
                out.push_str(&pad);
                out.push_str("/* TODO(v1): unsupported for-loop form; expected 'for item in collection' */\n");
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
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
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
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ReturnStatement { value } => {
            if let Some(ctx) = fn_ctx {
                if ctx.is_danger {
                    match (ctx.return_type.is_some(), value) {
                        (true, Some(expr)) => {
                            out.push_str(&pad);
                            out.push_str("*out = ");
                            out.push_str(&emit_expr(expr));
                            out.push_str(";\n");
                            out.push_str(&pad);
                            out.push_str("return 0;\n");
                            return;
                        }
                        (true, None) => {
                            out.push_str(&pad);
                            out.push_str("return 1;\n");
                            return;
                        }
                        (false, Some(expr)) => {
                            out.push_str(&pad);
                            out.push_str("return ");
                            out.push_str(&emit_expr(expr));
                            out.push_str(";\n");
                            return;
                        }
                        (false, None) => {
                            out.push_str(&pad);
                            out.push_str("return 1;\n");
                            return;
                        }
                    }
                }
            }
            out.push_str(&pad);
            out.push_str("return");
            if let Some(expr) = value {
                out.push(' ');
                out.push_str(&emit_expr(expr));
            }
            out.push_str(";\n");
        }
        Statement::ReturnError { code } => {
            out.push_str(&pad);
            out.push_str("return ErrorCode_");
            out.push_str(code);
            out.push_str(";\n");
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
        } => {
            if cases.is_empty() {
                if let Some(else_block) = else_block {
                    emit_block(else_block, out, indent, declared, fn_ctx);
                }
                return;
            }
            let when_expr = emit_expr(when_expression);
            let when_tmp = format!("__when_tmp_{}", indent);
            out.push_str(&pad);
            out.push_str("int64_t ");
            out.push_str(&when_tmp);
            out.push_str(" = ");
            out.push_str(&when_expr);
            out.push_str(";\n");
            for (idx, (case_exprs, case_block)) in cases.iter().enumerate() {
                out.push_str(&pad);
                if idx == 0 {
                    out.push_str("if (");
                } else {
                    out.push_str("else if (");
                }
                if case_exprs.is_empty() {
                    out.push_str("0");
                } else {
                    for (j, expr) in case_exprs.iter().enumerate() {
                        if j > 0 {
                            out.push_str(" || ");
                        }
                        out.push('(');
                        out.push_str(&when_tmp);
                        out.push_str(" == ");
                        out.push_str(&emit_expr(expr));
                        out.push(')');
                    }
                }
                out.push_str(") {\n");
                let mut inner = declared.clone();
                emit_block(case_block, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            }
            if let Some(else_block) = else_block {
                out.push_str(&pad);
                out.push_str("else {\n");
                let mut inner = declared.clone();
                emit_block(else_block, out, indent + 1, &mut inner, fn_ctx);
                out.push_str(&pad);
                out.push_str("}\n");
            }
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
                emit_statement(s, out, indent, &mut inner, fn_ctx);
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
        Expression::LiteralBool(v) => {
            if *v { "true".to_string() } else { "false".to_string() }
        }
        Expression::VariableReference(name) => name.clone(),
        Expression::Call { name, args } => {
            let rendered: Vec<String> = args.iter().map(emit_expr).collect();
            format!("{}({})", name, rendered.join(", "))
        }
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
