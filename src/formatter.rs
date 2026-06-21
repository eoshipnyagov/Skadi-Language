use crate::ast_nodes::{
    BlockStatement, Expression, ForLoopStyle, FunctionParam, Program, Statement, StructMethod,
};
use crate::lexer::lex;
use crate::parser::parse_program;

const INDENT: &str = "    ";

pub fn format_source(source: &str) -> Result<String, String> {
    let tokens = lex(source).map_err(|err| err.to_string())?;
    let program = parse_program(&tokens)?;
    format_program(&program)
}

pub fn format_program(program: &Program) -> Result<String, String> {
    let mut formatter = Formatter::new();
    formatter.render_program(program)?;
    Ok(format!("{}\n", formatter.out.trim_end()))
}

struct Formatter {
    out: String,
}

impl Formatter {
    fn new() -> Self {
        Self { out: String::new() }
    }

    fn render_program(&mut self, program: &Program) -> Result<(), String> {
        self.render_statements(&program.statements, 0, true)
    }

    fn render_statements(
        &mut self,
        statements: &[Statement],
        indent: usize,
        top_level: bool,
    ) -> Result<(), String> {
        for (index, statement) in statements.iter().enumerate() {
            self.render_statement(statement, indent)?;
            if index + 1 < statements.len() {
                if top_level {
                    self.out.push_str("\n\n");
                } else {
                    self.out.push('\n');
                }
            }
        }
        Ok(())
    }

    fn render_statement(&mut self, statement: &Statement, indent: usize) -> Result<(), String> {
        match statement {
            Statement::VarDecl {
                name,
                value,
                is_fixed,
                declared_type,
                ..
            } => {
                if *is_fixed {
                    return Err("format does not support 'fixed' declarations yet.".to_string());
                }
                self.write_indent(indent);
                self.out.push_str("new ");
                if let Some(declared_type) = declared_type {
                    self.out.push_str(declared_type);
                    self.out.push(' ');
                }
                self.out.push_str(name);
                self.out.push_str(" = ");
                self.out.push_str(&self.render_expression(value, 0));
            }
            Statement::MemoryDecl {
                name,
                size_spec,
                on_error,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("Memory ");
                self.out.push_str(name);
                self.out.push_str(" = memory(");
                self.out.push_str(size_spec.trim());
                self.out.push(')');
                if let Some(on_error) = on_error {
                    self.out.push_str(" on error ");
                    self.render_block(on_error, indent)?;
                }
            }
            Statement::Assignment { target, value, .. } => {
                self.write_indent(indent);
                self.out.push_str(target);
                self.out.push_str(" = ");
                self.out.push_str(&self.render_expression(value, 0));
            }
            Statement::IncDec {
                target,
                is_increment,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str(target);
                self.out.push_str(if *is_increment { "++" } else { "--" });
            }
            Statement::FieldAssignment {
                object,
                field,
                value,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str(object);
                self.out.push('.');
                self.out.push_str(field);
                self.out.push_str(" = ");
                self.out.push_str(&self.render_expression(value, 0));
            }
            Statement::FunctionDef {
                name,
                params,
                body,
                returns,
                is_danger,
                ..
            } => {
                self.write_indent(indent);
                if *is_danger {
                    self.out.push_str("danger ");
                }
                self.out.push_str("fn ");
                self.out.push_str(name);
                self.out.push('(');
                self.out.push_str(&self.render_params(params));
                self.out.push(')');
                if let Some(returns) = returns {
                    self.out.push(' ');
                    self.out.push_str(returns);
                }
                self.out.push(' ');
                self.render_block(body, indent)?;
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("if ");
                self.out.push_str(&self.render_expression(condition, 0));
                self.out.push(' ');
                self.render_block(then_block, indent)?;
                if let Some(else_block) = else_block {
                    if else_block.statements.len() == 1
                        && matches!(else_block.statements[0], Statement::IfStatement { .. })
                    {
                        self.out.push_str(" else ");
                        if let Statement::IfStatement { .. } = &else_block.statements[0] {
                            self.render_statement(&else_block.statements[0], 0)?;
                        }
                    } else {
                        self.out.push_str(" else ");
                        self.render_block(else_block, indent)?;
                    }
                }
            }
            Statement::ForLoop {
                initialization,
                condition,
                legacy_parts,
                style,
                body,
                ..
            } => {
                self.write_indent(indent);
                match style {
                    ForLoopStyle::ForIn => {
                        let Some(loop_var) = initialization else {
                            return Err("format expected for-in loop variable.".to_string());
                        };
                        let Some(collection) = condition else {
                            return Err("format expected for-in collection.".to_string());
                        };
                        self.out.push_str("for ");
                        self.out.push_str(&self.render_expression(loop_var, 0));
                        self.out.push_str(" in ");
                        self.out.push_str(&self.render_expression(collection, 0));
                    }
                    ForLoopStyle::IterateAs => {
                        let Some(loop_var) = initialization else {
                            return Err("format expected iterate variable.".to_string());
                        };
                        let Some(collection) = condition else {
                            return Err("format expected iterate collection.".to_string());
                        };
                        self.out.push_str("iterate ");
                        self.out.push_str(&self.render_expression(collection, 0));
                        self.out.push_str(" as ");
                        self.out.push_str(&self.render_expression(loop_var, 0));
                    }
                    ForLoopStyle::LegacyCStyle => {
                        let Some(parts) = legacy_parts else {
                            return Err("format expected legacy for-loop clauses.".to_string());
                        };
                        self.out.push_str("for (");
                        self.out.push_str(parts.initialization.trim());
                        self.out.push_str("; ");
                        self.out.push_str(parts.condition.trim());
                        self.out.push_str("; ");
                        self.out.push_str(parts.update.trim());
                        self.out.push(')');
                    }
                }
                self.out.push(' ');
                self.render_block(body, indent)?;
            }
            Statement::WhenBlock {
                when_expression,
                cases,
                else_block,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("when ");
                self.out
                    .push_str(&self.render_expression(when_expression, 0));
                self.out.push_str(" {\n");
                for (case_index, (expressions, block)) in cases.iter().enumerate() {
                    self.write_indent(indent + 1);
                    self.out.push_str("is ");
                    let rendered = expressions
                        .iter()
                        .map(|expr| self.render_expression(expr, 0))
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.out.push_str(&rendered);
                    self.out.push(' ');
                    self.render_block(block, indent + 1)?;
                    self.out.push('\n');
                    if case_index + 1 < cases.len() || else_block.is_some() {
                        self.out.push('\n');
                    }
                }
                if let Some(else_block) = else_block {
                    self.write_indent(indent + 1);
                    self.out.push_str("else ");
                    self.render_block(else_block, indent + 1)?;
                    self.out.push('\n');
                }
                self.write_indent(indent);
                self.out.push('}');
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                self.write_indent(indent);
                self.out.push_str("while ");
                self.out.push_str(&self.render_expression(condition, 0));
                self.out.push(' ');
                self.render_block(body, indent)?;
            }
            Statement::LoopStatement { body, .. } => {
                self.write_indent(indent);
                self.out.push_str("loop ");
                self.render_block(body, indent)?;
            }
            Statement::BreakStatement { .. } => {
                self.write_indent(indent);
                self.out.push_str("break");
            }
            Statement::ContinueStatement { .. } => {
                self.write_indent(indent);
                self.out.push_str("continue");
            }
            Statement::PassStatement { .. } => {
                self.write_indent(indent);
                self.out.push_str("pass");
            }
            Statement::LabelDecl { name, variants, .. } => {
                self.write_indent(indent);
                self.out.push_str("label ");
                self.out.push_str(name);
                self.out.push_str(" {\n");
                for (index, variant) in variants.iter().enumerate() {
                    self.write_indent(indent + 1);
                    self.out.push_str(variant);
                    if index + 1 < variants.len() {
                        self.out.push('\n');
                    }
                }
                self.out.push('\n');
                self.write_indent(indent);
                self.out.push('}');
            }
            Statement::StructDecl {
                name,
                fields,
                methods,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("struct ");
                self.out.push_str(name);
                self.out.push_str(" {\n");
                for (index, field) in fields.iter().enumerate() {
                    self.write_indent(indent + 1);
                    self.out.push_str(&field.field_type);
                    self.out.push(' ');
                    self.out.push_str(&field.name);
                    if index + 1 < fields.len() || !methods.is_empty() {
                        self.out.push('\n');
                    }
                }
                if !fields.is_empty() && !methods.is_empty() {
                    self.out.push('\n');
                }
                for (index, method) in methods.iter().enumerate() {
                    self.render_struct_method(method, indent + 1)?;
                    if index + 1 < methods.len() {
                        self.out.push_str("\n\n");
                    } else {
                        self.out.push('\n');
                    }
                }
                self.write_indent(indent);
                self.out.push('}');
            }
            Statement::OnBlock {
                trigger,
                target,
                body,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("on ");
                self.out.push_str(trigger);
                if let Some(target) = target
                    && !target.trim().is_empty()
                {
                    self.out.push(' ');
                    self.out.push_str(target.trim());
                }
                self.out.push(' ');
                self.render_block(body, indent)?;
            }
            Statement::DangerAssignOnError {
                target,
                call_name,
                args,
                on_error,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str(target);
                self.out.push_str(" = ");
                self.out.push_str(call_name);
                self.out.push('(');
                self.out.push_str(&self.render_arg_list(args));
                self.out.push(')');
                self.out.push_str(" on error ");
                self.render_block(on_error, indent)?;
            }
            Statement::DangerCallOnError {
                call_name,
                args,
                on_error,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str(call_name);
                self.out.push('(');
                self.out.push_str(&self.render_arg_list(args));
                self.out.push(')');
                self.out.push_str(" on error ");
                self.render_block(on_error, indent)?;
            }
            Statement::ListPush {
                list_name, value, ..
            } => {
                self.write_indent(indent);
                self.out.push_str(list_name);
                self.out.push_str(".push(");
                self.out.push_str(&self.render_expression(value, 0));
                self.out.push(')');
            }
            Statement::ListPopOnError {
                target,
                list_name,
                on_error,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str(target);
                self.out.push_str(" = ");
                self.out.push_str(list_name);
                self.out.push_str(".pop() on error ");
                self.render_block(on_error, indent)?;
            }
            Statement::PlaceIn {
                memory_name,
                on_error,
                body,
                ..
            } => {
                self.write_indent(indent);
                self.out.push_str("place in ");
                self.out.push_str(memory_name);
                self.out.push(' ');
                self.render_block(body, indent)?;
                if let Some(on_error) = on_error {
                    self.out.push_str(" on error ");
                    self.render_block(on_error, indent)?;
                }
            }
            Statement::MemoryClear { memory_name, .. } => {
                self.write_indent(indent);
                self.out.push_str(memory_name);
                self.out.push_str(".clear()");
            }
            Statement::StopTask { task_name, .. } => {
                self.write_indent(indent);
                self.out.push_str("stop ");
                self.out.push_str(task_name);
            }
            Statement::ReturnError { code, .. } => {
                self.write_indent(indent);
                self.out.push_str("return error ");
                self.out.push_str(code);
            }
            Statement::ReturnStatement { value, .. } => {
                self.write_indent(indent);
                self.out.push_str("return");
                if let Some(value) = value {
                    self.out.push(' ');
                    self.out.push_str(&self.render_expression(value, 0));
                }
            }
            Statement::ExpressionStatement { expr, .. } => {
                self.write_indent(indent);
                self.out.push_str(&self.render_expression(expr, 0));
            }
            Statement::BlockStatement { statements, .. } => {
                self.write_indent(indent);
                self.out.push_str("{\n");
                self.render_statements(statements, indent + 1, false)?;
                self.out.push('\n');
                self.write_indent(indent);
                self.out.push('}');
            }
            Statement::OnErrorBlock { statements, .. } => {
                self.write_indent(indent);
                self.out.push_str("on error {\n");
                self.render_statements(statements, indent + 1, false)?;
                self.out.push('\n');
                self.write_indent(indent);
                self.out.push('}');
            }
        }
        Ok(())
    }

    fn render_struct_method(&mut self, method: &StructMethod, indent: usize) -> Result<(), String> {
        self.write_indent(indent);
        if method.is_danger {
            self.out.push_str("danger ");
        }
        self.out.push_str("fn ");
        self.out.push_str(&method.name);
        self.out.push('(');
        self.out.push_str(&self.render_params(&method.params));
        self.out.push(')');
        if let Some(returns) = &method.returns {
            self.out.push(' ');
            self.out.push_str(returns);
        }
        self.out.push(' ');
        self.render_block(&method.body, indent)?;
        Ok(())
    }

    fn render_block(&mut self, block: &BlockStatement, indent: usize) -> Result<(), String> {
        self.out.push_str("{\n");
        self.render_statements(&block.statements, indent + 1, false)?;
        self.out.push('\n');
        self.write_indent(indent);
        self.out.push('}');
        Ok(())
    }

    fn render_params(&self, params: &[FunctionParam]) -> String {
        params
            .iter()
            .map(|param| {
                if let Some(param_type) = &param.param_type {
                    format!("{param_type} {}", param.name)
                } else {
                    param.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_arg_list(&self, args: &[Expression]) -> String {
        args.iter()
            .map(|arg| self.render_expression(arg, 0))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn render_expression(&self, expr: &Expression, parent_precedence: u8) -> String {
        match expr {
            Expression::LiteralInt(value) => value.to_string(),
            Expression::LiteralFloat(value) => self.render_float(*value),
            Expression::LiteralBool(value) => value.to_string(),
            Expression::LiteralString(value) => value.clone(),
            Expression::ListLiteral(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| self.render_expression(item, 0))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Expression::Index { base, index } => {
                let base_text = self.render_expression(base, 10);
                format!("{}[{}]", base_text, self.render_expression(index, 0))
            }
            Expression::VariableReference(name) => name.clone(),
            Expression::MemberAccess { base, field } => format!("{base}.{field}"),
            Expression::Call { name, args } => {
                format!("{name}({})", self.render_arg_list(args))
            }
            Expression::RunTask { call_name, args } => {
                format!("run {call_name}({})", self.render_arg_list(args))
            }
            Expression::WaitTask { task_name } => format!("wait {task_name}"),
            Expression::Stopping => "stopping".to_string(),
            Expression::BinaryOp { op, left, right } if op == "neg" => {
                let value = right
                    .as_ref()
                    .map(|expr| self.render_expression(expr, 9))
                    .unwrap_or_else(|| self.render_expression(left, 9));
                self.wrap_if_needed(format!("-{value}"), 9, parent_precedence)
            }
            Expression::BinaryOp { op, left, right } if op == "not" => {
                let value = right
                    .as_ref()
                    .map(|expr| self.render_expression(expr, 9))
                    .unwrap_or_else(|| self.render_expression(left, 9));
                self.wrap_if_needed(format!("not {value}"), 9, parent_precedence)
            }
            Expression::BinaryOp { op, left, right } => {
                let precedence = self.infix_precedence(op);
                let left_text = self.render_expression(left, precedence);
                let right_text = right
                    .as_ref()
                    .map(|expr| self.render_expression(expr, precedence + 1))
                    .unwrap_or_default();
                self.wrap_if_needed(
                    format!("{left_text} {op} {right_text}"),
                    precedence,
                    parent_precedence,
                )
            }
            Expression::StructConstruction { fields } => {
                let mut entries = fields.iter().collect::<Vec<_>>();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                let rendered = entries
                    .into_iter()
                    .map(|(name, value)| format!("{name} = {}", self.render_expression(value, 0)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{rendered}}}")
            }
        }
    }

    fn render_float(&self, value: f32) -> String {
        let mut text = value.to_string();
        if !text.contains('.') && !text.contains('e') && !text.contains('E') {
            text.push_str(".0");
        }
        text
    }

    fn infix_precedence(&self, op: &str) -> u8 {
        match op {
            "or" | "xor" | "||" => 1,
            "and" | "&&" => 2,
            "==" | "!=" | "<" | ">" | "<=" | ">=" => 3,
            "+" | "-" => 4,
            "*" | "/" | "%" | "div" | "mod" => 6,
            "^" => 8,
            _ => 0,
        }
    }

    fn wrap_if_needed(&self, rendered: String, precedence: u8, parent_precedence: u8) -> String {
        if precedence < parent_precedence {
            format!("({rendered})")
        } else {
            rendered
        }
    }

    fn write_indent(&mut self, indent: usize) {
        for _ in 0..indent {
            self.out.push_str(INDENT);
        }
    }
}
