use crate::ast_nodes::Expression;
use crate::common_types::{Token, TokenKind};

fn parse_err(code: &str, message: impl AsRef<str>) -> String {
    format!("[{}] {}", code, message.as_ref())
}

struct ExprParser<'a> {
    tokens: &'a [Token],
    idx: usize,
    end: usize,
}

impl<'a> ExprParser<'a> {
    fn new(tokens: &'a [Token], start: usize, end: usize) -> Self {
        Self { tokens, idx: start, end }
    }

    fn parse(mut self) -> Result<Expression, String> {
        let expr = self.parse_bp(0)?;
        Ok(expr)
    }

    fn parse_bp(&mut self, min_bp: u8) -> Result<Expression, String> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.idx >= self.end {
                break;
            }

            let tok = &self.tokens[self.idx];
            let op = if is_infix_operator(tok) {
                tok.lexeme.as_str()
            } else {
                break;
            };

            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }
            self.idx += 1;
            let rhs = self.parse_bp(r_bp)?;
            lhs = Expression::BinaryOp {
                op: op.to_string(),
                left: Box::new(lhs),
                right: Some(Box::new(rhs)),
            };
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expression, String> {
        if self.idx >= self.end {
            return Err(parse_err("SC-PARSE-201", "expected expression, found end of input."));
        }
        let tok = &self.tokens[self.idx];

        if tok.kind == TokenKind::OpArithmetic && tok.lexeme == "-" {
            self.idx += 1;
            let rhs = self.parse_bp(9)?;
            return Ok(Expression::BinaryOp {
                op: "neg".to_string(),
                left: Box::new(Expression::LiteralInt(0)),
                right: Some(Box::new(rhs)),
            });
        }

        if tok.kind == TokenKind::OpLogical && (tok.lexeme == "!" || tok.lexeme == "not") {
            self.idx += 1;
            let rhs = self.parse_bp(9)?;
            return Ok(Expression::BinaryOp {
                op: "not".to_string(),
                left: Box::new(Expression::LiteralInt(1)),
                right: Some(Box::new(rhs)),
            });
        }

        if tok.kind == TokenKind::OpPunctuation && tok.lexeme == "(" {
            self.idx += 1;
            let expr = self.parse_bp(0)?;
            if self.idx >= self.end
                || self.tokens[self.idx].kind != TokenKind::OpPunctuation
                || self.tokens[self.idx].lexeme != ")"
            {
                return Err(parse_err("SC-PARSE-202", "expected ')' to close grouped expression."));
            }
            self.idx += 1;
            return Ok(expr);
        }

        self.idx += 1;
        match tok.kind {
            TokenKind::TypeInt => {
                let parsed = tok.lexeme.parse::<i64>().unwrap_or(0);
                Ok(Expression::LiteralInt(parsed))
            }
            TokenKind::TypeFloat => {
                let parsed = tok.lexeme.parse::<f32>().unwrap_or(0.0);
                Ok(Expression::LiteralFloat(parsed))
            }
            TokenKind::TypeBool => Ok(Expression::LiteralBool(tok.lexeme == "true")),
            TokenKind::Identifier => {
                if self.idx < self.end
                    && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == "("
                {
                    self.idx += 1; // skip '('
                    let mut args = Vec::new();
                    if self.idx < self.end
                        && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                        && self.tokens[self.idx].lexeme == ")"
                    {
                        self.idx += 1;
                        return Ok(Expression::Call {
                            name: tok.lexeme.clone(),
                            args,
                        });
                    }
                    loop {
                        let arg = self.parse_bp(0)?;
                        args.push(arg);
                        if self.idx >= self.end {
                            return Err(parse_err("SC-PARSE-203", "expected ')' to close function call."));
                        }
                        if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                            && self.tokens[self.idx].lexeme == ","
                        {
                            self.idx += 1;
                            continue;
                        }
                        if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                            && self.tokens[self.idx].lexeme == ")"
                        {
                            self.idx += 1;
                            break;
                        }
                        return Err(parse_err("SC-PARSE-204", "expected ',' or ')' in argument list."));
                    }
                    Ok(Expression::Call {
                        name: tok.lexeme.clone(),
                        args,
                    })
                } else {
                    Ok(Expression::VariableReference(tok.lexeme.clone()))
                }
            }
            TokenKind::TypeString | TokenKind::TypeChar => {
                Ok(Expression::VariableReference(tok.lexeme.clone()))
            }
            _ => Err(parse_err(
                "SC-PARSE-205",
                format!("unexpected token in expression: {:?} ('{}')", tok.kind, tok.lexeme),
            )),
        }
    }
}

fn is_infix_operator(tok: &Token) -> bool {
    matches!(tok.kind, TokenKind::OpArithmetic | TokenKind::OpComparison | TokenKind::OpLogical)
}

fn infix_binding_power(op: &str) -> (u8, u8) {
    match op {
        "^" => (8, 8),
        "*" | "/" | "%" | "div" | "mod" => (6, 7),
        "+" | "-" => (4, 5),
        "==" | "!=" | "<" | ">" | "<=" | ">=" => (3, 4),
        "and" | "&&" => (2, 3),
        "or" | "xor" | "||" => (1, 2),
        _ => (0, 1),
    }
}

pub fn parse_expression_range(tokens: &[Token], start: usize, end: usize) -> Result<Expression, String> {
    if start >= end {
        return Err(parse_err("SC-PARSE-206", "expected expression, found empty range."));
    }
    ExprParser::new(tokens, start, end).parse()
}
