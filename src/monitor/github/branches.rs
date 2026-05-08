use anyhow::Result;
use octocrab::Octocrab;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::monitor::types::MatchedBranch;

/// Fetch all branches and return those matching any of the given issue numbers.
/// Returns a map from issue_number → Vec<MatchedBranch>.
pub async fn find_branches_for_issues(
    client: &Octocrab,
    owner: &str,
    name: &str,
    issue_numbers: &[u64],
    pr_head_branches: &HashSet<String>,
) -> Result<HashMap<u64, Vec<MatchedBranch>>> {
    if issue_numbers.is_empty() {
        return Ok(HashMap::new());
    }

    let branches = fetch_all_branches(client, owner, name).await?;

    let mut result: HashMap<u64, Vec<MatchedBranch>> = HashMap::new();

    for issue_num in issue_numbers {
        let patterns = build_patterns(*issue_num)?;
        let mut matched: Vec<MatchedBranch> = Vec::new();

        for branch_name in &branches {
            if patterns.iter().any(|re| re.is_match(branch_name)) {
                let likely_author = extract_likely_author(branch_name);
                let has_pr = pr_head_branches.contains(branch_name);
                matched.push(MatchedBranch {
                    name: branch_name.clone(),
                    likely_author,
                    has_pr,
                });
            }
        }

        if !matched.is_empty() {
            result.insert(*issue_num, matched);
        }
    }

    Ok(result)
}

async fn fetch_all_branches(client: &Octocrab, owner: &str, name: &str) -> Result<Vec<String>> {
    let mut branches = Vec::new();
    let mut page: u32 = 1;
    const MAX_PAGES: u32 = 10;

    loop {
        let url = format!("/repos/{owner}/{name}/branches?per_page=100&page={page}");
        let resp: serde_json::Value = client.get(&url, None::<&()>).await?;

        let arr = match resp.as_array() {
            Some(a) => a,
            None => break,
        };

        if arr.is_empty() {
            break;
        }

        for item in arr {
            if let Some(branch_name) = item.get("name").and_then(|v| v.as_str()) {
                branches.push(branch_name.to_string());
            }
        }

        page += 1;
        if page > MAX_PAGES {
            break;
        }
    }

    Ok(branches)
}

fn build_patterns(issue_num: u64) -> Result<Vec<Regex>> {
    let n = issue_num.to_string();
    let patterns = vec![
        format!(r"(?i)(?:feature|fix|bugfix|hotfix|issue|feat|chore|refactor)[/\-_]{n}(?:[/\-_]|$)"),
        format!(r"^{n}[/\-_]"),
        format!(r"^[^/]+/{n}(?:[/\-_]|$)"),
        format!(r"[/\-_]{n}$"),
    ];
    patterns
        .into_iter()
        .map(|p| Regex::new(&p).map_err(Into::into))
        .collect()
}

fn extract_likely_author(branch_name: &str) -> Option<String> {
    let parts: Vec<&str> = branch_name.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() {
        let prefix = parts[0];
        if prefix.chars().any(|c| c.is_alphabetic()) {
            return Some(prefix.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(branch: &str, issue: u64) -> bool {
        let patterns = build_patterns(issue).unwrap();
        patterns.iter().any(|re| re.is_match(branch))
    }

    #[test]
    fn test_branch_patterns() {
        assert!(matches("feature/123-fix-login", 123));
        assert!(matches("fix/123", 123));
        assert!(matches("issue-123", 123));
        assert!(matches("alice/123-login-fix", 123));
        assert!(matches("123-some-feature", 123));
        assert!(matches("hotfix/456-crash", 456));
        assert!(matches("main-123", 123));

        assert!(!matches("feature/1234-fix", 123));
        assert!(!matches("main", 123));
        assert!(!matches("develop", 123));
    }

    #[test]
    fn author_standard() {
        assert_eq!(extract_likely_author("alice/123-fix"), Some("alice".to_string()));
    }

    #[test]
    fn author_keyword_prefix() {
        assert_eq!(extract_likely_author("feature/123"), Some("feature".to_string()));
    }

    #[test]
    fn author_no_slash() {
        assert_eq!(extract_likely_author("123-fix"), None);
    }

    #[test]
    fn author_numeric_prefix() {
        assert_eq!(extract_likely_author("123/fix"), None);
    }

    #[test]
    fn author_empty_prefix() {
        assert_eq!(extract_likely_author("/something"), None);
    }
}
