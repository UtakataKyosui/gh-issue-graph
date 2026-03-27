use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// GitHub API トークン (省略時は環境変数 or gh auth token)
    pub token: Option<String>,
    /// デフォルトリポジトリ (owner/repo 形式)
    pub default_repo: Option<String>,
    /// デフォルト探索深度
    pub default_depth: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: None,
            default_repo: None,
            default_depth: Some(2),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("GH_ISSUE_GRAPH_CONFIG") {
            return PathBuf::from(path);
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gh-issue-graph")
            .join("config.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    /// GitHub トークンを解決する
    /// 優先順: GH_TOKEN env → GITHUB_TOKEN env → config.token → `gh auth token` コマンド
    pub fn resolve_token(&self) -> Result<String> {
        if let Ok(token) = std::env::var("GH_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.is_empty() {
                return Ok(token);
            }
        }
        if let Some(ref token) = self.token {
            if !token.is_empty() {
                return Ok(token.clone());
            }
        }
        // `gh auth token` コマンドで取得
        let output = std::process::Command::new("gh")
            .args(["auth", "token"])
            .output()
            .context("Failed to run `gh auth token`. Is the GitHub CLI installed and authenticated?")?;
        if output.status.success() {
            let token = String::from_utf8(output.stdout)?.trim().to_string();
            if !token.is_empty() {
                return Ok(token);
            }
        }
        anyhow::bail!(
            "No GitHub token found. Set GH_TOKEN, GITHUB_TOKEN, or run `gh auth login`."
        )
    }
}
