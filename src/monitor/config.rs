use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default = "default_poll_interval")]
    pub poll_interval: u64,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub repositories: Vec<RepoConfig>,
}

fn default_poll_interval() -> u64 {
    60
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            poll_interval: 60,
            token: None,
            repositories: vec![],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepoConfig {
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
}

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("GH_ISSUE_MONITOR_CONFIG") {
        return PathBuf::from(p);
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("gh-issue-monitor")
        .join("config.json")
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let config: AppConfig =
        serde_json::from_str(&content).with_context(|| format!("Invalid JSON in {}", path.display()))?;
    Ok(config)
}

pub fn init_config() -> Result<()> {
    let path = config_path();
    if path.exists() {
        println!("Config already exists at {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let default = AppConfig {
        poll_interval: 60,
        token: None,
        repositories: vec![RepoConfig {
            owner: "my-org".to_string(),
            name: "my-repo".to_string(),
            labels: None,
        }],
    };
    let json = serde_json::to_string_pretty(&default)?;
    std::fs::write(&path, json)?;
    println!("Created config at {}", path.display());
    Ok(())
}

pub fn show_config(config: &AppConfig) -> Result<()> {
    let mut display = config.clone();
    if display.token.is_some() {
        display.token = Some("***".to_string());
    }
    println!("{}", serde_json::to_string_pretty(&display)?);
    Ok(())
}

/// Resolve token: GH_TOKEN > GITHUB_TOKEN > config token > gh auth token
pub fn resolve_token(config: &AppConfig) -> Result<String> {
    if let Ok(t) = std::env::var("GH_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.is_empty() {
            return Ok(t);
        }
    }
    if let Some(t) = &config.token {
        if !t.is_empty() {
            return Ok(t.clone());
        }
    }
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("`gh auth token` failed — install gh CLI or set GH_TOKEN")?;
    let token = String::from_utf8(output.stdout)
        .context("gh auth token output is not valid UTF-8")?
        .trim()
        .to_string();
    if token.is_empty() {
        anyhow::bail!(
            "No GitHub token found. Set GH_TOKEN, GITHUB_TOKEN, or run `gh auth login`"
        );
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn load_missing_file_returns_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("GH_ISSUE_MONITOR_CONFIG", "/tmp/gh_issue_monitor_nonexistent_test.json");
        let cfg = load_config().unwrap();
        assert_eq!(cfg.poll_interval, 60);
        assert!(cfg.repositories.is_empty());
        std::env::remove_var("GH_ISSUE_MONITOR_CONFIG");
    }

    #[test]
    fn load_valid_json() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join("gh_issue_monitor_test_valid.json");
        let mut f = std::fs::File::create(&tmp).unwrap();
        write!(f, r#"{{"poll_interval":30,"repositories":[{{"owner":"a","name":"b"}}]}}"#).unwrap();
        std::env::set_var("GH_ISSUE_MONITOR_CONFIG", tmp.to_str().unwrap());
        let cfg = load_config().unwrap();
        assert_eq!(cfg.poll_interval, 30);
        assert_eq!(cfg.repositories.len(), 1);
        assert_eq!(cfg.repositories[0].owner, "a");
        std::env::remove_var("GH_ISSUE_MONITOR_CONFIG");
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn load_invalid_json_errors() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join("gh_issue_monitor_test_invalid.json");
        std::fs::write(&tmp, b"not json").unwrap();
        std::env::set_var("GH_ISSUE_MONITOR_CONFIG", tmp.to_str().unwrap());
        assert!(load_config().is_err());
        std::env::remove_var("GH_ISSUE_MONITOR_CONFIG");
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn resolve_token_gh_token_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("GH_TOKEN", "tok_gh");
        std::env::remove_var("GITHUB_TOKEN");
        let cfg = AppConfig::default();
        assert_eq!(resolve_token(&cfg).unwrap(), "tok_gh");
        std::env::remove_var("GH_TOKEN");
    }

    #[test]
    fn resolve_token_github_token_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GH_TOKEN");
        std::env::set_var("GITHUB_TOKEN", "tok_github");
        let cfg = AppConfig::default();
        assert_eq!(resolve_token(&cfg).unwrap(), "tok_github");
        std::env::remove_var("GITHUB_TOKEN");
    }

    #[test]
    fn resolve_token_config_value() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("GH_TOKEN");
        std::env::remove_var("GITHUB_TOKEN");
        let cfg = AppConfig { token: Some("tok_cfg".to_string()), ..AppConfig::default() };
        assert_eq!(resolve_token(&cfg).unwrap(), "tok_cfg");
    }
}
