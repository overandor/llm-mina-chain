//! Typed SQL-like AST for Solana blockchain queries.
//!
//! Replaces regex-based parsing with a proper grammar structure
//! that supports planning, optimization, and semantic execution.

use serde::{Deserialize, Serialize};

/// The top-level query statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Statement {
    Select(SelectStmt),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectStmt {
    pub columns: Vec<SelectItem>,
    pub from: Vec<TableRef>,
    pub r#where: Option<Expr>,
    pub limit: Option<u64>,
    pub order_by: Vec<OrderByExpr>,
}

/// A selected column or expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectItem {
    Wildcard,
    Expr(Expr, Option<String>), // expression, optional alias
}

/// Reference to a virtual blockchain table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
}

/// A boolean or arithmetic expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    /// Numeric literal (e.g. 42)
    Number(u64),
    /// String literal (e.g. 'Tokenkeg...')
    StringLiteral(String),
    /// Identifier (column or table name)
    Identifier(String),
    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    /// Unary negation
    UnaryNeg(Box<Expr>),
    /// Function call
    Function {
        name: String,
        args: Vec<Expr>,
    },
    /// IS NULL / IS NOT NULL
    IsNull(Box<Expr>, bool), // expr, negated
    /// IN (...)
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// BETWEEN low AND high
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
    Like,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderByExpr {
    pub expr: Expr,
    pub asc: bool,
}

impl SelectStmt {
    /// Convenience: is this a simple `SELECT * FROM <table>` with no WHERE/LIMIT?
    pub fn is_simple_table_scan(&self) -> bool {
        self.columns.len() == 1
            && matches!(self.columns[0], SelectItem::Wildcard)
            && self.r#where.is_none()
            && self.limit.is_none()
            && self.order_by.is_empty()
    }

    /// Convenience: get the primary table name.
    pub fn primary_table(&self) -> Option<&str> {
        self.from.first().map(|t| t.name.as_str())
    }
}

impl Expr {
    /// Try to extract a simple `column = value` comparison.
    pub fn as_eq_comparison(&self) -> Option<(String, Expr)> {
        if let Expr::BinaryOp { left, op: BinaryOp::Eq, right } = self {
            if let Expr::Identifier(name) = left.as_ref() {
                return Some((name.clone(), *right.clone()));
            }
        }
        None
    }

    /// Walk the AST and extract all referenced column names.
    pub fn column_refs(&self) -> Vec<String> {
        let mut refs = Vec::new();
        self.collect_columns(&mut refs);
        refs
    }

    fn collect_columns(&self, out: &mut Vec<String>) {
        match self {
            Expr::Identifier(name) => out.push(name.clone()),
            Expr::BinaryOp { left, right, .. } => {
                left.collect_columns(out);
                right.collect_columns(out);
            }
            Expr::UnaryNeg(inner) => inner.collect_columns(out),
            Expr::Function { args, .. } => {
                for a in args {
                    a.collect_columns(out);
                }
            }
            Expr::IsNull(inner, _) => inner.collect_columns(out),
            Expr::InList { expr, list, .. } => {
                expr.collect_columns(out);
                for item in list {
                    item.collect_columns(out);
                }
            }
            Expr::Between { expr, low, high, .. } => {
                expr.collect_columns(out);
                low.collect_columns(out);
                high.collect_columns(out);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ast_eq_comparison() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Identifier("program_id".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::StringLiteral("Tokenkeg...".to_string())),
        };
        let (col, val) = expr.as_eq_comparison().unwrap();
        assert_eq!(col, "program_id");
        assert_eq!(val, Expr::StringLiteral("Tokenkeg...".to_string()));
    }

    #[test]
    fn ast_column_refs() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::Identifier("slot".to_string())),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Number(42)),
        };
        let refs = expr.column_refs();
        assert_eq!(refs, vec!["slot"]);
    }
}
