//! Authentication and rate limiting for the Solana Agent API
//! Supports API key authentication and request rate limiting

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Authentication configuration
#[derive(Clone, Default)]
pub struct AuthConfig {
    pub api_keys: Vec<String>,
    pub require_auth: bool,
}

impl AuthConfig {
    pub fn new(api_keys: Vec<String>, require_auth: bool) -> Self {
        Self {
            api_keys,
            require_auth,
        }
    }

    pub fn from_env() -> Self {
        let require_auth = std::env::var("SOLANA_AGENT_REQUIRE_AUTH")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let api_keys = if require_auth {
            std::env::var("SOLANA_AGENT_API_KEYS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![]
        };

        Self {
            api_keys,
            require_auth,
        }
    }

    pub fn validate_key(&self, key: &str) -> bool {
        if !self.require_auth {
            return true;
        }
        self.api_keys.iter().any(|k| k == key)
    }
}

/// Rate limiter using sliding window algorithm
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            requests: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(window_secs),
        }
    }

    pub async fn check_rate_limit(&self, identifier: &str) -> bool {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window_start = now - self.window;

        let timestamps = requests.entry(identifier.to_string()).or_insert_with(Vec::new);
        
        // Remove timestamps outside the window
        timestamps.retain(|&t| t > window_start);

        if timestamps.len() >= self.max_requests {
            warn!("Rate limit exceeded for identifier: {}", identifier);
            return false;
        }

        timestamps.push(now);
        debug!("Request allowed for identifier: {} (count: {})", identifier, timestamps.len());
        true
    }

    pub async fn cleanup(&self) {
        let mut requests = self.requests.write().await;
        let now = Instant::now();
        let window_start = now - self.window;

        for timestamps in requests.values_mut() {
            timestamps.retain(|&t| t > window_start);
        }

        // Remove empty entries
        requests.retain(|_, v| !v.is_empty());
    }
}

/// API key authentication middleware
pub async fn api_key_auth_middleware(
    State(config): State<Arc<AuthConfig>>,
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    if !config.require_auth {
        return Ok(next.run(request).await);
    }

    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
        });

    match api_key {
        Some(key) if config.validate_key(key) => {
            debug!("API key authenticated successfully");
            Ok(next.run(request).await)
        }
        _ => {
            warn!("Authentication failed: missing or invalid API key");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request<Body>,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    // Use IP address or API key as identifier
    let identifier = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
        })
        .unwrap_or({
            // Fallback to a generic identifier if no API key
            "anonymous"
        });

    if limiter.check_rate_limit(identifier).await {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.require_auth);
        assert!(config.validate_key("any_key"));
    }

    #[test]
    fn test_auth_config_with_keys() {
        let config = AuthConfig::new(vec!["key1".to_string(), "key2".to_string()], true);
        assert!(config.require_auth);
        assert!(config.validate_key("key1"));
        assert!(config.validate_key("key2"));
        assert!(!config.validate_key("key3"));
    }

    #[test]
    fn test_auth_config_no_auth_required() {
        let config = AuthConfig::new(vec!["key1".to_string()], false);
        assert!(!config.require_auth);
        assert!(config.validate_key("any_key"));
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(3, 1); // 3 requests per second

        assert!(limiter.check_rate_limit("user1").await);
        assert!(limiter.check_rate_limit("user1").await);
        assert!(limiter.check_rate_limit("user1").await);
        assert!(!limiter.check_rate_limit("user1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_users() {
        let limiter = RateLimiter::new(2, 1);

        assert!(limiter.check_rate_limit("user1").await);
        assert!(limiter.check_rate_limit("user1").await);
        assert!(!limiter.check_rate_limit("user1").await);

        // Different user should still be allowed
        assert!(limiter.check_rate_limit("user2").await);
        assert!(limiter.check_rate_limit("user2").await);
    }
}
