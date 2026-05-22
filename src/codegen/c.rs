use std::collections::HashMap;

use crate::ast_nodes::{BlockStatement, Expression, Program, Statement};

struct FunctionContext {
    is_danger: bool,
    return_type: Option<String>,
}

const LIST_TYPE_MAP: [(&str, &str, &str); 11] = [
    ("i8", "int8_t", "i8"),
    ("i16", "int16_t", "i16"),
    ("i32", "int32_t", "i32"),
    ("i64", "int64_t", "i64"),
    ("u8", "uint8_t", "u8"),
    ("u16", "uint16_t", "u16"),
    ("u32", "uint32_t", "u32"),
    ("u64", "uint64_t", "u64"),
    ("f32", "float", "f32"),
    ("f64", "double", "f64"),
    ("bool", "bool", "bool"),
];

fn list_elem_from_decl(t: &str) -> Option<&str> {
    t.strip_suffix(" List").map(str::trim)
}

fn list_meta(elem: &str) -> Option<(&'static str, &'static str)> {
    LIST_TYPE_MAP
        .iter()
        .find(|(name, _, _)| *name == elem)
        .map(|(_, c_ty, suffix)| (*c_ty, *suffix))
}

fn emit_list_runtime(out: &mut String) {
    for (_, c_ty, suffix) in LIST_TYPE_MAP {
        out.push_str(&format!(
            "typedef struct {{\n    {} *data;\n    size_t len;\n    size_t cap;\n}} SkadiList_{};\n\n",
            c_ty, suffix
        ));
        out.push_str(&format!("static SkadiList_{} sk_list_{}_new(void) {{\n", suffix, suffix));
        out.push_str(&format!("    SkadiList_{} xs;\n", suffix));
        out.push_str("    xs.data = NULL;\n");
        out.push_str("    xs.len = 0;\n");
        out.push_str("    xs.cap = 0;\n");
        out.push_str("    return xs;\n");
        out.push_str("}\n\n");
        out.push_str(&format!(
            "static int sk_list_{}_push(SkadiList_{} *xs, {} v) {{\n",
            suffix, suffix, c_ty
        ));
        out.push_str("    if (xs->len == xs->cap) {\n");
        out.push_str("        size_t next = xs->cap == 0 ? 4 : xs->cap * 2;\n");
        out.push_str(&format!(
            "        {} *p = ({0}*)realloc(xs->data, next * sizeof({0}));\n",
            c_ty
        ));
        out.push_str("        if (!p) return 1;\n");
        out.push_str("        xs->data = p;\n");
        out.push_str("        xs->cap = next;\n");
        out.push_str("    }\n");
        out.push_str("    xs->data[xs->len++] = v;\n");
        out.push_str("    return 0;\n");
        out.push_str("}\n\n");
        out.push_str(&format!(
            "static int sk_list_{}_pop(SkadiList_{} *xs, {} *out) {{\n",
            suffix, suffix, c_ty
        ));
        out.push_str("    if (xs->len == 0) return 1;\n");
        out.push_str("    *out = xs->data[xs->len - 1];\n");
        out.push_str("    xs->len -= 1;\n");
        out.push_str("    return 0;\n");
        out.push_str("}\n\n");
    }
}

pub fn transpile_program_to_c(program: &Program) -> String {
    let mut out = String::new();
    let needs_list_runtime = program_uses_list_runtime(program);
    out.push_str("#include <stdio.h>\n\n");
    if needs_list_runtime {
        out.push_str("#include <stddef.h>\n");
        out.push_str("#include <stdlib.h>\n");
    }
    out.push_str("#include <stdint.h>\n");
    out.push_str("#include <stdbool.h>\n\n");
    if needs_list_runtime {
        emit_list_runtime(&mut out);
    }
    emit_error_code_enum(program, &mut out);

    for stmt in &program.statements {
        if let Statement::FunctionDef { .. } = stmt {
            emit_function(stmt, &mut out);
            out.push('\n');
        }
    }

    out.push_str("int main(void) {\n");
    let mut declared: HashMap<String, String> = HashMap::new();
    for stmt in &program.statements {
        if !matches!(stmt, Statement::FunctionDef { .. }) {
            emit_statement(stmt, &mut out, 1, &mut declared, None);
        }
    }
    out.push_str("    return 0;\n");
    out.push_str("}\n");

    out
}

fn program_uses_list_runtime(program: &Program) -> bool {
    fn block_has_for(block: &BlockStatement) -> bool {
        block.statements.iter().any(statement_needs_list)
    }
    fn statement_needs_list(stmt: &Statement) -> bool {
        match stmt {
            Statement::ForLoop { .. } => true,
            Statement::VarDecl { declared_type, .. } => declared_type
                .as_deref()
                .map(|t| t.ends_with(" List"))
                .unwrap_or(false),
            Statement::ListPush { .. } | Statement::ListPopOnError { .. } => true,
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
            Statement::WhileLoop { body, .. } | Statement::LoopStatement { body, .. } => block_has_for(body),
            Statement::DangerAssignOnError { on_error, .. }
            | Statement::DangerCallOnError { on_error, .. } => block_has_for(on_error),
            Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
                statements.iter().any(statement_needs_list)
            }
            _ => false,
        }
    }
    program.statements.iter().any(statement_needs_list)
}

fn emit_error_code_enum(program: &Program, out: &mut String) {
    for stmt in &program.statements {
        if let Statement::LabelDecl { name, variants, .. } = stmt {
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
        let mut declared: HashMap<String, String> = params
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    p.param_type.clone().unwrap_or_else(|| "Int".to_string()),
                )
            })
            .collect();
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
    declared: &mut HashMap<String, String>,
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
    declared: &mut HashMap<String, String>,
    fn_ctx: Option<&FunctionContext>,
) {
    let pad = "    ".repeat(indent);
    match stmt {
        Statement::Assignment { target, value, .. } => {
            let expr = emit_expr(value);
            out.push_str(&pad);
            out.push_str(target);
            out.push_str(" = ");
            out.push_str(&expr);
            out.push_str(";\n");
        }
        Statement::IfStatement { condition, then_block, else_block, .. } => {
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
        Statement::WhileLoop { condition, body, .. } => {
            out.push_str(&pad);
            out.push_str("while (");
            out.push_str(&emit_expr(condition));
            out.push_str(") {\n");
            let mut inner = declared.clone();
            emit_block(body, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::LoopStatement { body, .. } => {
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
                inner.insert(var_name, "Int".to_string());
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
        Statement::StructDecl { name, .. } => {
            out.push_str(&pad);
            out.push_str("/* struct ");
            out.push_str(name);
            out.push_str(" TODO(v1): C struct lowering */\n");
        }
        Statement::OnBlock { trigger, .. } => {
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
            ..
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
            ..
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
        Statement::ListPush { list_name, value, .. } => {
            let suffix = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .and_then(|elem| list_meta(elem).map(|(_, s)| s))
                .unwrap_or("i64");
            out.push_str(&pad);
            out.push_str("(void)sk_list_");
            out.push_str(suffix);
            out.push_str("_push(&");
            out.push_str(list_name);
            out.push_str(", ");
            out.push_str(&emit_expr(value));
            out.push_str(");\n");
        }
        Statement::ListPopOnError {
            target,
            list_name,
            on_error,
            ..
        } => {
            let suffix = declared
                .get(list_name)
                .and_then(|t| list_elem_from_decl(t))
                .and_then(|elem| list_meta(elem).map(|(_, s)| s))
                .unwrap_or("i64");
            out.push_str(&pad);
            out.push_str("if (sk_list_");
            out.push_str(suffix);
            out.push_str("_pop(&");
            out.push_str(list_name);
            out.push_str(", &");
            out.push_str(target);
            out.push_str(") != 0) {\n");
            let mut inner = declared.clone();
            emit_block(on_error, out, indent + 1, &mut inner, fn_ctx);
            out.push_str(&pad);
            out.push_str("}\n");
        }
        Statement::ReturnStatement { value, .. } => {
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
        Statement::ReturnError { code, .. } => {
            out.push_str(&pad);
            out.push_str("return ErrorCode_");
            out.push_str(code);
            out.push_str(";\n");
        }
        Statement::WhenBlock {
            when_expression,
            cases,
            else_block,
            ..
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
            if let Some(dt) = declared_type.as_deref() {
                if let Some(elem) = list_elem_from_decl(dt)
                    && let Some((_, suffix)) = list_meta(elem)
                {
                    out.push_str(&pad);
                    out.push_str("SkadiList_");
                    out.push_str(suffix);
                    out.push(' ');
                    out.push_str(name);
                    out.push_str(" = sk_list_");
                    out.push_str(suffix);
                    out.push_str("_new();\n");
                    if let Expression::ListLiteral(items) = value.as_ref() {
                        for item in items {
                            out.push_str(&pad);
                            out.push_str("(void)sk_list_");
                            out.push_str(suffix);
                            out.push_str("_push(&");
                            out.push_str(name);
                            out.push_str(", ");
                            out.push_str(&emit_expr(item));
                            out.push_str(");\n");
                        }
                    }
                    declared.insert(name.clone(), dt.to_string());
                    return;
                }
            }
            out.push_str(&pad);
            out.push_str(map_skadi_type_to_c(declared_type.as_deref()));
            out.push(' ');
            out.push_str(name);
            out.push_str(" = ");
            out.push_str(&emit_expr(value));
            out.push_str(";\n");
            declared.insert(name.clone(), declared_type.clone().unwrap_or_else(|| "Int".to_string()));
        }
        Statement::BlockStatement { statements, .. } | Statement::OnErrorBlock { statements, .. } => {
            let mut inner = declared.clone();
            for s in statements {
                emit_statement(s, out, indent, &mut inner, fn_ctx);
            }
        }
    }
}

fn map_skadi_type_to_c(skadi_type: Option<&str>) -> &'static str {
    match skadi_type.unwrap_or("Int") {
        "i8" => "int8_t",
        "i16" => "int16_t",
        "i32" => "int32_t",
        "Int" | "i64" => "int64_t",
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        "f32" => "float",
        "Float" | "f64" => "double",
        "bool" => "bool",
        "char" => "char",
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
        Expression::Index { base, index } => {
            format!("{}.data[{}]", emit_expr(base), emit_expr(index))
        }
        Expression::Call { name, args } => {
            if name == "len" && args.len() == 1 {
                return format!("((int64_t){}.len)", emit_expr(&args[0]));
            }
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
        Expression::ListLiteral(_) => "0 /* TODO(v1): list literal */".to_string(),
    }
}
