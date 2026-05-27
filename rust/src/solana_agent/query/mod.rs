//! SQL-like Query Engine — proper AST-based parsing and planning.
//!
//! Replaces regex-based parsing with:
//! - Tokenizer (string → tokens)
//! - Parser (tokens → AST)
//! - Planner (AST → ExecutionPlan)
//!
//! This architecture enables future query optimization,
//! caching, parallel execution, and streaming.

pub mod ast;
pub mod parser;
pub mod planner;
pub mod tokenizer;

pub use ast::{BinaryOp, Expr, SelectItem, SelectStmt, Statement};
pub use parser::Parser;
pub use planner::{ExecutionPlan, FilterValue};
pub use tokenizer::{Token, Tokenizer};

/// Convenience: parse a SQL-like string into an execution plan.
pub fn parse_query(sql: &str) -> Result<ExecutionPlan, String> {
    let tokens = Tokenizer::new(sql).tokenize()?;
    let ast = Parser::new(tokens).parse()?;
    match ast {
        Statement::Select(stmt) => ExecutionPlan::from_ast(&stmt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_parse_and_plan() {
        let plan = parse_query("SELECT * FROM status").unwrap();
        assert_eq!(plan.table, "status");
        assert_eq!(plan.rpc_method, "getHealth");
    }

    #[test]
    fn end_to_end_account_query() {
        let plan = parse_query("SELECT * FROM accounts WHERE pubkey = 'So1111'").unwrap();
        assert_eq!(plan.table, "accounts");
        assert_eq!(plan.rpc_method, "getAccountInfo");
        assert_eq!(plan.filters[0].0, "pubkey");
    }
}
