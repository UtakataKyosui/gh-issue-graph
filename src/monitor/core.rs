use anyhow::Result;
use chrono::Utc;
use octocrab::Octocrab;
use std::collections::{HashMap, HashSet};

use crate::monitor::config::RepoConfig;
use crate::monitor::github::{branches, graphql};
use crate::monitor::types::{IssueStatus, PrState, RepoMonitorResult, TreeEntry};

/// Filter options for narrowing down issues.
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    pub labels: Vec<String>,
    pub keywords: Vec<String>,
    pub milestone: Option<String>,
}

impl FilterOptions {
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty() && self.keywords.is_empty() && self.milestone.is_none()
    }

    pub fn matches_keywords(&self, issue: &IssueStatus) -> bool {
        if self.keywords.is_empty() {
            return true;
        }
        let title_lower = issue.title.to_lowercase();
        self.keywords
            .iter()
            .all(|kw| title_lower.contains(&kw.to_lowercase()))
    }

    pub fn matches_milestone(&self, issue: &IssueStatus) -> bool {
        match &self.milestone {
            None => true,
            Some(ms) => issue
                .milestone
                .as_ref()
                .map(|m| m.title == *ms)
                .unwrap_or(false),
        }
    }

    pub fn matches(&self, issue: &IssueStatus) -> bool {
        self.matches_keywords(issue) && self.matches_milestone(issue)
    }
}

pub fn build_issue_tree(issues: &[IssueStatus]) -> Vec<TreeEntry> {
    if issues.is_empty() {
        return vec![];
    }

    let number_to_idx: HashMap<u64, usize> = issues
        .iter()
        .enumerate()
        .map(|(i, issue)| (issue.number, i))
        .collect();

    let child_numbers: HashSet<u64> = issues
        .iter()
        .filter_map(|i| {
            i.parent_issue_number
                .filter(|parent| number_to_idx.contains_key(parent))
                .map(|_| i.number)
        })
        .collect();

    let mut children_of: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, issue) in issues.iter().enumerate() {
        if let Some(parent_num) = issue.parent_issue_number {
            if number_to_idx.contains_key(&parent_num) {
                children_of.entry(parent_num).or_default().push(idx);
            }
        }
    }

    let roots: Vec<usize> = issues
        .iter()
        .enumerate()
        .filter(|(_, i)| !child_numbers.contains(&i.number))
        .map(|(idx, _)| idx)
        .collect();

    let mut entries = Vec::new();
    for (pos, &root_idx) in roots.iter().enumerate() {
        let is_last = pos == roots.len() - 1;
        add_tree_entries(
            issues,
            root_idx,
            0,
            is_last,
            vec![],
            &children_of,
            &mut entries,
        );
    }

    entries
}

fn add_tree_entries(
    issues: &[IssueStatus],
    issue_idx: usize,
    depth: usize,
    is_last_sibling: bool,
    ancestor_chain: Vec<bool>,
    children_of: &HashMap<u64, Vec<usize>>,
    entries: &mut Vec<TreeEntry>,
) {
    let issue_number = issues[issue_idx].number;
    entries.push(TreeEntry {
        issue_index: issue_idx,
        depth,
        is_last_sibling,
        parent_is_last: ancestor_chain.clone(),
    });

    if let Some(children) = children_of.get(&issue_number) {
        let mut child_chain = vec![is_last_sibling];
        child_chain.extend_from_slice(&ancestor_chain);
        for (pos, &child_idx) in children.iter().enumerate() {
            let child_is_last = pos == children.len() - 1;
            add_tree_entries(
                issues,
                child_idx,
                depth + 1,
                child_is_last,
                child_chain.clone(),
                children_of,
                entries,
            );
        }
    }
}

pub async fn fetch_repo(
    client: &Octocrab,
    repo: &RepoConfig,
    issue_filter: Option<u64>,
    filter: &FilterOptions,
) -> RepoMonitorResult {
    let result = fetch_repo_inner(client, repo, issue_filter, filter).await;
    match result {
        Ok(mut r) => {
            r.checked_at = Utc::now();
            r
        }
        Err(e) => RepoMonitorResult {
            owner: repo.owner.clone(),
            name: repo.name.clone(),
            issues: vec![],
            rate_limit: None,
            checked_at: Utc::now(),
            error: Some(e.to_string()),
        },
    }
}

async fn fetch_repo_inner(
    client: &Octocrab,
    repo: &RepoConfig,
    issue_filter: Option<u64>,
    filter: &FilterOptions,
) -> Result<RepoMonitorResult> {
    let mut effective_labels: Vec<String> = repo.labels.clone().unwrap_or_default();
    for l in &filter.labels {
        if !effective_labels.contains(l) {
            effective_labels.push(l.clone());
        }
    }
    let label_arg = if effective_labels.is_empty() {
        None
    } else {
        Some(effective_labels.as_slice())
    };

    let fetch = graphql::fetch_issues(client, &repo.owner, &repo.name, label_arg).await?;

    let mut issues = fetch.issues;

    if let Some(num) = issue_filter {
        issues.retain(|i| i.number == num);
    }

    for issue in &mut issues {
        issue.linked_prs.retain(|pr| pr.state == PrState::Open);
    }

    issues.retain(|i| filter.matches(i));

    let pr_head_branches: HashSet<String> = issues
        .iter()
        .flat_map(|i| i.linked_prs.iter().map(|pr| pr.head_branch.clone()))
        .collect();

    let issue_numbers: Vec<u64> = issues.iter().map(|i| i.number).collect();
    let branch_map = branches::find_branches_for_issues(
        client,
        &repo.owner,
        &repo.name,
        &issue_numbers,
        &pr_head_branches,
    )
    .await
    .unwrap_or_default();

    for issue in &mut issues {
        if let Some(branches) = branch_map.get(&issue.number) {
            issue.matched_branches = branches.clone();
        }
    }

    Ok(RepoMonitorResult {
        owner: repo.owner.clone(),
        name: repo.name.clone(),
        issues,
        rate_limit: fetch.rate_limit,
        checked_at: Utc::now(),
        error: None,
    })
}

pub async fn fetch_all(
    client: &Octocrab,
    repos: &[RepoConfig],
    issue_filter: Option<u64>,
    filter: &FilterOptions,
) -> Result<Vec<RepoMonitorResult>> {
    let mut handles = Vec::new();
    for repo in repos {
        let client = client.clone();
        let repo = repo.clone();
        let filter = filter.clone();
        handles.push(tokio::spawn(async move {
            fetch_repo(&client, &repo, issue_filter, &filter).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }
    Ok(results)
}

pub fn print_results(results: &[RepoMonitorResult]) {
    for result in results {
        println!(
            "\n[{}] checked at {}",
            result.repo_name(),
            result.checked_at.format("%Y-%m-%d %H:%M:%S UTC")
        );

        if let Some(err) = &result.error {
            eprintln!("  Error: {err}");
            continue;
        }

        if result.issues.is_empty() {
            println!("  No open issues found.");
            continue;
        }

        for issue in &result.issues {
            let active = if issue.has_active_work() { "●" } else { "○" };
            println!("\n  {active} Issue #{}: {}", issue.number, issue.title);

            if let Some(ms) = &issue.milestone {
                println!("    Milestone: {} ({:.0}%)", ms.title, ms.progress_percentage);
            }

            if !issue.sub_issues.is_empty() {
                if let Some(summary) = &issue.sub_issues_summary {
                    println!("    Sub-issues: {}/{} completed", summary.completed, summary.total);
                }
            }

            if !issue.relationships.is_empty() {
                for rel in &issue.relationships {
                    use crate::monitor::types::RelationshipType;
                    let (arrow, label) = match rel.relationship_type {
                        RelationshipType::Tracks => ("→", "tracks"),
                        RelationshipType::TrackedBy => ("←", "tracked by"),
                        RelationshipType::Duplicate => ("≡", "duplicate of"),
                    };
                    println!("    {arrow} {label} #{}: {}", rel.issue.number, rel.issue.title);
                }
            }

            if issue.linked_prs.is_empty() && issue.matched_branches.is_empty() {
                println!("    (no linked PRs or branches)");
                continue;
            }

            for pr in &issue.linked_prs {
                let draft = if pr.is_draft { ", draft" } else { "" };
                let close = if pr.will_close { " [will close]" } else { "" };
                let state_str = match pr.state {
                    PrState::Open => "open",
                    PrState::Merged => "merged",
                    PrState::Closed => "closed",
                };
                println!(
                    "    PR #{} by @{} ({state_str}{draft}) -- branch: {}{}",
                    pr.number, pr.author, pr.head_branch, close
                );
            }

            for branch in &issue.matched_branches {
                if branch.has_pr {
                    continue;
                }
                let author = branch
                    .likely_author
                    .as_deref()
                    .map(|a| format!(" (likely @{a})"))
                    .unwrap_or_default();
                println!("    Branch: {}{} (no PR)", branch.name, author);
            }
        }

        if let Some(rl) = &result.rate_limit {
            println!("\n  API rate limit: {}/{} remaining", rl.remaining, rl.limit);
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::types::{IssueStatus, Milestone};

    fn make_issue(number: u64, title: &str) -> IssueStatus {
        IssueStatus {
            number,
            title: title.to_string(),
            url: String::new(),
            labels: vec![],
            linked_prs: vec![],
            matched_branches: vec![],
            parent_issue_number: None,
            sub_issues: vec![],
            sub_issues_summary: None,
            milestone: None,
            relationships: vec![],
        }
    }

    fn make_issue_with_parent(number: u64, parent: u64) -> IssueStatus {
        let mut i = make_issue(number, &format!("Issue {number}"));
        i.parent_issue_number = Some(parent);
        i
    }

    fn make_issue_with_milestone(number: u64, ms_title: &str) -> IssueStatus {
        let mut i = make_issue(number, &format!("Issue {number}"));
        i.milestone = Some(Milestone {
            number: 1,
            title: ms_title.to_string(),
            state: "OPEN".to_string(),
            due_on: None,
            progress_percentage: 0.0,
            url: String::new(),
        });
        i
    }

    #[test]
    fn filter_default_is_empty() {
        assert!(FilterOptions::default().is_empty());
    }

    #[test]
    fn filter_with_labels_not_empty() {
        let f = FilterOptions { labels: vec!["bug".to_string()], keywords: vec![], milestone: None };
        assert!(!f.is_empty());
    }

    #[test]
    fn filter_with_keywords_not_empty() {
        let f = FilterOptions { labels: vec![], keywords: vec!["fix".to_string()], milestone: None };
        assert!(!f.is_empty());
    }

    #[test]
    fn filter_with_milestone_not_empty() {
        let f = FilterOptions { labels: vec![], keywords: vec![], milestone: Some("v1.0".to_string()) };
        assert!(!f.is_empty());
    }

    #[test]
    fn matches_keywords_empty_matches_all() {
        let f = FilterOptions::default();
        assert!(f.matches_keywords(&make_issue(1, "anything")));
    }

    #[test]
    fn matches_keywords_single_hit_case_insensitive() {
        let f = FilterOptions { labels: vec![], keywords: vec!["FIX".to_string()], milestone: None };
        assert!(f.matches_keywords(&make_issue(1, "fix login bug")));
    }

    #[test]
    fn matches_keywords_single_miss() {
        let f = FilterOptions { labels: vec![], keywords: vec!["auth".to_string()], milestone: None };
        assert!(!f.matches_keywords(&make_issue(1, "fix login bug")));
    }

    #[test]
    fn matches_keywords_and_semantics_both_present() {
        let f = FilterOptions {
            labels: vec![],
            keywords: vec!["fix".to_string(), "login".to_string()],
            milestone: None,
        };
        assert!(f.matches_keywords(&make_issue(1, "fix login bug")));
    }

    #[test]
    fn matches_keywords_and_semantics_one_missing() {
        let f = FilterOptions {
            labels: vec![],
            keywords: vec!["fix".to_string(), "auth".to_string()],
            milestone: None,
        };
        assert!(!f.matches_keywords(&make_issue(1, "fix login bug")));
    }

    #[test]
    fn matches_milestone_none_matches_all() {
        let f = FilterOptions::default();
        assert!(f.matches_milestone(&make_issue(1, "anything")));
        assert!(f.matches_milestone(&make_issue_with_milestone(1, "v1.0")));
    }

    #[test]
    fn matches_milestone_hit() {
        let f = FilterOptions { labels: vec![], keywords: vec![], milestone: Some("v1.0".to_string()) };
        assert!(f.matches_milestone(&make_issue_with_milestone(1, "v1.0")));
    }

    #[test]
    fn matches_milestone_miss_wrong_title() {
        let f = FilterOptions { labels: vec![], keywords: vec![], milestone: Some("v1.0".to_string()) };
        assert!(!f.matches_milestone(&make_issue_with_milestone(1, "v2.0")));
    }

    #[test]
    fn matches_milestone_miss_no_milestone() {
        let f = FilterOptions { labels: vec![], keywords: vec![], milestone: Some("v1.0".to_string()) };
        assert!(!f.matches_milestone(&make_issue(1, "no milestone")));
    }

    #[test]
    fn tree_all_roots() {
        let issues = vec![make_issue(1, "A"), make_issue(2, "B"), make_issue(3, "C")];
        let tree = build_issue_tree(&issues);
        assert_eq!(tree.len(), 3);
        assert!(tree.iter().all(|e| e.depth == 0));
        assert!(!tree[0].is_last_sibling);
        assert!(!tree[1].is_last_sibling);
        assert!(tree[2].is_last_sibling);
    }

    #[test]
    fn tree_one_parent_two_children() {
        let issues = vec![
            make_issue(1, "Parent"),
            make_issue_with_parent(2, 1),
            make_issue_with_parent(3, 1),
        ];
        let tree = build_issue_tree(&issues);
        assert_eq!(tree.len(), 3);
        assert_eq!(tree[0].issue_index, 0);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[1].issue_index, 1);
        assert_eq!(tree[1].depth, 1);
        assert!(!tree[1].is_last_sibling);
        assert_eq!(tree[2].issue_index, 2);
        assert_eq!(tree[2].depth, 1);
        assert!(tree[2].is_last_sibling);
    }

    #[test]
    fn tree_child_not_duplicated_as_root() {
        let issues = vec![
            make_issue(1, "Parent"),
            make_issue_with_parent(2, 1),
        ];
        let tree = build_issue_tree(&issues);
        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].depth, 0);
        assert_eq!(tree[1].depth, 1);
    }

    #[test]
    fn tree_parent_not_in_list_treated_as_root() {
        let mut issue = make_issue(5, "Orphan");
        issue.parent_issue_number = Some(99);
        let issues = vec![issue];
        let tree = build_issue_tree(&issues);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].depth, 0);
    }

    #[test]
    fn tree_empty_issues() {
        let tree = build_issue_tree(&[]);
        assert!(tree.is_empty());
    }
}
