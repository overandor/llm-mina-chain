//! Query Planner — translates parsed AST into execution plans.
//!
//! The planner maps SQL-like constructs to Solana RPC operations,
//! enabling optimization decisions (batching, caching, parallelism)
//! before execution.

use serde::{Deserialize, Serialize};

use super::ast::{BinaryOp, Expr, SelectStmt, SelectItem};

/// An execution plan describes how to fulfill a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Which virtual blockchain table to query.
    pub table: String,
    /// Columns to return.
    pub projection: Vec<String>,
    /// Simple equality filters extracted from WHERE.
    pub filters: Vec<(String, FilterValue)>,
    /// Complex remaining expression (if any).
    pub remaining_filter: Option<Expr>,
    /// Max rows to return.
    pub limit: Option<u64>,
    /// RPC method name that will service this plan.
    pub rpc_method: String,
    /// RPC params as JSON.
    pub rpc_params: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(u64),
}

impl ExecutionPlan {
    /// Build a plan from a parsed SELECT statement.
    pub fn from_ast(stmt: &SelectStmt) -> Result<Self, String> {
        let table = stmt
            .primary_table()
            .ok_or("Missing FROM table")?
            .to_string();

        let projection: Vec<String> = stmt
            .columns
            .iter()
            .map(|col| match col {
                SelectItem::Wildcard => "*".to_string(),
                SelectItem::Expr(Expr::Identifier(name), _) => name.clone(),
                SelectItem::Expr(_, Some(alias)) => alias.clone(),
                _ => "expr".to_string(),
            })
            .collect();

        let (filters, remaining_filter) = extract_filters(&stmt.r#where)?;
        let limit = stmt.limit;

        let (rpc_method, rpc_params) = plan_rpc_call(&table, &filters)?;

        Ok(ExecutionPlan {
            table,
            projection,
            filters,
            remaining_filter,
            limit,
            rpc_method,
            rpc_params,
        })
    }
}

/// Alias for the complex extract_filters return type.
type FilterExtractResult = Result<(Vec<(String, FilterValue)>, Option<Expr>), String>;

/// Extract simple equality filters from WHERE expressions.
/// Returns (simple_filters, complex_remaining_expr).
fn extract_filters(where_expr: &Option<Expr>) -> FilterExtractResult {
    let Some(expr) = where_expr else {
        return Ok((vec![], None));
    };

    // Flatten AND chain
    let mut and_parts = Vec::new();
    collect_and_parts(expr.clone(), &mut and_parts);

    let mut filters = Vec::new();
    let mut complex = Vec::new();

    for part in and_parts {
        if let Some((col, val)) = part.as_eq_comparison() {
            let fv = match val {
                Expr::StringLiteral(s) => FilterValue::String(s),
                Expr::Number(n) => FilterValue::Number(n),
                _ => {
                    complex.push(part);
                    continue;
                }
            };
            filters.push((col, fv));
        } else {
            complex.push(part);
        }
    }

    let remaining = if complex.is_empty() {
        None
    } else if complex.len() == 1 {
        Some(complex.into_iter().next().unwrap())
    } else {
        // Rebuild AND chain from remaining parts
        let mut result = complex.pop().unwrap();
        for part in complex {
            result = Expr::BinaryOp {
                left: Box::new(part),
                op: BinaryOp::And,
                right: Box::new(result),
            };
        }
        Some(result)
    };

    Ok((filters, remaining))
}

fn collect_and_parts(expr: Expr, out: &mut Vec<Expr>) {
    if let Expr::BinaryOp { left, op: BinaryOp::And, right } = expr {
        collect_and_parts(*left, out);
        collect_and_parts(*right, out);
    } else {
        out.push(expr);
    }
}

/// Map virtual table + filters to an RPC method and params.
fn plan_rpc_call(table: &str, filters: &[(String, FilterValue)]) -> Result<(String, serde_json::Value), String> {
    match table {
        "status" | "health" => Ok(("getHealth".to_string(), serde_json::json!([]))),
        "slot" | "slots" => Ok(("getSlot".to_string(), serde_json::json!([{"commitment": "confirmed"}]))),
        "epoch_info" | "epoch" => Ok(("getEpochInfo".to_string(), serde_json::json!([{"commitment": "confirmed"}]))),
        "supply" => Ok(("getSupply".to_string(), serde_json::json!([{"commitment": "confirmed"}]))),
        "block_height" | "blockheight" => Ok(("getBlockHeight".to_string(), serde_json::json!([{"commitment": "confirmed"}]))),
        "version" => Ok(("getVersion".to_string(), serde_json::json!([]))),
        "vote_accounts" => Ok(("getVoteAccounts".to_string(), serde_json::json!([{"commitment": "confirmed"}]))),
        "cluster_nodes" => Ok(("getClusterNodes".to_string(), serde_json::json!([]))),
        "genesis_hash" => Ok(("getGenesisHash".to_string(), serde_json::json!([]))),
        "first_available_block" => Ok(("getFirstAvailableBlock".to_string(), serde_json::json!([]))),
        "performance_samples" => Ok(("getRecentPerformanceSamples".to_string(), serde_json::json!([10]))),
        "blocks" => {
            // Try to find a slot filter
            for (col, val) in filters {
                if col == "slot" {
                    if let FilterValue::Number(n) = val {
                        return Ok(("getBlock".to_string(), serde_json::json!([
                            n,
                            {"commitment": "confirmed", "maxSupportedTransactionVersion": 0}
                        ])));
                    }
                }
            }
            Ok(("getSlot".to_string(), serde_json::json!([{"commitment": "confirmed"}])))
        }
        "accounts" => {
            for (col, val) in filters {
                if col == "pubkey" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getAccountInfo".to_string(), serde_json::json!([
                            s,
                            {"encoding": "base64", "commitment": "confirmed"}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        "transactions" | "transaction" => {
            for (col, val) in filters {
                if col == "signature" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getTransaction".to_string(), serde_json::json!([
                            s,
                            {"commitment": "confirmed", "maxSupportedTransactionVersion": 0}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        "token_accounts" => {
            for (col, val) in filters {
                if col == "owner" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getTokenAccountsByOwner".to_string(), serde_json::json!([
                            s,
                            {"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"},
                            {"encoding": "base64", "commitment": "confirmed"}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        "program_accounts" => {
            for (col, val) in filters {
                if col == "program_id" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getProgramAccounts".to_string(), serde_json::json!([
                            s,
                            {"encoding": "base64", "commitment": "confirmed"}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        "token_supply" => {
            for (col, val) in filters {
                if col == "mint" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getTokenSupply".to_string(), serde_json::json!([
                            s,
                            {"commitment": "confirmed"}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        "block_signatures" | "block_signatures_for_address" => {
            for (col, val) in filters {
                if col == "address" {
                    if let FilterValue::String(s) = val {
                        return Ok(("getSignaturesForAddress".to_string(), serde_json::json!([
                            s,
                            {"commitment": "confirmed", "limit": 10}
                        ])));
                    }
                }
            }
            Ok(("getHealth".to_string(), serde_json::json!([])))
        }
        _ => Err(format!("Unknown table: {}", table)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ast::*;

    #[test]
    fn plan_simple_status() {
        let stmt = SelectStmt {
            columns: vec![SelectItem::Wildcard],
            from: vec![TableRef { name: "status".to_string(), alias: None }],
            r#where: None,
            limit: None,
            order_by: vec![],
        };
        let plan = ExecutionPlan::from_ast(&stmt).unwrap();
        assert_eq!(plan.rpc_method, "getHealth");
    }

    #[test]
    fn plan_account_with_filter() {
        let stmt = SelectStmt {
            columns: vec![SelectItem::Wildcard],
            from: vec![TableRef { name: "accounts".to_string(), alias: None }],
            r#where: Some(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("pubkey".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::StringLiteral("So1111".to_string())),
            }),
            limit: None,
            order_by: vec![],
        };
        let plan = ExecutionPlan::from_ast(&stmt).unwrap();
        assert_eq!(plan.rpc_method, "getAccountInfo");
        assert_eq!(plan.filters.len(), 1);
        assert_eq!(plan.filters[0].0, "pubkey");
    }
}
