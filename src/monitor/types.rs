use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RepoMonitorResult {
    pub owner: String,
    pub name: String,
    pub issues: Vec<IssueStatus>,
    pub rate_limit: Option<RateLimitInfo>,
    pub checked_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl RepoMonitorResult {
    pub fn repo_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubIssueRef {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: IssueState,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubIssueSummary {
    pub total: u32,
    pub completed: u32,
    pub percent_completed: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Milestone {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub due_on: Option<String>,
    pub progress_percentage: f64,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    Tracks,
    TrackedBy,
    Duplicate,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedIssueRef {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: IssueState,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueRelationship {
    pub relationship_type: RelationshipType,
    pub issue: TrackedIssueRef,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueStatus {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub labels: Vec<String>,
    pub linked_prs: Vec<LinkedPr>,
    pub matched_branches: Vec<MatchedBranch>,
    pub parent_issue_number: Option<u64>,
    pub sub_issues: Vec<SubIssueRef>,
    pub sub_issues_summary: Option<SubIssueSummary>,
    pub milestone: Option<Milestone>,
    pub relationships: Vec<IssueRelationship>,
}

impl IssueStatus {
    pub fn has_active_work(&self) -> bool {
        !self.linked_prs.is_empty() || !self.matched_branches.is_empty()
    }

    pub fn has_relationships(&self) -> bool {
        !self.relationships.is_empty()
    }
}

/// Tree entry used by the UI layer to render issues in a directory-tree style.
/// Not serialized; computed on the fly from a flat `Vec<IssueStatus>`.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    /// Index into the flat issues slice this entry refers to.
    pub issue_index: usize,
    /// 0 = root, 1 = direct child, 2 = grandchild, …
    pub depth: usize,
    /// Whether this entry is the last sibling among its parent's children.
    pub is_last_sibling: bool,
    /// For each ancestor level (closest first), whether that ancestor was the
    /// last sibling — used to decide whether to draw `│` or blank padding.
    pub parent_is_last: Vec<bool>,
}

impl TreeEntry {
    pub fn connector(&self) -> String {
        if self.depth == 0 {
            return String::new();
        }
        let mut prefix = String::new();
        for i in (0..(self.depth - 1)).rev() {
            let ancestor_is_last = self.parent_is_last.get(i).copied().unwrap_or(false);
            if ancestor_is_last {
                prefix.push_str("    ");
            } else {
                prefix.push_str("│   ");
            }
        }
        if self.is_last_sibling {
            prefix.push_str("└── ");
        } else {
            prefix.push_str("├── ");
        }
        prefix
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkedPr {
    pub number: u64,
    pub title: String,
    pub state: PrState,
    pub author: String,
    pub head_branch: String,
    pub url: String,
    pub link_type: LinkType,
    pub will_close: bool,
    pub is_draft: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

impl std::fmt::Display for PrState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrState::Open => write!(f, "open"),
            PrState::Merged => write!(f, "merged"),
            PrState::Closed => write!(f, "closed"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    CrossReference,
    Connected,
    Closing,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchedBranch {
    pub name: String,
    pub likely_author: Option<String>,
    pub has_pr: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    pub remaining: u32,
    pub limit: u32,
    pub reset_at: String,
    pub cost: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_issue(linked_prs: Vec<LinkedPr>, matched_branches: Vec<MatchedBranch>) -> IssueStatus {
        IssueStatus {
            number: 1,
            title: "Test issue".to_string(),
            url: "https://github.com/o/r/issues/1".to_string(),
            labels: vec![],
            linked_prs,
            matched_branches,
            parent_issue_number: None,
            sub_issues: vec![],
            sub_issues_summary: None,
            milestone: None,
            relationships: vec![],
        }
    }

    fn make_pr() -> LinkedPr {
        LinkedPr {
            number: 10,
            title: "Fix".to_string(),
            state: PrState::Open,
            author: "alice".to_string(),
            head_branch: "fix-1".to_string(),
            url: "https://github.com/o/r/pull/10".to_string(),
            link_type: LinkType::CrossReference,
            will_close: false,
            is_draft: false,
        }
    }

    fn make_branch() -> MatchedBranch {
        MatchedBranch {
            name: "alice/1-fix".to_string(),
            likely_author: Some("alice".to_string()),
            has_pr: false,
        }
    }

    #[test]
    fn has_active_work_no_prs_no_branches() {
        assert!(!make_issue(vec![], vec![]).has_active_work());
    }

    #[test]
    fn has_active_work_with_pr() {
        assert!(make_issue(vec![make_pr()], vec![]).has_active_work());
    }

    #[test]
    fn has_active_work_with_branch() {
        assert!(make_issue(vec![], vec![make_branch()]).has_active_work());
    }

    #[test]
    fn has_active_work_with_both() {
        assert!(make_issue(vec![make_pr()], vec![make_branch()]).has_active_work());
    }

    #[test]
    fn repo_name_format() {
        let r = RepoMonitorResult {
            owner: "foo".to_string(),
            name: "bar".to_string(),
            issues: vec![],
            rate_limit: None,
            checked_at: Utc::now(),
            error: None,
        };
        assert_eq!(r.repo_name(), "foo/bar");
    }

    #[test]
    fn pr_state_display() {
        assert_eq!(PrState::Open.to_string(), "open");
        assert_eq!(PrState::Merged.to_string(), "merged");
        assert_eq!(PrState::Closed.to_string(), "closed");
    }

    #[test]
    fn tree_entry_connector_depth0() {
        let e = TreeEntry {
            issue_index: 0,
            depth: 0,
            is_last_sibling: false,
            parent_is_last: vec![],
        };
        assert_eq!(e.connector(), "");
    }

    #[test]
    fn tree_entry_connector_depth1_not_last() {
        let e = TreeEntry {
            issue_index: 0,
            depth: 1,
            is_last_sibling: false,
            parent_is_last: vec![false],
        };
        assert_eq!(e.connector(), "├── ");
    }

    #[test]
    fn tree_entry_connector_depth1_last() {
        let e = TreeEntry {
            issue_index: 0,
            depth: 1,
            is_last_sibling: true,
            parent_is_last: vec![false],
        };
        assert_eq!(e.connector(), "└── ");
    }

    #[test]
    fn tree_entry_connector_depth2_parent_not_last() {
        let e = TreeEntry {
            issue_index: 0,
            depth: 2,
            is_last_sibling: false,
            parent_is_last: vec![false, false],
        };
        assert_eq!(e.connector(), "│   ├── ");
    }

    #[test]
    fn tree_entry_connector_depth2_parent_last() {
        let e = TreeEntry {
            issue_index: 0,
            depth: 2,
            is_last_sibling: true,
            parent_is_last: vec![true, false],
        };
        assert_eq!(e.connector(), "    └── ");
    }
}
