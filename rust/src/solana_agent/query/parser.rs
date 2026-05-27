//! Recursive-descent parser for the SQL-like query language.
//!
//! Converts `Vec<Token>` → `Statement` (AST).

use super::ast::*;
use super::tokenizer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> Result<Statement, String> {
        let stmt = self.parse_select()?;
        if !self.is_at_end() {
            return Err(format!(
                "Unexpected token after end of statement: {:?}",
                self.peek()
            ));
        }
        Ok(Statement::Select(stmt))
    }

    // SELECT <columns> FROM <tables> [WHERE <expr>] [ORDER BY <expr> [ASC|DESC]] [LIMIT <n>]
    fn parse_select(&mut self) -> Result<SelectStmt, String> {
        self.expect(Token::Select)?;

        let columns = self.parse_select_list()?;
        self.expect(Token::From)?;
        let from = self.parse_table_list()?;
        let r#where = if self.match_token(Token::Where) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let order_by = if self.match_token(Token::Order) {
            self.expect(Token::By)?;
            self.parse_order_by_list()?
        } else {
            vec![]
        };
        let limit = if self.match_token(Token::Limit) {
            Some(self.expect_number()?)
        } else {
            None
        };

        Ok(SelectStmt {
            columns,
            from,
            r#where,
            limit,
            order_by,
        })
    }

    // <select_item> (, <select_item>)*
    fn parse_select_list(&mut self) -> Result<Vec<SelectItem>, String> {
        let mut items = vec![self.parse_select_item()?];
        while self.match_token(Token::Comma) {
            items.push(self.parse_select_item()?);
        }
        Ok(items)
    }

    fn parse_select_item(&mut self) -> Result<SelectItem, String> {
        if self.match_token(Token::Star) {
            Ok(SelectItem::Wildcard)
        } else {
            let expr = self.parse_expr()?;
            let alias = if self.match_keyword("as") {
                Some(self.expect_identifier()?)
            } else {
                None
            };
            Ok(SelectItem::Expr(expr, alias))
        }
    }

    // <table_ref> (, <table_ref>)*
    fn parse_table_list(&mut self) -> Result<Vec<TableRef>, String> {
        let mut tables = vec![self.parse_table_ref()?];
        while self.match_token(Token::Comma) {
            tables.push(self.parse_table_ref()?);
        }
        Ok(tables)
    }

    fn parse_table_ref(&mut self) -> Result<TableRef, String> {
        let name = self.expect_identifier()?;
        let alias = if self.match_keyword("as") {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        Ok(TableRef { name, alias })
    }

    // ORDER BY <expr> [ASC|DESC] (, <expr> [ASC|DESC])*
    fn parse_order_by_list(&mut self) -> Result<Vec<OrderByExpr>, String> {
        let mut items = vec![self.parse_order_by_expr()?];
        while self.match_token(Token::Comma) {
            items.push(self.parse_order_by_expr()?);
        }
        Ok(items)
    }

    fn parse_order_by_expr(&mut self) -> Result<OrderByExpr, String> {
        let expr = self.parse_expr()?;
        let asc = if self.match_token(Token::Asc) {
            true
        } else if self.match_token(Token::Desc) {
            false
        } else {
            true // default ASC
        };
        Ok(OrderByExpr { expr, asc })
    }

    // ---- Expression parsing (precedence climbing) ----

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and_expr()?;
        while self.match_token(Token::Or) {
            let right = self.parse_and_expr()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison_expr()?;
        while self.match_token(Token::And) {
            let right = self.parse_comparison_expr()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_additive_expr()?;

        if self.match_token(Token::Eq) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Eq,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Neq) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Neq,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Lt) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Lt,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Gt) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Gt,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Lte) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Lte,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Gte) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Gte,
                right: Box::new(right),
            })
        } else if self.match_token(Token::Like) {
            let right = self.parse_additive_expr()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Like,
                right: Box::new(right),
            })
        } else if self.match_token(Token::In) {
            self.expect(Token::LParen)?;
            let mut list = vec![self.parse_expr()?];
            while self.match_token(Token::Comma) {
                list.push(self.parse_expr()?);
            }
            self.expect(Token::RParen)?;
            Ok(Expr::InList {
                expr: Box::new(left),
                list,
                negated: false,
            })
        } else if self.match_token(Token::Is) {
            let negated = self.match_token(Token::Not);
            self.expect(Token::Null)?;
            Ok(Expr::IsNull(Box::new(left), negated))
        } else if self.match_keyword("not") && self.peek_ahead_matches(0, Token::In) {
            self.advance(); // consume 'not'
            self.advance(); // consume 'in'
            self.expect(Token::LParen)?;
            let mut list = vec![self.parse_expr()?];
            while self.match_token(Token::Comma) {
                list.push(self.parse_expr()?);
            }
            self.expect(Token::RParen)?;
            Ok(Expr::InList {
                expr: Box::new(left),
                list,
                negated: true,
            })
        } else if self.match_keyword("not") && self.peek_ahead_matches(0, Token::Between) {
            self.advance();
            self.advance();
            let low = self.parse_additive_expr()?;
            self.expect(Token::And)?;
            let high = self.parse_additive_expr()?;
            Ok(Expr::Between {
                expr: Box::new(left),
                low: Box::new(low),
                high: Box::new(high),
                negated: true,
            })
        } else if self.match_token(Token::Between) {
            let low = self.parse_additive_expr()?;
            self.expect(Token::And)?;
            let high = self.parse_additive_expr()?;
            Ok(Expr::Between {
                expr: Box::new(left),
                low: Box::new(low),
                high: Box::new(high),
                negated: false,
            })
        } else {
            Ok(left)
        }
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_unary_expr()?;
        // For now, no + or - operators in our simplified grammar
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        if self.match_token(Token::Not) {
            let inner = self.parse_unary_expr()?;
            // Simplified: NOT before a comparison is handled in comparison_expr
            Ok(inner)
        } else {
            self.parse_primary_expr()
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, String> {
        if self.match_token(Token::LParen) {
            let expr = self.parse_expr()?;
            self.expect(Token::RParen)?;
            Ok(expr)
        } else if let Some(n) = self.match_number() {
            Ok(Expr::Number(n))
        } else if let Some(s) = self.match_string_literal() {
            Ok(Expr::StringLiteral(s))
        } else if let Some(ident) = self.match_identifier() {
            if self.match_token(Token::LParen) {
                // Function call
                let mut args = Vec::new();
                if !self.match_token(Token::RParen) {
                    args.push(self.parse_expr()?);
                    while self.match_token(Token::Comma) {
                        args.push(self.parse_expr()?);
                    }
                    self.expect(Token::RParen)?;
                }
                Ok(Expr::Function { name: ident, args })
            } else {
                Ok(Expr::Identifier(ident))
            }
        } else {
            Err(format!("Unexpected token: {:?}", self.peek()))
        }
    }

    // ---- Helpers ----

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn peek_ahead_matches(&self, offset: usize, tok: Token) -> bool {
        self.peek_ahead(offset).map(|t| *t == tok).unwrap_or(false)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if !self.is_at_end() {
            self.pos += 1;
        }
        tok
    }

    fn is_at_end(&self) -> bool {
        matches!(self.tokens.get(self.pos), Some(Token::Eof) | None)
    }

    fn match_token(&mut self, expected: Token) -> bool {
        if *self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        if let Token::Identifier(ref s) = self.peek() {
            if s.eq_ignore_ascii_case(kw) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_number(&mut self) -> Option<u64> {
        if let Token::Number(n) = *self.peek() {
            self.advance();
            Some(n)
        } else {
            None
        }
    }

    fn match_string_literal(&mut self) -> Option<String> {
        if let Token::StringLiteral(ref s) = self.peek() {
            let s = s.clone();
            self.advance();
            Some(s)
        } else {
            None
        }
    }

    fn match_identifier(&mut self) -> Option<String> {
        if let Token::Identifier(ref s) = self.peek() {
            let s = s.clone();
            self.advance();
            Some(s)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.match_token(expected.clone()) {
            Ok(())
        } else {
            Err(format!(
                "Expected {:?}, found {:?}",
                expected,
                self.peek()
            ))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        self.match_identifier()
            .ok_or_else(|| format!("Expected identifier, found {:?}", self.peek()))
    }

    fn expect_number(&mut self) -> Result<u64, String> {
        self.match_number()
            .ok_or_else(|| format!("Expected number, found {:?}", self.peek()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::tokenizer::Tokenizer;

    fn parse(sql: &str) -> Statement {
        let tokens = Tokenizer::new(sql).tokenize().unwrap();
        Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn parse_select_star() {
        let stmt = parse("SELECT * FROM status");
        match stmt {
            Statement::Select(s) => {
                assert_eq!(s.columns, vec![SelectItem::Wildcard]);
                assert_eq!(s.from, vec![TableRef { name: "status".to_string(), alias: None }]);
                assert!(s.r#where.is_none());
                assert!(s.limit.is_none());
            }
        }
    }

    #[test]
    fn parse_where_eq() {
        let stmt = parse("SELECT * FROM accounts WHERE pubkey = 'abc'");
        match stmt {
            Statement::Select(s) => {
                let w = s.r#where.unwrap();
                let (col, val) = w.as_eq_comparison().unwrap();
                assert_eq!(col, "pubkey");
                assert_eq!(val, Expr::StringLiteral("abc".to_string()));
            }
        }
    }

    #[test]
    fn parse_limit() {
        let stmt = parse("SELECT * FROM transactions LIMIT 50");
        match stmt {
            Statement::Select(s) => {
                assert_eq!(s.limit, Some(50));
            }
        }
    }

    #[test]
    fn parse_and_condition() {
        let stmt = parse("SELECT * FROM accounts WHERE program_id = 'Token' AND amount > 1000");
        match stmt {
            Statement::Select(s) => {
                assert!(s.r#where.is_some());
            }
        }
    }
}
