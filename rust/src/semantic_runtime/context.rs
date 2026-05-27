use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context snapshot gathered from all available sources before executing an intent.
/// This gives the orchestrator awareness of the current environment state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeContext {
    /// Local filesystem context
    pub fs: FilesystemContext,
    /// Git repository context
    pub git: GitContext,
    /// Solana blockchain context
    pub solana: SolanaContext,
    /// Session-level metadata
    pub session: SessionContext,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemContext {
    pub current_dir: String,
    pub recent_files: Vec<String>,
    pub project_root: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitContext {
    pub is_repo: bool,
    pub branch: String,
    pub last_commit: String,
    pub remote_url: Option<String>,
    pub uncommitted_changes: Vec<String>,
    pub recent_commits: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolanaContext {
    pub current_slot: Option<u64>,
    pub current_epoch: Option<u64>,
    pub connected_endpoint: String,
    pub rpc_health: String,
    pub recent_queries: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionContext {
    pub action_count: usize,
    pub last_intent: Option<String>,
    pub last_subsystem: Option<String>,
    pub preferences: HashMap<String, String>,
}

impl RuntimeContext {
    /// Gather context from all available sources.
    /// This is best-effort: missing sources are silently skipped.
    pub async fn gather() -> Self {
        let mut ctx = Self::default();

        // Filesystem
        if let Ok(dir) = std::env::current_dir() {
            ctx.fs.current_dir = dir.to_string_lossy().to_string();
        }
        ctx.fs.project_root = Self::find_project_root();

        // Git
        ctx.gather_git_context();

        // Session
        ctx.session.preferences.insert(
            "lang".to_string(),
            std::env::var("LANG").unwrap_or_else(|_| "en".to_string()),
        );

        ctx
    }

    /// Attach Solana-specific context.
    pub async fn with_solana(mut self, endpoint: &str) -> Self {
        self.solana.connected_endpoint = endpoint.to_string();
        self.solana.rpc_health = "unknown".to_string();
        self
    }

    fn find_project_root() -> Option<String> {
        let mut path = std::env::current_dir().ok()?;
        loop {
            if path.join("Cargo.toml").exists()
                || path.join("package.json").exists()
                || path.join(".git").exists()
            {
                return Some(path.to_string_lossy().to_string());
            }
            if !path.pop() {
                break;
            }
        }
        None
    }

    fn gather_git_context(&mut self) {
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
        {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if s != "true" {
                return;
            }
            self.git.is_repo = true;
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["branch", "--show-current"])
            .output()
        {
            self.git.branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["log", "-1", "--format=%H"])
            .output()
        {
            self.git.last_commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
        {
            let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !url.is_empty() {
                self.git.remote_url = Some(url);
            }
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["diff", "--name-only"])
            .output()
        {
            let diff = String::from_utf8_lossy(&output.stdout);
            self.git.uncommitted_changes = diff
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["log", "-5", "--format=%s"])
            .output()
        {
            self.git.recent_commits = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
        }
    }
}
