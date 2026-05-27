//! Tokenizer for the SQL-like query language.
//!
//! Converts raw query strings into a stream of typed tokens.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    // Keywords
    Select,
    From,
    Where,
    Limit,
    Order,
    By,
    Asc,
    Desc,
    And,
    Or,
    Not,
    Is,
    Null,
    In,
    Between,
    Like,
    // Literals
    Number(u64),
    StringLiteral(String),
    Identifier(String),
    // Operators
    Eq,      // =
    Neq,     // != or <>
    Lt,      // <
    Gt,      // >
    Lte,     // <=
    Gte,     // >=
    Star,    // *
    Comma,   // ,
    LParen,  // (
    RParen,  // )
    // End
    Eof,
}

pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            if tok == Token::Eof {
                tokens.push(tok);
                break;
            }
            tokens.push(tok);
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Ok(Token::Eof);
        }

        let c = self.peek();

        // Number
        if c.is_ascii_digit() {
            return self.read_number();
        }

        // String literal (single or double quotes)
        if c == '\'' || c == '"' {
            return self.read_string_literal();
        }

        // Identifier or keyword
        if c.is_ascii_alphabetic() || c == '_' {
            return self.read_word();
        }

        // Multi-char operators
        match (c, self.peek_next()) {
            ('!', Some('=')) => {
                self.advance();
                self.advance();
                return Ok(Token::Neq);
            }
            ('<', Some('=')) => {
                self.advance();
                self.advance();
                return Ok(Token::Lte);
            }
            ('>', Some('=')) => {
                self.advance();
                self.advance();
                return Ok(Token::Gte);
            }
            ('<', Some('>')) => {
                self.advance();
                self.advance();
                return Ok(Token::Neq);
            }
            _ => {}
        }

        // Single-char tokens
        let tok = match c {
            '=' => Token::Eq,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '*' => Token::Star,
            ',' => Token::Comma,
            '(' => Token::LParen,
            ')' => Token::RParen,
            _ => return Err(format!("Unexpected character: '{}' at position {}", c, self.pos)),
        };
        self.advance();
        Ok(tok)
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let start = self.pos;
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }
        let s: String = self.input[start..self.pos].iter().collect();
        let n = s.parse::<u64>().map_err(|e| format!("Invalid number: {}", e))?;
        Ok(Token::Number(n))
    }

    fn read_string_literal(&mut self) -> Result<Token, String> {
        let quote = self.advance(); // consume opening quote
        let start = self.pos;
        while !self.is_at_end() && self.peek() != quote {
            self.advance();
        }
        if self.is_at_end() {
            return Err("Unterminated string literal".to_string());
        }
        let s: String = self.input[start..self.pos].iter().collect();
        self.advance(); // consume closing quote
        Ok(Token::StringLiteral(s))
    }

    fn read_word(&mut self) -> Result<Token, String> {
        let start = self.pos;
        while !self.is_at_end()
            && (self.peek().is_ascii_alphanumeric() || self.peek() == '_' || self.peek() == '.')
        {
            self.advance();
        }
        let s: String = self.input[start..self.pos].iter().collect::<String>().to_lowercase();

        let tok = match s.as_str() {
            "select" => Token::Select,
            "from" => Token::From,
            "where" => Token::Where,
            "limit" => Token::Limit,
            "order" => Token::Order,
            "by" => Token::By,
            "asc" => Token::Asc,
            "desc" => Token::Desc,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "is" => Token::Is,
            "null" => Token::Null,
            "in" => Token::In,
            "between" => Token::Between,
            "like" => Token::Like,
            _ => Token::Identifier(s),
        };
        Ok(tok)
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> char {
        self.input[self.pos]
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.input.len() {
            Some(self.input[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let c = self.input[self.pos];
        self.pos += 1;
        c
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_select_star() {
        let t = Tokenizer::new("SELECT * FROM status");
        let tokens = t.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Select,
                Token::Star,
                Token::From,
                Token::Identifier("status".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_where_clause() {
        let t = Tokenizer::new("SELECT slot FROM blocks WHERE slot > 42 LIMIT 10");
        let tokens = t.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Select,
                Token::Identifier("slot".to_string()),
                Token::From,
                Token::Identifier("blocks".to_string()),
                Token::Where,
                Token::Identifier("slot".to_string()),
                Token::Gt,
                Token::Number(42),
                Token::Limit,
                Token::Number(10),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn tokenize_string_literal() {
        let t = Tokenizer::new("SELECT * FROM accounts WHERE pubkey = 'So11111111111111111111111111111111111111112'");
        let tokens = t.tokenize().unwrap();
        assert!(matches!(tokens[6], Token::Eq));
        assert!(matches!(&tokens[7], Token::StringLiteral(s) if s == "So11111111111111111111111111111111111111112"));
    }
}
