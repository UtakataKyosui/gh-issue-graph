use serde::{Deserialize, Serialize};

/// ラベルから解析した優先度
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical, // P0, priority:critical
    High,     // P1, priority:high
    Medium,   // P2, priority:medium
    Low,      // P3, priority:low
    #[default]
    None,
}

impl Priority {
    /// ステータスバー/CLI 向けの短縮インジケータ
    pub fn icon(&self) -> &'static str {
        match self {
            Priority::Critical => "[P0]",
            Priority::High     => "[P1]",
            Priority::Medium   => "[P2]",
            Priority::Low      => "[P3]",
            Priority::None     => "",
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High     => write!(f, "high"),
            Priority::Medium   => write!(f, "medium"),
            Priority::Low      => write!(f, "low"),
            Priority::None     => write!(f, ""),
        }
    }
}

/// ラベルリストから優先度をパースする
/// 対応パターン: P0/P1/P2/P3, priority:critical/high/medium/low, priority/high 等
pub fn parse_priority(labels: &[String]) -> Priority {
    for label in labels {
        let lower = label.to_lowercase();
        // "P0" / "P1" / "P2" / "P3" (単独)
        match lower.as_str() {
            "p0" => return Priority::Critical,
            "p1" => return Priority::High,
            "p2" => return Priority::Medium,
            "p3" => return Priority::Low,
            _ => {}
        }
        // "priority:xxx" / "priority/xxx" / "priority-xxx"
        let stripped = lower
            .strip_prefix("priority:")
            .or_else(|| lower.strip_prefix("priority/"))
            .or_else(|| lower.strip_prefix("priority-"));
        if let Some(level) = stripped {
            match level.trim() {
                "critical" | "p0" | "urgent" => return Priority::Critical,
                "high"     | "p1"            => return Priority::High,
                "medium"   | "p2" | "normal" => return Priority::Medium,
                "low"      | "p3"            => return Priority::Low,
                _ => {}
            }
        }
    }
    Priority::None
}

/// リポジトリ内の Issue/PR を一意に識別する ID
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

impl NodeId {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, number: u64) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            number,
        }
    }

    pub fn repo_full(&self) -> String {
        format!("{}/{}", self.owner, self.repo)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}#{}", self.owner, self.repo, self.number)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Issue,
    PullRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    Open,
    Closed,
    Merged,
    Draft,
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Open => write!(f, "open"),
            NodeState::Closed => write!(f, "closed"),
            NodeState::Merged => write!(f, "merged"),
            NodeState::Draft => write!(f, "draft"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssueNode {
    pub id: NodeId,
    /// GraphQL global node ID (for API cursors)
    pub graphql_id: String,
    pub kind: NodeKind,
    pub state: NodeState,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub priority: Priority,
    pub milestone: Option<String>,
    pub assignees: Vec<String>,
    pub url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl IssueNode {
    pub fn display_number(&self) -> String {
        match self.kind {
            NodeKind::Issue => format!("#{}", self.id.number),
            NodeKind::PullRequest => format!("PR #{}", self.id.number),
        }
    }
}

/// Issue/PR 間の関係性の種類
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    /// Sub-Issue 親子関係
    ParentChild,
    /// PR が Issue を close する参照
    ClosingReference,
    /// タイムラインの相互参照 (CrossReferencedEvent)
    CrossReference,
    /// UI で手動リンク (ConnectedEvent)
    ConnectedEvent,
    /// 本文/コメント中の #N メンション
    BodyMention,
    /// 同一マイルストーン (グループ化用)
    SameMilestone,
    /// Duplicate of #N マーカー
    Duplicate,
}

impl std::fmt::Display for RelationshipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationshipKind::ParentChild => write!(f, "parent-child"),
            RelationshipKind::ClosingReference => write!(f, "closes"),
            RelationshipKind::CrossReference => write!(f, "cross-ref"),
            RelationshipKind::ConnectedEvent => write!(f, "connected"),
            RelationshipKind::BodyMention => write!(f, "mentions"),
            RelationshipKind::SameMilestone => write!(f, "same-milestone"),
            RelationshipKind::Duplicate => write!(f, "duplicate"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub kind: RelationshipKind,
    pub source: NodeId,
    pub target: NodeId,
    pub bidirectional: bool,
}

impl Relationship {
    pub fn new(kind: RelationshipKind, source: NodeId, target: NodeId) -> Self {
        let bidirectional = matches!(kind, RelationshipKind::CrossReference | RelationshipKind::SameMilestone);
        Self { kind, source, target, bidirectional }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priority_patterns() {
        // P0/P1/P2/P3 単独
        assert_eq!(parse_priority(&["P0".to_string()]), Priority::Critical);
        assert_eq!(parse_priority(&["p1".to_string()]), Priority::High);
        assert_eq!(parse_priority(&["P2".to_string()]), Priority::Medium);
        assert_eq!(parse_priority(&["p3".to_string()]), Priority::Low);

        // priority:xxx パターン
        assert_eq!(parse_priority(&["priority:critical".to_string()]), Priority::Critical);
        assert_eq!(parse_priority(&["priority:high".to_string()]), Priority::High);
        assert_eq!(parse_priority(&["priority:medium".to_string()]), Priority::Medium);
        assert_eq!(parse_priority(&["priority:low".to_string()]), Priority::Low);

        // priority/xxx パターン
        assert_eq!(parse_priority(&["priority/high".to_string()]), Priority::High);
        assert_eq!(parse_priority(&["priority/p0".to_string()]), Priority::Critical);

        // ラベルなし / 無関係なラベル
        assert_eq!(parse_priority(&[]), Priority::None);
        assert_eq!(parse_priority(&["bug".to_string(), "enhancement".to_string()]), Priority::None);

        // 複数ラベルのうち最初に一致したもの
        assert_eq!(
            parse_priority(&["bug".to_string(), "P1".to_string()]),
            Priority::High
        );
    }
}
