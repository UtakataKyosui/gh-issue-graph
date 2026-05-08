use anyhow::Result;
use octocrab::Octocrab;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::monitor::types::{
    IssueRelationship, IssueState, IssueStatus, LinkedPr, LinkType, Milestone, PrState,
    RateLimitInfo, RelationshipType, SubIssueRef, SubIssueSummary, TrackedIssueRef,
};

static BASIC_QUERY_REPOS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn basic_query_repos() -> &'static Mutex<HashSet<String>> {
    BASIC_QUERY_REPOS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn is_schema_compat_error(e: &anyhow::Error) -> bool {
    e.to_string().contains("doesn't exist on type")
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    repository: Option<RepositoryData>,
    #[serde(rename = "rateLimit")]
    rate_limit: Option<RateLimitData>,
}

#[derive(Debug, Deserialize)]
struct RepositoryData {
    issues: IssuesConnection,
}

#[derive(Debug, Deserialize)]
struct IssuesConnection {
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
    nodes: Vec<IssueNode>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IssueNode {
    number: u64,
    title: String,
    url: String,
    labels: LabelConnection,
    #[serde(rename = "timelineItems")]
    timeline_items: TimelineConnection,
    #[serde(rename = "parentIssue", default)]
    parent_issue: Option<IssueRefNode>,
    #[serde(rename = "subIssues", default)]
    sub_issues: Option<SubIssuesConnection>,
    #[serde(rename = "subIssuesSummary", default)]
    sub_issues_summary: Option<SubIssuesSummaryNode>,
    #[serde(default)]
    milestone: Option<MilestoneNode>,
    #[serde(rename = "trackedIssues", default)]
    tracked_issues: Option<TrackedIssuesConnection>,
    #[serde(rename = "trackedInIssues", default)]
    tracked_in_issues: Option<TrackedIssuesConnection>,
}

#[derive(Debug, Deserialize)]
struct IssueRefNode {
    number: u64,
    title: String,
    url: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct SubIssuesConnection {
    nodes: Vec<IssueRefNode>,
}

#[derive(Debug, Deserialize)]
struct SubIssuesSummaryNode {
    total: u32,
    completed: u32,
    #[serde(rename = "percentCompleted")]
    percent_completed: f64,
}

#[derive(Debug, Deserialize)]
struct MilestoneNode {
    number: u64,
    title: String,
    state: String,
    #[serde(rename = "dueOn")]
    due_on: Option<String>,
    #[serde(rename = "progressPercentage")]
    progress_percentage: f64,
    url: String,
}

#[derive(Debug, Deserialize)]
struct TrackedIssuesConnection {
    nodes: Vec<IssueRefNode>,
}

#[derive(Debug, Deserialize)]
struct LabelConnection {
    nodes: Vec<LabelNode>,
}

#[derive(Debug, Deserialize)]
struct LabelNode {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TimelineConnection {
    nodes: Vec<TimelineItemNode>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum TimelineItemNode {
    CrossReferencedEvent {
        #[serde(rename = "willCloseTarget")]
        will_close_target: bool,
        source: ReferencedSubject,
    },
    ConnectedEvent {
        subject: ReferencedSubject,
    },
    ClosedEvent {
        closer: Option<CloserSubject>,
    },
    MarkedAsDuplicateEvent {
        canonical: Option<CanonicalSubject>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum ReferencedSubject {
    PullRequest(PrNode),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum CloserSubject {
    PullRequest(PrNode),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__typename")]
enum CanonicalSubject {
    Issue(IssueRefNode),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct PrNode {
    number: u64,
    title: String,
    state: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    url: String,
    author: Option<AuthorNode>,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
}

#[derive(Debug, Deserialize)]
struct AuthorNode {
    login: String,
}

#[derive(Debug, Deserialize)]
struct RateLimitData {
    remaining: u32,
    limit: u32,
    #[serde(rename = "resetAt")]
    reset_at: String,
    cost: u32,
}

const QUERY_FULL: &str = r#"
query($owner: String!, $name: String!, $labels: [String!], $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(
      first: 50
      after: $cursor
      states: OPEN
      labels: $labels
      orderBy: { field: UPDATED_AT, direction: DESC }
    ) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title url
        labels(first: 10) { nodes { name } }
        milestone { number title state dueOn progressPercentage url }
        parentIssue { number title url state }
        subIssues(first: 10) { nodes { number title url state } }
        subIssuesSummary { total completed percentCompleted }
        trackedIssues(first: 10) { nodes { number title url state } }
        trackedInIssues(first: 10) { nodes { number title url state } }
        timelineItems(
          first: 30
          itemTypes: [CROSS_REFERENCED_EVENT, CONNECTED_EVENT, CLOSED_EVENT, MARKED_AS_DUPLICATE_EVENT]
        ) {
          nodes {
            __typename
            ... on CrossReferencedEvent {
              willCloseTarget
              source {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on ConnectedEvent {
              subject {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on ClosedEvent {
              closer {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on MarkedAsDuplicateEvent {
              canonical {
                __typename
                ... on Issue { number title url state }
              }
            }
          }
        }
      }
    }
  }
  rateLimit { remaining limit resetAt cost }
}
"#;

const QUERY_BASIC: &str = r#"
query($owner: String!, $name: String!, $labels: [String!], $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(
      first: 50
      after: $cursor
      states: OPEN
      labels: $labels
      orderBy: { field: UPDATED_AT, direction: DESC }
    ) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title url
        labels(first: 10) { nodes { name } }
        milestone { number title state dueOn progressPercentage url }
        trackedIssues(first: 10) { nodes { number title url state } }
        trackedInIssues(first: 10) { nodes { number title url state } }
        timelineItems(
          first: 30
          itemTypes: [CROSS_REFERENCED_EVENT, CONNECTED_EVENT, CLOSED_EVENT, MARKED_AS_DUPLICATE_EVENT]
        ) {
          nodes {
            __typename
            ... on CrossReferencedEvent {
              willCloseTarget
              source {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on ConnectedEvent {
              subject {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on ClosedEvent {
              closer {
                __typename
                ... on PullRequest {
                  number title state isDraft url
                  author { login }
                  headRefName
                }
              }
            }
            ... on MarkedAsDuplicateEvent {
              canonical {
                __typename
                ... on Issue { number title url state }
              }
            }
          }
        }
      }
    }
  }
  rateLimit { remaining limit resetAt cost }
}
"#;

pub struct FetchResult {
    pub issues: Vec<IssueStatus>,
    pub rate_limit: Option<RateLimitInfo>,
}

pub async fn fetch_issues(
    client: &Octocrab,
    owner: &str,
    name: &str,
    labels: Option<&[String]>,
) -> Result<FetchResult> {
    let repo_key = format!("{owner}/{name}");
    let already_basic = basic_query_repos().lock().unwrap().contains(&repo_key);

    let query = if already_basic { QUERY_BASIC } else { QUERY_FULL };

    match fetch_with_query(client, owner, name, labels, query).await {
        Ok(r) => Ok(r),
        Err(e) if !already_basic && is_schema_compat_error(&e) => {
            basic_query_repos().lock().unwrap().insert(repo_key);
            fetch_with_query(client, owner, name, labels, QUERY_BASIC).await
        }
        Err(e) => Err(e),
    }
}

async fn fetch_with_query(
    client: &Octocrab,
    owner: &str,
    name: &str,
    labels: Option<&[String]>,
    query: &str,
) -> Result<FetchResult> {
    let mut all_issues: Vec<IssueStatus> = Vec::new();
    let mut cursor: Option<String> = None;
    let mut last_rate_limit: Option<RateLimitInfo> = None;

    loop {
        let vars = serde_json::json!({
            "owner": owner,
            "name": name,
            "labels": labels,
            "cursor": cursor,
        });

        let resp: serde_json::Value = client
            .graphql(&serde_json::json!({ "query": query, "variables": vars }))
            .await?;

        if let Some(errors) = resp.get("errors") {
            let is_not_found = errors
                .as_array()
                .map(|arr| {
                    arr.iter().any(|e| {
                        e.get("message")
                            .and_then(|m| m.as_str())
                            .map(|m| m.contains("Could not resolve"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if is_not_found {
                anyhow::bail!("Repository {owner}/{name} not found (check name and permissions)");
            }
            anyhow::bail!("GraphQL errors for {owner}/{name}: {errors}");
        }

        let data: QueryResponse = serde_json::from_value(
            resp.get("data")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        )?;

        if let Some(rl) = data.rate_limit {
            last_rate_limit = Some(RateLimitInfo {
                remaining: rl.remaining,
                limit: rl.limit,
                reset_at: rl.reset_at,
                cost: rl.cost,
            });
        }

        let repo = match data.repository {
            Some(r) => r,
            None => break,
        };

        for issue in repo.issues.nodes {
            let labels_vec: Vec<String> =
                issue.labels.nodes.into_iter().map(|l| l.name).collect();

            let mut linked_prs: Vec<LinkedPr> = Vec::new();
            let mut seen_pr_numbers = std::collections::HashSet::new();
            let mut relationships: Vec<IssueRelationship> = Vec::new();

            for item in issue.timeline_items.nodes {
                match item {
                    TimelineItemNode::CrossReferencedEvent {
                        will_close_target,
                        source: ReferencedSubject::PullRequest(pr),
                    } => {
                        if seen_pr_numbers.insert(pr.number) {
                            linked_prs.push(pr_node_to_linked_pr(
                                pr,
                                LinkType::CrossReference,
                                will_close_target,
                            ));
                        }
                    }
                    TimelineItemNode::ConnectedEvent {
                        subject: ReferencedSubject::PullRequest(pr),
                    } => {
                        if seen_pr_numbers.insert(pr.number) {
                            linked_prs.push(pr_node_to_linked_pr(pr, LinkType::Connected, false));
                        }
                    }
                    TimelineItemNode::ClosedEvent {
                        closer: Some(CloserSubject::PullRequest(pr)),
                    } => {
                        if seen_pr_numbers.insert(pr.number) {
                            linked_prs.push(pr_node_to_linked_pr(pr, LinkType::Closing, true));
                        }
                    }
                    TimelineItemNode::MarkedAsDuplicateEvent {
                        canonical: Some(CanonicalSubject::Issue(canonical_issue)),
                    } => {
                        relationships.push(IssueRelationship {
                            relationship_type: RelationshipType::Duplicate,
                            issue: issue_ref_to_tracked(canonical_issue),
                        });
                    }
                    _ => {}
                }
            }

            if let Some(conn) = issue.tracked_issues {
                for tracked in conn.nodes {
                    relationships.push(IssueRelationship {
                        relationship_type: RelationshipType::Tracks,
                        issue: issue_ref_to_tracked(tracked),
                    });
                }
            }

            if let Some(conn) = issue.tracked_in_issues {
                for tracker in conn.nodes {
                    relationships.push(IssueRelationship {
                        relationship_type: RelationshipType::TrackedBy,
                        issue: issue_ref_to_tracked(tracker),
                    });
                }
            }

            let sub_issues = issue
                .sub_issues
                .map(|c| {
                    c.nodes
                        .into_iter()
                        .map(|n| SubIssueRef {
                            number: n.number,
                            title: n.title,
                            url: n.url,
                            state: parse_issue_state(&n.state),
                        })
                        .collect()
                })
                .unwrap_or_default();

            let sub_issues_summary = issue.sub_issues_summary.map(|s| SubIssueSummary {
                total: s.total,
                completed: s.completed,
                percent_completed: s.percent_completed,
            });

            let milestone = issue.milestone.map(|m| Milestone {
                number: m.number,
                title: m.title,
                state: m.state,
                due_on: m.due_on,
                progress_percentage: m.progress_percentage,
                url: m.url,
            });

            let parent_issue_number = issue.parent_issue.map(|p| p.number);

            all_issues.push(IssueStatus {
                number: issue.number,
                title: issue.title,
                url: issue.url,
                labels: labels_vec,
                linked_prs,
                matched_branches: vec![],
                parent_issue_number,
                sub_issues,
                sub_issues_summary,
                milestone,
                relationships,
            });
        }

        if repo.issues.page_info.has_next_page {
            cursor = repo.issues.page_info.end_cursor;
        } else {
            break;
        }
    }

    Ok(FetchResult {
        issues: all_issues,
        rate_limit: last_rate_limit,
    })
}

fn parse_issue_state(state: &str) -> IssueState {
    match state {
        "CLOSED" => IssueState::Closed,
        _ => IssueState::Open,
    }
}

fn issue_ref_to_tracked(node: IssueRefNode) -> TrackedIssueRef {
    TrackedIssueRef {
        number: node.number,
        title: node.title,
        url: node.url,
        state: parse_issue_state(&node.state),
    }
}

fn pr_node_to_linked_pr(pr: PrNode, link_type: LinkType, will_close: bool) -> LinkedPr {
    let state = match pr.state.as_str() {
        "MERGED" => PrState::Merged,
        "CLOSED" => PrState::Closed,
        _ => PrState::Open,
    };
    LinkedPr {
        number: pr.number,
        title: pr.title,
        state,
        author: pr.author.map(|a| a.login).unwrap_or_else(|| "ghost".to_string()),
        head_branch: pr.head_ref_name,
        url: pr.url,
        link_type,
        will_close,
        is_draft: pr.is_draft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_compat_error_detected() {
        let msg = "GraphQL errors for owner/repo: [{{\"message\":\"Field 'parentIssue' doesn't exist on type 'Issue'\"}}]";
        let e = anyhow::anyhow!("{}", msg);
        assert!(is_schema_compat_error(&e));
    }

    #[test]
    fn schema_compat_error_not_triggered_for_other_errors() {
        let e = anyhow::anyhow!("GraphQL errors for owner/repo: Could not resolve to a Repository");
        assert!(!is_schema_compat_error(&e));
    }

    fn make_pr_node(state: &str, author: Option<&str>, is_draft: bool) -> PrNode {
        PrNode {
            number: 42,
            title: "Fix bug".to_string(),
            state: state.to_string(),
            is_draft,
            url: "https://github.com/o/r/pull/42".to_string(),
            author: author.map(|l| AuthorNode { login: l.to_string() }),
            head_ref_name: "fix-42".to_string(),
        }
    }

    fn make_issue_json(timeline_nodes: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "repository": {
                "issues": {
                    "pageInfo": { "hasNextPage": false, "endCursor": null },
                    "nodes": [{
                        "number": 1,
                        "title": "Test issue",
                        "url": "https://github.com/o/r/issues/1",
                        "labels": { "nodes": [] },
                        "milestone": null,
                        "parentIssue": null,
                        "subIssues": { "nodes": [] },
                        "subIssuesSummary": null,
                        "trackedIssues": { "nodes": [] },
                        "trackedInIssues": { "nodes": [] },
                        "timelineItems": { "nodes": timeline_nodes }
                    }]
                }
            },
            "rateLimit": null
        })
    }

    #[test]
    fn pr_state_open() {
        let pr = pr_node_to_linked_pr(make_pr_node("OPEN", Some("alice"), false), LinkType::CrossReference, false);
        assert_eq!(pr.state, PrState::Open);
    }

    #[test]
    fn pr_state_merged() {
        let pr = pr_node_to_linked_pr(make_pr_node("MERGED", Some("alice"), false), LinkType::CrossReference, false);
        assert_eq!(pr.state, PrState::Merged);
    }

    #[test]
    fn pr_state_closed() {
        let pr = pr_node_to_linked_pr(make_pr_node("CLOSED", Some("alice"), false), LinkType::CrossReference, false);
        assert_eq!(pr.state, PrState::Closed);
    }

    #[test]
    fn pr_state_unknown_defaults_to_open() {
        let pr = pr_node_to_linked_pr(make_pr_node("NONSENSE", Some("alice"), false), LinkType::CrossReference, false);
        assert_eq!(pr.state, PrState::Open);
    }

    #[test]
    fn pr_author_present() {
        let pr = pr_node_to_linked_pr(make_pr_node("OPEN", Some("alice"), false), LinkType::CrossReference, false);
        assert_eq!(pr.author, "alice");
    }

    #[test]
    fn pr_author_none_fallback_ghost() {
        let pr = pr_node_to_linked_pr(make_pr_node("OPEN", None, false), LinkType::CrossReference, false);
        assert_eq!(pr.author, "ghost");
    }

    #[test]
    fn deser_cross_referenced_event() {
        let json = make_issue_json(serde_json::json!([{
            "__typename": "CrossReferencedEvent",
            "willCloseTarget": true,
            "source": {
                "__typename": "PullRequest",
                "number": 10, "title": "Fix #1", "state": "OPEN", "isDraft": false,
                "url": "https://github.com/o/r/pull/10",
                "author": { "login": "alice" }, "headRefName": "fix-1"
            }
        }]));
        let resp: QueryResponse = serde_json::from_value(json).unwrap();
        let item = &resp.repository.unwrap().issues.nodes[0].timeline_items.nodes[0];
        assert!(matches!(item, TimelineItemNode::CrossReferencedEvent { will_close_target: true, .. }));
    }

    #[test]
    fn deser_connected_event() {
        let json = make_issue_json(serde_json::json!([{
            "__typename": "ConnectedEvent",
            "subject": {
                "__typename": "PullRequest",
                "number": 11, "title": "PR", "state": "OPEN", "isDraft": false,
                "url": "https://github.com/o/r/pull/11",
                "author": { "login": "bob" }, "headRefName": "feat-2"
            }
        }]));
        let resp: QueryResponse = serde_json::from_value(json).unwrap();
        let item = &resp.repository.unwrap().issues.nodes[0].timeline_items.nodes[0];
        assert!(matches!(item, TimelineItemNode::ConnectedEvent { .. }));
    }

    #[test]
    fn deser_closed_event() {
        let json = make_issue_json(serde_json::json!([{
            "__typename": "ClosedEvent",
            "closer": {
                "__typename": "PullRequest",
                "number": 12, "title": "Closes #3", "state": "MERGED", "isDraft": false,
                "url": "https://github.com/o/r/pull/12",
                "author": null, "headRefName": "close-3"
            }
        }]));
        let resp: QueryResponse = serde_json::from_value(json).unwrap();
        let item = &resp.repository.unwrap().issues.nodes[0].timeline_items.nodes[0];
        assert!(matches!(item, TimelineItemNode::ClosedEvent { .. }));
    }

    #[test]
    fn deser_marked_as_duplicate_event() {
        let json = make_issue_json(serde_json::json!([{
            "__typename": "MarkedAsDuplicateEvent",
            "canonical": {
                "__typename": "Issue",
                "number": 50, "title": "Original issue",
                "url": "https://github.com/o/r/issues/50", "state": "OPEN"
            }
        }]));
        let resp: QueryResponse = serde_json::from_value(json).unwrap();
        let item = &resp.repository.unwrap().issues.nodes[0].timeline_items.nodes[0];
        assert!(matches!(item, TimelineItemNode::MarkedAsDuplicateEvent { .. }));
    }

    #[test]
    fn deser_unknown_event() {
        let json = make_issue_json(serde_json::json!([{
            "__typename": "LabeledEvent",
            "label": { "name": "bug" }
        }]));
        let resp: QueryResponse = serde_json::from_value(json).unwrap();
        let item = &resp.repository.unwrap().issues.nodes[0].timeline_items.nodes[0];
        assert!(matches!(item, TimelineItemNode::Unknown));
    }
}
