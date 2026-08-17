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
        Self {
            tokens,
            idx: start,
            end,
        }
    }

    fn parse(mut self) -> Result<Expression, String> {
        let expr = self.parse_bp(0)?;
        if self.idx < self.end {
            let tok = &self.tokens[self.idx];
            return Err(parse_err(
                "SC-PARSE-213",
                format!(
                    "unexpected trailing token in expression: {:?} ('{}')",
                    tok.kind, tok.lexeme
                ),
            ));
        }
        Ok(expr)
    }

    fn parse_bp(&mut self, min_bp: u8) -> Result<Expression, String> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.idx >= self.end {
                break;
            }

            let tok = &self.tokens[self.idx];
            if tok.kind == TokenKind::OpPunctuation && tok.lexeme == "[" {
                if 10 < min_bp {
                    break;
                }
                self.idx += 1; // skip '['
                let index_expr = self.parse_bp(0)?;
                if self.idx >= self.end
                    || self.tokens[self.idx].kind != TokenKind::OpPunctuation
                    || self.tokens[self.idx].lexeme != "]"
                {
                    return Err(parse_err(
                        "SC-PARSE-209",
                        "expected ']' to close index access.",
                    ));
                }
                self.idx += 1; // skip ']'
                lhs = Expression::Index {
                    base: Box::new(lhs),
                    index: Box::new(index_expr),
                };
                continue;
            }
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
            return Err(parse_err(
                "SC-PARSE-201",
                "expected expression, found end of input.",
            ));
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

        if tok.kind == TokenKind::Identifier && tok.lexeme == "stopping" {
            self.idx += 1;
            return Ok(Expression::Stopping);
        }

        if tok.kind == TokenKind::Identifier && tok.lexeme == "timed_out" {
            self.idx += 1;
            return Ok(Expression::TimedOut);
        }

        if tok.kind == TokenKind::Identifier && tok.lexeme == "wait" {
            self.idx += 1;
            if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                return Err(parse_err(
                    "SC-PARSE-214",
                    "wait expected task handle identifier.",
                ));
            }
            let task_name = self.tokens[self.idx].lexeme.clone();
            self.idx += 1;
            let timeout = if self.idx < self.end && self.tokens[self.idx].lexeme == "for" {
                self.idx += 1;
                if self.idx >= self.end {
                    return Err(parse_err(
                        "SC-PARSE-222",
                        "timed wait expected Duration expression after 'for'.",
                    ));
                }
                Some(Box::new(self.parse_bp(0)?))
            } else {
                None
            };
            return Ok(Expression::WaitTask { task_name, timeout });
        }

        if tok.kind == TokenKind::Identifier && tok.lexeme == "run" {
            self.idx += 1;
            if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                return Err(parse_err(
                    "SC-PARSE-215",
                    "run expected function call target.",
                ));
            }
            let call_name = self.tokens[self.idx].lexeme.clone();
            self.idx += 1;
            if self.idx >= self.end
                || self.tokens[self.idx].kind != TokenKind::OpPunctuation
                || self.tokens[self.idx].lexeme != "("
            {
                return Err(parse_err(
                    "SC-PARSE-216",
                    "run expected function call syntax: run worker(...).",
                ));
            }
            self.idx += 1;
            let mut args = Vec::new();
            if self.idx < self.end
                && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                && self.tokens[self.idx].lexeme == ")"
            {
                self.idx += 1;
                return Ok(Expression::RunTask { call_name, args });
            }
            loop {
                let arg = self.parse_bp(0)?;
                args.push(arg);
                if self.idx >= self.end {
                    return Err(parse_err("SC-PARSE-217", "expected ')' to close run call."));
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
                return Err(parse_err(
                    "SC-PARSE-218",
                    "expected ',' or ')' in run call.",
                ));
            }
            return Ok(Expression::RunTask { call_name, args });
        }

        if tok.kind == TokenKind::OpPunctuation && tok.lexeme == "(" {
            self.idx += 1;
            let expr = self.parse_bp(0)?;
            if self.idx >= self.end
                || self.tokens[self.idx].kind != TokenKind::OpPunctuation
                || self.tokens[self.idx].lexeme != ")"
            {
                return Err(parse_err(
                    "SC-PARSE-202",
                    "expected ')' to close grouped expression.",
                ));
            }
            self.idx += 1;
            return Ok(expr);
        }

        if tok.kind == TokenKind::OpPunctuation && tok.lexeme == "[" {
            self.idx += 1;
            let mut items = Vec::new();
            if self.idx < self.end
                && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                && self.tokens[self.idx].lexeme == "]"
            {
                self.idx += 1;
                return Ok(Expression::ListLiteral(items));
            }
            loop {
                let item = self.parse_bp(0)?;
                items.push(item);
                if self.idx >= self.end {
                    return Err(parse_err(
                        "SC-PARSE-207",
                        "expected ']' to close list literal.",
                    ));
                }
                if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == ","
                {
                    self.idx += 1;
                    continue;
                }
                if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == "]"
                {
                    self.idx += 1;
                    break;
                }
                return Err(parse_err(
                    "SC-PARSE-208",
                    "expected ',' or ']' in list literal.",
                ));
            }
            return Ok(Expression::ListLiteral(items));
        }

        if tok.kind == TokenKind::OpPunctuation && tok.lexeme == "{" {
            self.idx += 1;
            let mut fields = std::collections::HashMap::new();
            if self.idx < self.end
                && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                && self.tokens[self.idx].lexeme == "}"
            {
                self.idx += 1;
                return Ok(Expression::StructConstruction { fields });
            }
            loop {
                if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                    return Err(parse_err(
                        "SC-PARSE-210",
                        "struct literal expected field name.",
                    ));
                }
                let field_name = self.tokens[self.idx].lexeme.clone();
                self.idx += 1;

                let field_value = if self.idx < self.end
                    && self.tokens[self.idx].kind == TokenKind::OpAssignment
                    && self.tokens[self.idx].lexeme == "="
                {
                    self.idx += 1; // skip '='
                    self.parse_bp(0)?
                } else {
                    // Field punning sugar: `{value}` == `{value = value}`
                    Expression::VariableReference(field_name.clone())
                };
                fields.insert(field_name, Box::new(field_value));

                if self.idx >= self.end {
                    return Err(parse_err(
                        "SC-PARSE-211",
                        "expected '}' to close struct literal.",
                    ));
                }
                if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == ","
                {
                    self.idx += 1;
                    continue;
                }
                if self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == "}"
                {
                    self.idx += 1;
                    break;
                }
                return Err(parse_err(
                    "SC-PARSE-212",
                    "expected ',' or '}' in struct literal.",
                ));
            }
            return Ok(Expression::StructConstruction { fields });
        }

        if matches!(tok.kind, TokenKind::TypeInt | TokenKind::TypeFloat) && self.idx + 1 < self.end
        {
            let unit = &self.tokens[self.idx + 1];
            let is_adjacent =
                unit.line == tok.line && unit.col == tok.col + tok.lexeme.chars().count() as u32;
            if unit.kind == TokenKind::Identifier
                && is_adjacent
                && matches!(unit.lexeme.as_str(), "ns" | "us" | "ms" | "s" | "min" | "h")
            {
                if tok.kind == TokenKind::TypeFloat {
                    return Err(parse_err(
                        "SC-PARSE-216",
                        "duration literal requires an integer magnitude.",
                    ));
                }
                let magnitude = parse_integer_literal(&tok.lexeme).map_err(|_| {
                    parse_err("SC-PARSE-216", "invalid duration literal magnitude.")
                })?;
                let multiplier = match unit.lexeme.as_str() {
                    "ns" => 1_i64,
                    "us" => 1_000_i64,
                    "ms" => 1_000_000_i64,
                    "s" => 1_000_000_000_i64,
                    "min" => 60_000_000_000_i64,
                    "h" => 3_600_000_000_000_i64,
                    _ => unreachable!(),
                };
                let nanoseconds = magnitude.checked_mul(multiplier).ok_or_else(|| {
                    parse_err("SC-PARSE-216", "duration literal exceeds i64 nanoseconds.")
                })?;
                self.idx += 2;
                return Ok(Expression::LiteralDuration {
                    nanoseconds,
                    magnitude,
                    unit: unit.lexeme.clone(),
                });
            }
            if unit.kind == TokenKind::Identifier
                && is_adjacent
                && matches!(unit.lexeme.as_str(), "b" | "kb" | "mb" | "gb" | "tb")
            {
                let expression = parse_byte_size_literal(tok, unit)?;
                self.idx += 2;
                return Ok(expression);
            }
            if unit.kind == TokenKind::Identifier
                && is_adjacent
                && matches!(unit.lexeme.as_str(), "deg" | "rad")
            {
                let expression = parse_angle_literal(tok, unit)?;
                self.idx += 2;
                return Ok(expression);
            }
        }

        self.idx += 1;
        match tok.kind {
            TokenKind::TypeInt => {
                let parsed = parse_integer_literal(&tok.lexeme).map_err(|_| {
                    parse_err(
                        "SC-PARSE-225",
                        format!("invalid integer literal '{}'.", tok.lexeme),
                    )
                })?;
                Ok(Expression::LiteralInt(parsed))
            }
            TokenKind::TypeFloat => {
                let parsed = tok.lexeme.replace('_', "").parse::<f32>().map_err(|_| {
                    parse_err(
                        "SC-PARSE-226",
                        format!("invalid Float literal '{}'.", tok.lexeme),
                    )
                })?;
                if !parsed.is_finite() {
                    return Err(parse_err(
                        "SC-PARSE-226",
                        "Float literal must produce a finite f32 value.",
                    ));
                }
                Ok(Expression::LiteralFloat(parsed))
            }
            TokenKind::TypeBool => Ok(Expression::LiteralBool(tok.lexeme == "true")),
            TokenKind::KeywordDirect => {
                if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                    return Err(parse_err(
                        "SC-PARSE-222",
                        "'direct' call argument requires an identifier.",
                    ));
                }
                let name = self.tokens[self.idx].lexeme.clone();
                self.idx += 1;
                Ok(Expression::DirectBorrow(name))
            }
            TokenKind::KeywordView => {
                if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                    return Err(parse_err(
                        "SC-PARSE-223",
                        "'view' call argument requires an identifier.",
                    ));
                }
                let name = self.tokens[self.idx].lexeme.clone();
                self.idx += 1;
                Ok(Expression::ViewBorrow(name))
            }
            TokenKind::KeywordMove => {
                if self.idx >= self.end || self.tokens[self.idx].kind != TokenKind::Identifier {
                    return Err(parse_err(
                        "SC-PARSE-224",
                        "'move' requires an owning resource identifier.",
                    ));
                }
                let name = self.tokens[self.idx].lexeme.clone();
                self.idx += 1;
                Ok(Expression::Move(name))
            }
            TokenKind::Identifier | TokenKind::KeywordMy => {
                let mut call_name = tok.lexeme.clone();
                if self.idx + 1 < self.end
                    && self.tokens[self.idx].kind == TokenKind::OpPunctuation
                    && self.tokens[self.idx].lexeme == "."
                    && self.tokens[self.idx + 1].kind == TokenKind::Identifier
                {
                    let member = self.tokens[self.idx + 1].lexeme.clone();
                    if self.idx + 2 < self.end
                        && self.tokens[self.idx + 2].kind == TokenKind::OpPunctuation
                        && self.tokens[self.idx + 2].lexeme == "("
                    {
                        call_name = format!("{}.{}", tok.lexeme, member);
                        self.idx += 2;
                    } else {
                        self.idx += 2;
                        return Ok(Expression::MemberAccess {
                            base: tok.lexeme.clone(),
                            field: member,
                        });
                    }
                }
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
                            name: call_name,
                            args,
                        });
                    }
                    loop {
                        let arg = self.parse_bp(0)?;
                        args.push(arg);
                        if self.idx >= self.end {
                            return Err(parse_err(
                                "SC-PARSE-203",
                                "expected ')' to close function call.",
                            ));
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
                        return Err(parse_err(
                            "SC-PARSE-204",
                            "expected ',' or ')' in argument list.",
                        ));
                    }
                    Ok(Expression::Call {
                        name: call_name,
                        args,
                    })
                } else {
                    Ok(Expression::VariableReference(tok.lexeme.clone()))
                }
            }
            TokenKind::TypeString => Ok(Expression::LiteralString(tok.lexeme.clone())),
            TokenKind::TypeChar => parse_char_literal(tok),
            _ => Err(parse_err(
                "SC-PARSE-205",
                format!(
                    "unexpected token in expression: {:?} ('{}')",
                    tok.kind, tok.lexeme
                ),
            )),
        }
    }
}

fn parse_char_literal(token: &Token) -> Result<Expression, String> {
    let Some(inner) = token
        .lexeme
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    else {
        return Err(parse_err("SC-PARSE-221", "unterminated Char literal."));
    };
    let value = match inner {
        "\\n" => '\n',
        "\\r" => '\r',
        "\\t" => '\t',
        "\\0" => '\0',
        "\\\\" => '\\',
        "\\'" => '\'',
        _ => {
            let mut chars = inner.chars();
            let Some(value) = chars.next() else {
                return Err(parse_err("SC-PARSE-221", "Char literal cannot be empty."));
            };
            if chars.next().is_some() || inner.starts_with('\\') {
                return Err(parse_err(
                    "SC-PARSE-221",
                    "Char literal must contain exactly one character or a supported escape.",
                ));
            }
            value
        }
    };
    if !value.is_ascii() {
        return Err(parse_err(
            "SC-PARSE-221",
            "Char literal must be ASCII in the current C runtime.",
        ));
    }
    Ok(Expression::LiteralChar(value))
}

fn parse_integer_literal(lexeme: &str) -> Result<i64, std::num::ParseIntError> {
    let compact = lexeme.replace('_', "");
    let (digits, radix) = if let Some(value) = compact
        .strip_prefix("0b")
        .or_else(|| compact.strip_prefix("0B"))
    {
        (value, 2)
    } else if let Some(value) = compact
        .strip_prefix("0o")
        .or_else(|| compact.strip_prefix("0O"))
    {
        (value, 8)
    } else if let Some(value) = compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
    {
        (value, 16)
    } else {
        (compact.as_str(), 10)
    };
    i64::from_str_radix(digits, radix)
}

fn parse_byte_size_literal(
    magnitude_token: &Token,
    unit_token: &Token,
) -> Result<Expression, String> {
    if magnitude_token.kind == TokenKind::TypeFloat {
        return Err(parse_err(
            "SC-PARSE-219",
            "ByteSize literal requires an integer magnitude.",
        ));
    }
    let magnitude = magnitude_token.lexeme.as_str();
    let magnitude = parse_integer_literal(magnitude)
        .map_err(|_| parse_err("SC-PARSE-219", "invalid ByteSize literal magnitude."))?;
    let multiplier = match unit_token.lexeme.as_str() {
        "b" => 1_i64,
        "kb" => 1024_i64,
        "mb" => 1024_i64 * 1024,
        "gb" => 1024_i64 * 1024 * 1024,
        "tb" => 1024_i64 * 1024 * 1024 * 1024,
        _ => {
            return Err(parse_err(
                "SC-PARSE-219",
                "unsupported ByteSize literal unit.",
            ));
        }
    };
    let bytes = magnitude
        .checked_mul(multiplier)
        .ok_or_else(|| parse_err("SC-PARSE-219", "ByteSize literal exceeds i64 bytes."))?;
    Ok(Expression::LiteralByteSize {
        bytes,
        magnitude,
        unit: unit_token.lexeme.clone(),
    })
}

fn parse_angle_literal(magnitude_token: &Token, unit_token: &Token) -> Result<Expression, String> {
    let magnitude = magnitude_token
        .lexeme
        .replace('_', "")
        .parse::<f32>()
        .map_err(|_| parse_err("SC-PARSE-220", "invalid Angle literal magnitude."))?;
    let radians = match unit_token.lexeme.as_str() {
        "deg" => magnitude * std::f32::consts::PI / 180.0,
        "rad" => magnitude,
        _ => {
            return Err(parse_err("SC-PARSE-220", "unsupported Angle literal unit."));
        }
    };
    if !radians.is_finite() {
        return Err(parse_err(
            "SC-PARSE-220",
            "Angle literal must produce a finite f32 value.",
        ));
    }
    Ok(Expression::LiteralAngle {
        radians,
        magnitude: magnitude_token.lexeme.clone(),
        unit: unit_token.lexeme.clone(),
    })
}

pub(super) fn parse_memory_size_expression(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<Expression, String> {
    if end == start + 2
        && tokens[start].kind == TokenKind::TypeInt
        && tokens[start + 1].kind == TokenKind::Identifier
        && matches!(
            tokens[start + 1].lexeme.as_str(),
            "b" | "kb" | "mb" | "gb" | "tb"
        )
    {
        return parse_byte_size_literal(&tokens[start], &tokens[start + 1]);
    }
    parse_expression_range(tokens, start, end)
}

fn is_infix_operator(tok: &Token) -> bool {
    matches!(
        tok.kind,
        TokenKind::OpArithmetic | TokenKind::OpComparison | TokenKind::OpLogical
    )
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

pub fn parse_expression_range(
    tokens: &[Token],
    start: usize,
    end: usize,
) -> Result<Expression, String> {
    if start >= end {
        return Err(parse_err(
            "SC-PARSE-206",
            "expected expression, found empty range.",
        ));
    }
    ExprParser::new(tokens, start, end).parse()
}
