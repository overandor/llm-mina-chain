use regex::Regex;

use super::types::{Entity, EntityType, IntentType, ParsedIntent};

/// Routes natural language input to the appropriate intent type and extracts entities.
pub struct QueryRouter;

impl Default for QueryRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, input: &str) -> ParsedIntent {
        let input = input.trim();
        let lower = input.to_lowercase();

        // Try each intent pattern in order of specificity
        if let Some(intent) = self.try_solana_account(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_solana_transaction(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_solana_block(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_solana_status(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_knowledge_base(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_filesystem(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_github(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_command(&lower, input) {
            return intent;
        }
        if let Some(intent) = self.try_wallet_analysis(&lower, input) {
            return intent;
        }

        // Fallback: if it starts with SELECT or looks like SQL, treat as Solana query
        if lower.starts_with("select ") || lower.starts_with("show ") {
            return ParsedIntent {
                raw: input.to_string(),
                intent_type: IntentType::QuerySolanaStatus,
                entities: vec![],
                confidence: 0.7,
            };
        }

        ParsedIntent {
            raw: input.to_string(),
            intent_type: IntentType::QueryKnowledgeBase,
            entities: vec![],
            confidence: 0.3,
        }
    }

    fn try_solana_account(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let patterns = [
            Regex::new(r"(account|balance|info|state)\s+(?:of|for\s+)?([a-zA-Z0-9]{32,44})").unwrap(),
            Regex::new(r"([a-zA-Z0-9]{32,44})\s+(?:account|balance)").unwrap(),
            Regex::new(r"(?:get|fetch|show)\s+(?:account\s+)?(?:info|balance|state)\s+(?:of|for\s+)?([a-zA-Z0-9]{32,44})").unwrap(),
        ];
        for pat in &patterns {
            if let Some(caps) = pat.captures(lower) {
                let pk = caps.get(2).or_else(|| caps.get(1)).unwrap().as_str();
                return Some(ParsedIntent {
                    raw: raw.to_string(),
                    intent_type: IntentType::QuerySolanaAccount,
                    entities: vec![Entity {
                        entity_type: EntityType::Pubkey,
                        value: pk.to_string(),
                        position: (0, 0),
                    }],
                    confidence: 0.9,
                });
            }
        }
        None
    }

    fn try_solana_transaction(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let pat = Regex::new(r"(transaction|tx|signature)\s+(?:of\s+)?([a-zA-Z0-9]{64,128})").unwrap();
        if let Some(caps) = pat.captures(lower) {
            let sig = caps.get(2).unwrap().as_str();
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QuerySolanaTransaction,
                entities: vec![Entity {
                    entity_type: EntityType::Signature,
                    value: sig.to_string(),
                    position: (0, 0),
                }],
                confidence: 0.9,
            });
        }
        None
    }

    fn try_solana_block(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let pat = Regex::new(r"block\s+(?:at\s+slot\s+)?(\d+)").unwrap();
        if let Some(caps) = pat.captures(lower) {
            let slot = caps.get(1).unwrap().as_str();
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QuerySolanaBlock,
                entities: vec![Entity {
                    entity_type: EntityType::Slot,
                    value: slot.to_string(),
                    position: (0, 0),
                }],
                confidence: 0.9,
            });
        }
        // "latest block" or "current block"
        if lower.contains("latest block") || lower.contains("current block") {
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QuerySolanaBlock,
                entities: vec![],
                confidence: 0.85,
            });
        }
        None
    }

    fn try_solana_status(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let keywords = ["slot", "epoch", "supply", "health", "status", "version", "validators", "nodes"];
        if keywords.iter().any(|k| lower.contains(k))
            && (lower.contains("solana") || lower.contains("chain") || lower.contains("network") || lower.contains("current"))
        {
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QuerySolanaStatus,
                entities: vec![],
                confidence: 0.75,
            });
        }
        None
    }

    fn try_knowledge_base(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let patterns = [
            "what is", "how does", "explain", "what are", "tell me about",
            "why is", "when did", "who created", "architecture", "proof of history",
            "poh", "accounts", "transactions", "programs", "tokens", "staking",
            "consensus", "fees", "pda", "state compression", "security",
        ];
        if patterns.iter().any(|p| lower.contains(p)) {
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QueryKnowledgeBase,
                entities: vec![],
                confidence: 0.8,
            });
        }
        None
    }

    fn try_filesystem(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let patterns = [
            "file", "directory", "folder", "path", "ls ", "cat ", "read ", "write ",
        ];
        if patterns.iter().any(|p| lower.contains(p)) {
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QueryFilesystem,
                entities: vec![],
                confidence: 0.6,
            });
        }
        None
    }

    fn try_github(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let patterns = [
            "github", "repo", "repository", "pr", "pull request", "issue",
            "commit", "branch", "merge", "clone",
        ];
        if patterns.iter().any(|p| lower.contains(p)) {
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::QueryGitHub,
                entities: vec![],
                confidence: 0.7,
            });
        }
        None
    }

    fn try_command(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        if lower.starts_with("!") || lower.starts_with("run ") || lower.starts_with("exec ") {
            let cmd = if lower.starts_with("!") {
                &raw[1..]
            } else {
                raw.split_once(' ').map(|x| x.1).unwrap_or("")
            };
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::ExecuteCommand,
                entities: vec![Entity {
                    entity_type: EntityType::Command,
                    value: cmd.to_string(),
                    position: (0, 0),
                }],
                confidence: 0.9,
            });
        }
        None
    }

    fn try_wallet_analysis(&self, lower: &str, raw: &str) -> Option<ParsedIntent> {
        let patterns = [
            "analyze wallet", "wallet analysis", "portfolio", "holdings",
            "token accounts", "nfts", "transactions for", "history of",
        ];
        if patterns.iter().any(|p| lower.contains(p)) {
            // Try to extract a pubkey
            let pk_pat = Regex::new(r"([a-zA-Z0-9]{32,44})").unwrap();
            let entities = pk_pat
                .captures(lower)
                .map(|caps| {
                    vec![Entity {
                        entity_type: EntityType::Pubkey,
                        value: caps.get(1).unwrap().as_str().to_string(),
                        position: (0, 0),
                    }]
                })
                .unwrap_or_default();
            return Some(ParsedIntent {
                raw: raw.to_string(),
                intent_type: IntentType::AnalyzeWallet,
                entities,
                confidence: 0.75,
            });
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_account_query() {
        let router = QueryRouter::new();
        let intent = router.parse("get account info for So11111111111111111111111111111111111111112");
        assert_eq!(intent.intent_type, IntentType::QuerySolanaAccount);
        assert_eq!(intent.entities.len(), 1);
        assert_eq!(intent.entities[0].value, "so11111111111111111111111111111111111111112");
    }

    #[test]
    fn route_knowledge_query() {
        let router = QueryRouter::new();
        let intent = router.parse("what is proof of history?");
        assert_eq!(intent.intent_type, IntentType::QueryKnowledgeBase);
    }

    #[test]
    fn route_sql_query() {
        let router = QueryRouter::new();
        let intent = router.parse("SELECT * FROM status");
        assert_eq!(intent.intent_type, IntentType::QuerySolanaStatus);
    }

    #[test]
    fn route_command() {
        let router = QueryRouter::new();
        let intent = router.parse("!cargo test");
        assert_eq!(intent.intent_type, IntentType::ExecuteCommand);
    }
}
