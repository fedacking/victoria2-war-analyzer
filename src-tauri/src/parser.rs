use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub key: String,
    pub value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Identifier(String),
    String(String),
    Integer(i64),
    Decimal(f64),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Statements(Vec<Statement>),
    Values(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    column: usize,
    message: String,
}

impl ParseError {
    fn new(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            column,
            message: message.into(),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Identifier(String),
    String(String),
    Integer(i64),
    Decimal(f64),
    Equals,
    LeftBrace,
    RightBrace,
    Eof,
}

pub fn parse_document(input: &str) -> Result<Document, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let statements = parser.parse_document()?;

    Ok(Document { statements })
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    Lexer::new(input).tokenize()
}

struct Lexer<'a> {
    input: &'a str,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            line: 1,
            column: 1,
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();

            let line = self.line;
            let column = self.column;

            let Some(current) = self.peek_char() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    column,
                });
                return Ok(tokens);
            };

            let kind = match current {
                '=' => {
                    self.bump_char();
                    TokenKind::Equals
                }
                '{' => {
                    self.bump_char();
                    TokenKind::LeftBrace
                }
                '}' => {
                    self.bump_char();
                    TokenKind::RightBrace
                }
                '"' => self.lex_string(line, column)?,
                _ => self.lex_word(line, column)?,
            };

            tokens.push(Token { kind, line, column });
        }
    }

    fn lex_string(&mut self, line: usize, column: usize) -> Result<TokenKind, ParseError> {
        self.bump_char();
        let start = self.position;

        while let Some(current) = self.peek_char() {
            if current == '"' {
                let value = self.input[start..self.position].to_string();
                self.bump_char();
                return Ok(TokenKind::String(value));
            }

            self.bump_char();
        }

        Err(ParseError::new(line, column, "unterminated string literal"))
    }

    fn lex_word(&mut self, line: usize, column: usize) -> Result<TokenKind, ParseError> {
        let start = self.position;

        while let Some(current) = self.peek_char() {
            if current.is_whitespace() || matches!(current, '=' | '{' | '}' | '"') {
                break;
            }

            self.bump_char();
        }

        let lexeme = &self.input[start..self.position];

        if is_integer_literal(lexeme) {
            let value = lexeme.parse::<i64>().map_err(|_| {
                ParseError::new(line, column, format!("invalid integer literal '{lexeme}'"))
            })?;
            return Ok(TokenKind::Integer(value));
        }

        if is_decimal_literal(lexeme) {
            let value = lexeme.parse::<f64>().map_err(|_| {
                ParseError::new(line, column, format!("invalid decimal literal '{lexeme}'"))
            })?;
            return Ok(TokenKind::Decimal(value));
        }

        if lexeme.chars().all(is_identifier_char) {
            return Ok(TokenKind::Identifier(lexeme.to_string()));
        }

        Err(ParseError::new(
            line,
            column,
            format!("unexpected token '{lexeme}'"),
        ))
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_char(), Some(current) if current.is_whitespace()) {
            self.bump_char();
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn bump_char(&mut self) -> Option<char> {
        let current = self.peek_char()?;
        self.position += current.len_utf8();

        if current == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(current)
    }
}

fn is_identifier_char(current: char) -> bool {
    current.is_ascii_alphanumeric() || matches!(current, '_' | '-' | '.' | ':')
}

fn is_integer_literal(lexeme: &str) -> bool {
    let digits = strip_number_sign(lexeme);
    !digits.is_empty() && digits.chars().all(|current| current.is_ascii_digit())
}

fn is_decimal_literal(lexeme: &str) -> bool {
    let digits = strip_number_sign(lexeme);
    let mut parts = digits.split('.');
    let Some(left) = parts.next() else {
        return false;
    };
    let Some(right) = parts.next() else {
        return false;
    };

    parts.next().is_none()
        && !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|current| current.is_ascii_digit())
        && right.chars().all(|current| current.is_ascii_digit())
}

fn strip_number_sign(lexeme: &str) -> &str {
    if let Some(stripped) = lexeme.strip_prefix('+') {
        stripped
    } else if let Some(stripped) = lexeme.strip_prefix('-') {
        stripped
    } else {
        lexeme
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

enum ScopeTerminator {
    Eof,
    RightBrace,
}

enum BlockMode {
    Statements,
    Values,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn parse_document(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.parse_statement_list(ScopeTerminator::Eof)
    }

    fn parse_statement_list(
        &mut self,
        terminator: ScopeTerminator,
    ) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();

        loop {
            match &self.peek().kind {
                TokenKind::Eof => match terminator {
                    ScopeTerminator::Eof => break,
                    ScopeTerminator::RightBrace => {
                        return Err(ParseError::new(
                            self.peek().line,
                            self.peek().column,
                            "expected '}' before end of input",
                        ));
                    }
                },
                TokenKind::RightBrace => match terminator {
                    ScopeTerminator::RightBrace => {
                        self.advance();
                        break;
                    }
                    ScopeTerminator::Eof => {
                        if self.only_trailing_right_braces_remain() {
                            self.consume_trailing_right_braces();
                            break;
                        }

                        return Err(ParseError::new(
                            self.peek().line,
                            self.peek().column,
                            "unexpected '}'",
                        ));
                    }
                },
                _ => {
                    let statement = self.parse_statement()?;
                    statements.push(statement);
                }
            }
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let key_token = self.advance().clone();
        let is_identifier_key = matches!(key_token.kind, TokenKind::Identifier(_));

        let key = match &key_token.kind {
            TokenKind::Identifier(key) => key.clone(),
            TokenKind::String(value) => value.clone(),
            TokenKind::Integer(value) => value.to_string(),
            TokenKind::Decimal(value) => value.to_string(),
            _ => {
                return Err(ParseError::new(
                    key_token.line,
                    key_token.column,
                    "expected identifier, string, or numeric key",
                ));
            }
        };

        if matches!(self.peek().kind, TokenKind::Equals) {
            self.advance();
            let value = self.parse_value()?;

            return Ok(Statement {
                key,
                value: Some(value),
            });
        }

        if is_identifier_key && self.is_bare_identifier_statement_boundary(&key_token) {
            return Ok(Statement { key, value: None });
        }

        let next_token = self.peek().clone();
        Err(ParseError::new(
            next_token.line,
            next_token.column,
            format!("expected '=' after key '{key}'"),
        ))
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let token = self.advance().clone();

        match token.kind {
            TokenKind::Identifier(identifier) => Ok(Value::Identifier(identifier)),
            TokenKind::String(value) => Ok(Value::String(value)),
            TokenKind::Integer(value) => Ok(Value::Integer(value)),
            TokenKind::Decimal(value) => Ok(Value::Decimal(value)),
            TokenKind::LeftBrace => Ok(Value::Block(self.parse_block()?)),
            TokenKind::RightBrace => Err(ParseError::new(
                token.line,
                token.column,
                "unexpected '}' while parsing value",
            )),
            TokenKind::Equals => Err(ParseError::new(
                token.line,
                token.column,
                "unexpected '=' while parsing value",
            )),
            TokenKind::Eof => Err(ParseError::new(
                token.line,
                token.column,
                "expected value before end of input",
            )),
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> &Token {
        let current = &self.tokens[self.position];

        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }

        current
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        if matches!(self.peek().kind, TokenKind::RightBrace) {
            self.advance();
            return Ok(Block::Statements(Vec::new()));
        }

        let mode = self.detect_block_mode()?;

        match mode {
            BlockMode::Statements => Ok(Block::Statements(
                self.parse_statement_list(ScopeTerminator::RightBrace)?,
            )),
            BlockMode::Values => Ok(Block::Values(self.parse_value_list()?)),
        }
    }

    fn parse_value_list(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut values = Vec::new();

        loop {
            match &self.peek().kind {
                TokenKind::RightBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    return Err(ParseError::new(
                        self.peek().line,
                        self.peek().column,
                        "expected '}' before end of input",
                    ));
                }
                _ => values.push(self.parse_value()?),
            }
        }

        Ok(values)
    }

    fn detect_block_mode(&self) -> Result<BlockMode, ParseError> {
        let current = self.peek();
        let next = self.peek_n(1);

        match &current.kind {
            TokenKind::Identifier(_) => {
                if matches!(next.kind, TokenKind::Equals)
                    || is_bare_identifier_statement_boundary(current, next)
                {
                    Ok(BlockMode::Statements)
                } else {
                    Ok(BlockMode::Values)
                }
            }
            TokenKind::String(_) => {
                if matches!(next.kind, TokenKind::Equals) {
                    Ok(BlockMode::Statements)
                } else {
                    Ok(BlockMode::Values)
                }
            }
            TokenKind::Integer(_) | TokenKind::Decimal(_) => {
                if matches!(next.kind, TokenKind::Equals) {
                    Ok(BlockMode::Statements)
                } else {
                    Ok(BlockMode::Values)
                }
            }
            TokenKind::LeftBrace => Ok(BlockMode::Values),
            TokenKind::RightBrace => Ok(BlockMode::Statements),
            TokenKind::Eof => Err(ParseError::new(
                current.line,
                current.column,
                "expected '}' before end of input",
            )),
            TokenKind::Equals => Err(ParseError::new(
                current.line,
                current.column,
                "unexpected '=' at start of block",
            )),
        }
    }

    fn is_bare_identifier_statement_boundary(&self, key_token: &Token) -> bool {
        let next_token = self.peek();

        is_bare_identifier_statement_boundary(key_token, next_token)
    }

    fn peek_n(&self, offset: usize) -> &Token {
        let last_index = self.tokens.len() - 1;
        let index = (self.position + offset).min(last_index);

        &self.tokens[index]
    }

    fn only_trailing_right_braces_remain(&self) -> bool {
        self.tokens[self.position..]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::RightBrace | TokenKind::Eof))
    }

    fn consume_trailing_right_braces(&mut self) {
        while matches!(self.peek().kind, TokenKind::RightBrace) {
            self.advance();
        }
    }
}

fn is_bare_identifier_statement_boundary(key_token: &Token, next_token: &Token) -> bool {
    matches!(next_token.kind, TokenKind::RightBrace | TokenKind::Eof)
        || next_token.line > key_token.line
}

#[cfg(test)]
mod tests {
    use super::{Block, Document, Statement, TokenKind, Value, parse_document, tokenize};

    #[test]
    fn tokenizes_identifiers_with_supported_characters() {
        let tokens = tokenize("foo_bar-baz.qux:quux").unwrap();

        assert_eq!(
            tokens[0].kind,
            TokenKind::Identifier("foo_bar-baz.qux:quux".to_string())
        );
    }

    #[test]
    fn tokenizes_strings_without_escapes() {
        let tokens = tokenize("\"United Kingdom\"").unwrap();

        assert_eq!(
            tokens[0].kind,
            TokenKind::String("United Kingdom".to_string())
        );
    }

    #[test]
    fn tokenizes_signed_numbers() {
        let tokens = tokenize("-42 +17 -12.5 99.25").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Integer(-42));
        assert_eq!(tokens[1].kind, TokenKind::Integer(17));
        assert_eq!(tokens[2].kind, TokenKind::Decimal(-12.5));
        assert_eq!(tokens[3].kind, TokenKind::Decimal(99.25));
    }

    #[test]
    fn tokenizes_braces_and_equals() {
        let tokens = tokenize("= { }").unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Equals);
        assert_eq!(tokens[1].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[2].kind, TokenKind::RightBrace);
    }

    #[test]
    fn parses_single_top_level_statement() {
        let document = parse_document("tag = ENG").unwrap();

        assert_eq!(
            document,
            Document {
                statements: vec![Statement {
                    key: "tag".to_string(),
                    value: Some(Value::Identifier("ENG".to_string())),
                }],
            }
        );
    }

    #[test]
    fn parses_numeric_statement_keys() {
        let document = parse_document(
            r#"
            123 = {
              456 = ENG
            }
            "#,
        )
        .unwrap();

        assert_eq!(document.statements[0].key, "123");
        assert!(matches!(
            document.statements[0].value,
            Some(Value::Block(Block::Statements(_)))
        ));

        let Some(Value::Block(Block::Statements(statements))) = &document.statements[0].value
        else {
            panic!("expected statement block value");
        };

        assert_eq!(statements[0].key, "456");
        assert_eq!(
            statements[0].value,
            Some(Value::Identifier("ENG".to_string()))
        );
    }

    #[test]
    fn parses_quoted_string_statement_keys() {
        let document = parse_document(
            r#"
            national_focus = {
              "234" = "promote_craftsmen"
            }
            "#,
        )
        .unwrap();

        let Some(Value::Block(Block::Statements(statements))) = &document.statements[0].value
        else {
            panic!("expected statement block value");
        };

        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0].key, "234");
        assert_eq!(
            statements[0].value,
            Some(Value::String("promote_craftsmen".to_string()))
        );
    }

    #[test]
    fn parses_value_list_blocks() {
        let document = parse_document(
            r#"
            budget_balance = {
              9246.92853 9244.16919 10585.42148
            }
            "#,
        )
        .unwrap();

        let Some(Value::Block(Block::Values(values))) = &document.statements[0].value else {
            panic!("expected value-list block");
        };

        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Value::Decimal(9246.92853));
        assert_eq!(values[1], Value::Decimal(9244.16919));
        assert_eq!(values[2], Value::Decimal(10585.42148));
    }

    #[test]
    fn parses_bare_identifier_statement() {
        let document = parse_document(
            r#"
            active_war
            tag = ENG
            "#,
        )
        .unwrap();

        assert_eq!(document.statements[0].key, "active_war");
        assert_eq!(document.statements[0].value, None);
        assert_eq!(
            document.statements[1].value,
            Some(Value::Identifier("ENG".to_string()))
        );
    }

    #[test]
    fn parses_nested_blocks() {
        let document = parse_document(
            r#"
            country = {
              tag = ENG
              name = "United Kingdom"
              prestige = 12.5
            }
            "#,
        )
        .unwrap();

        let Some(Value::Block(Block::Statements(statements))) = &document.statements[0].value
        else {
            panic!("expected statement block value");
        };

        assert_eq!(document.statements[0].key, "country");
        assert_eq!(statements.len(), 3);
        assert_eq!(statements[0].key, "tag");
        assert_eq!(statements[1].key, "name");
        assert_eq!(statements[2].value, Some(Value::Decimal(12.5)));
    }

    #[test]
    fn preserves_top_level_statement_order() {
        let document = parse_document(
            r#"
            first = 1
            second = 2
            third = 3
            "#,
        )
        .unwrap();

        let keys: Vec<&str> = document
            .statements
            .iter()
            .map(|statement| statement.key.as_str())
            .collect();

        assert_eq!(keys, vec!["first", "second", "third"]);
    }

    #[test]
    fn accepts_trailing_root_closing_braces() {
        let document = parse_document("tag = ENG }}").unwrap();

        assert_eq!(document.statements.len(), 1);
        assert_eq!(document.statements[0].key, "tag");
        assert_eq!(
            document.statements[0].value,
            Some(Value::Identifier("ENG".to_string()))
        );
    }

    #[test]
    fn rejects_missing_equals() {
        let error = parse_document("tag ENG").unwrap_err();

        assert!(error.to_string().contains("expected '=' after key 'tag'"));
    }

    #[test]
    fn rejects_non_trailing_root_closing_brace() {
        let error = parse_document("tag = ENG } other = FRA").unwrap_err();

        assert!(error.to_string().contains("unexpected '}'"));
    }

    #[test]
    fn rejects_bare_numeric_statement() {
        let error = parse_document("123").unwrap_err();

        assert!(error.to_string().contains("expected '=' after key '123'"));
    }

    #[test]
    fn rejects_missing_closing_brace() {
        let error = parse_document("country = { tag = ENG").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("expected '}' before end of input")
        );
    }

    #[test]
    fn rejects_invalid_string_termination() {
        let error = parse_document("name = \"United Kingdom").unwrap_err();

        assert!(error.to_string().contains("unterminated string literal"));
    }

    #[test]
    fn preserves_duplicate_keys_in_one_block() {
        let document = parse_document("country = { tag = ENG tag = FRA }").unwrap();

        let Some(Value::Block(Block::Statements(statements))) = &document.statements[0].value
        else {
            panic!("expected statement block value");
        };

        assert_eq!(statements.len(), 2);
        assert_eq!(statements[0].key, "tag");
        assert_eq!(
            statements[0].value,
            Some(Value::Identifier("ENG".to_string()))
        );
        assert_eq!(statements[1].key, "tag");
        assert_eq!(
            statements[1].value,
            Some(Value::Identifier("FRA".to_string()))
        );
    }
}
