/// Issue/PR 一覧の基本情報 + PR の closingIssuesReferences を取得するクエリ
pub const QUERY_ISSUES_PAGE: &str = r#"
query($owner: String!, $name: String!, $issueAfter: String, $prAfter: String) {
  repository(owner: $owner, name: $name) {
    issues(first: 50, after: $issueAfter, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        id number title state url body createdAt updatedAt
        labels(first: 20) { nodes { name } }
        milestone { title }
        assignees(first: 10) { nodes { login } }
      }
    }
    pullRequests(first: 50, after: $prAfter, orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes {
        id number title state url body createdAt updatedAt isDraft
        labels(first: 20) { nodes { name } }
        milestone { title }
        assignees(first: 10) { nodes { login } }
        closingIssuesReferences(first: 20) {
          nodes {
            number
            repository { owner { login } name }
          }
        }
      }
    }
  }
}
"#;

/// Sub-Issue 取得クエリ (GraphQL-Features: sub_issues ヘッダが必要)
pub const QUERY_SUB_ISSUES: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    issue(number: $number) {
      id
      parent {
        number
        repository { owner { login } name }
      }
      subIssues(first: 50) {
        nodes {
          number
          title
          state
          repository { owner { login } name }
        }
      }
    }
  }
}
"#;

/// タイムライン cross-reference / connected event 取得クエリ
pub const QUERY_TIMELINE: &str = r#"
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    issueOrPullRequest(number: $number) {
      ... on Issue {
        timelineItems(first: 100, itemTypes: [CROSS_REFERENCED_EVENT, CONNECTED_EVENT]) {
          nodes {
            ... on CrossReferencedEvent {
              source {
                ... on Issue {
                  number
                  repository { owner { login } name }
                }
                ... on PullRequest {
                  number
                  repository { owner { login } name }
                }
              }
            }
            ... on ConnectedEvent {
              subject {
                ... on Issue {
                  number
                  repository { owner { login } name }
                }
                ... on PullRequest {
                  number
                  repository { owner { login } name }
                }
              }
            }
          }
        }
      }
      ... on PullRequest {
        timelineItems(first: 100, itemTypes: [CROSS_REFERENCED_EVENT, CONNECTED_EVENT]) {
          nodes {
            ... on CrossReferencedEvent {
              source {
                ... on Issue {
                  number
                  repository { owner { login } name }
                }
                ... on PullRequest {
                  number
                  repository { owner { login } name }
                }
              }
            }
            ... on ConnectedEvent {
              subject {
                ... on Issue {
                  number
                  repository { owner { login } name }
                }
                ... on PullRequest {
                  number
                  repository { owner { login } name }
                }
              }
            }
          }
        }
      }
    }
  }
}
"#;
