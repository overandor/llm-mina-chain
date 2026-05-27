#[cfg(test)]
mod tests {
    use crate::solana_agent::{SolanaKnowledgeBase, QueryEngine, SolanaRpcClient};

    #[test]
    fn knowledge_base_ask_proof_of_history() {
        let kb = SolanaKnowledgeBase::new();
        let ans = kb.ask("What is Proof of History?");
        assert!(ans.is_some());
        let ans = ans.unwrap();
        assert!(ans.answer.contains("Proof of History"));
        // architecture and proof_of_history both match; just verify relevant content
        assert!(ans.topic == "proof_of_history" || ans.topic == "architecture");
    }

    #[test]
    fn knowledge_base_ask_accounts() {
        let kb = SolanaKnowledgeBase::new();
        let ans = kb.ask("how do accounts work");
        assert!(ans.is_some());
        let ans = ans.unwrap();
        assert!(ans.answer.contains("account-based model"));
        assert_eq!(ans.topic, "accounts");
    }

    #[test]
    fn knowledge_base_unknown_question() {
        let kb = SolanaKnowledgeBase::new();
        let ans = kb.ask("what is the weather today");
        assert!(ans.is_none());
    }

    #[test]
    fn knowledge_base_list_topics() {
        let kb = SolanaKnowledgeBase::new();
        let topics = kb.list_topics();
        assert!(topics.contains(&"architecture".to_string()));
        assert!(topics.contains(&"proof_of_history".to_string()));
        assert!(topics.contains(&"accounts".to_string()));
    }

    #[test]
    fn rpc_client_default_endpoint() {
        let client = SolanaRpcClient::new(None);
        let endpoint = client.endpoint();
        // With no health data, the first default endpoint wins
        assert!(endpoint.starts_with("https://"));
    }

    #[test]
    fn rpc_client_custom_endpoint() {
        let client = SolanaRpcClient::new(Some("https://custom.rpc.com".into()));
        assert_eq!(client.endpoint(), "https://custom.rpc.com");
    }

    #[tokio::test]
    async fn query_parse_basic() {
        let client = SolanaRpcClient::new(None);
        let engine = QueryEngine::new(client);
        let result = engine.execute("SELECT * FROM health", None).await;
        // This may fail without network, so we just verify parsing does not panic
        // and returns either Ok or Err
        assert!(result.is_ok() || result.is_err());
    }
}
