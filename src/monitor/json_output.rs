use anyhow::{bail, Result};
use serde_json::Value;

use crate::monitor::types::RepoMonitorResult;

const FIELD_MAP: &[(&str, &str)] = &[
    ("number", "number"),
    ("title", "title"),
    ("url", "url"),
    ("labels", "labels"),
    ("linkedPrs", "linked_prs"),
    ("matchedBranches", "matched_branches"),
    ("parentIssueNumber", "parent_issue_number"),
    ("subIssues", "sub_issues"),
    ("subIssuesSummary", "sub_issues_summary"),
    ("milestone", "milestone"),
    ("relationships", "relationships"),
    ("hasActiveWork", "has_active_work"),
    ("repository", "repository"),
];

#[derive(Debug)]
pub struct FieldSpec {
    pub serde_name: String,
}

pub fn parse_fields(input: &str) -> Result<Vec<FieldSpec>> {
    if input.is_empty() {
        return Ok(vec![]);
    }
    let mut specs = Vec::new();
    for raw in input.split(',') {
        let name = raw.trim();
        match FIELD_MAP.iter().find(|(camel, _)| *camel == name) {
            Some((_, serde_name)) => specs.push(FieldSpec {
                serde_name: serde_name.to_string(),
            }),
            None => {
                let valid: Vec<&str> = FIELD_MAP.iter().map(|(c, _)| *c).collect();
                bail!(
                    "Unknown field '{name}'. Available fields: {}",
                    valid.join(", ")
                );
            }
        }
    }
    Ok(specs)
}

pub fn select_fields(results: &[RepoMonitorResult], fields: &[FieldSpec]) -> Value {
    let mut output = Vec::new();
    for result in results {
        let repo_name = format!("{}/{}", result.owner, result.name);
        for issue in &result.issues {
            let full = serde_json::to_value(issue).unwrap_or(Value::Null);
            let mut obj = serde_json::Map::new();
            for field in fields {
                match field.serde_name.as_str() {
                    "repository" => {
                        obj.insert(
                            "repository".to_string(),
                            Value::String(repo_name.clone()),
                        );
                    }
                    "has_active_work" => {
                        obj.insert(
                            "has_active_work".to_string(),
                            Value::Bool(issue.has_active_work()),
                        );
                    }
                    key => {
                        if let Some(v) = full.get(key) {
                            obj.insert(key.to_string(), v.clone());
                        }
                    }
                }
            }
            output.push(Value::Object(obj));
        }
    }
    Value::Array(output)
}

pub fn apply_jq(filter_str: &str, input: &Value) -> Result<String> {
    use jaq_core::{load, Compiler, Ctx, RcIter};
    use load::{Arena, File, Loader};

    let program = File {
        code: filter_str,
        path: (),
    };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();

    let modules = loader
        .load(&arena, program)
        .map_err(|errs| anyhow::anyhow!("Invalid jq expression: {:?}", errs))?;

    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| anyhow::anyhow!("jq compile error: {:?}", errs))?;

    let val = serde_to_jaq(input.clone());
    let inputs = RcIter::new(core::iter::empty());
    let results: Vec<String> = filter
        .run((Ctx::new([], &inputs), val))
        .map(|r| match r {
            Ok(v) => v.to_string(),
            Err(e) => format!("// jq error: {e:?}"),
        })
        .collect();

    Ok(results.join("\n"))
}

fn serde_to_jaq(v: Value) -> jaq_json::Val {
    use jaq_json::Val;
    use std::rc::Rc;
    match v {
        Value::Null => Val::Null,
        Value::Bool(b) => Val::from(b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Val::from(i as isize)
            } else if let Some(f) = n.as_f64() {
                Val::from(f)
            } else {
                Val::Num(Rc::new(n.to_string()))
            }
        }
        Value::String(s) => Val::from(s),
        Value::Array(arr) => Val::Arr(Rc::new(arr.into_iter().map(serde_to_jaq).collect())),
        Value::Object(obj) => {
            let mut map: indexmap::IndexMap<Rc<String>, Val, foldhash::fast::RandomState> =
                indexmap::IndexMap::with_hasher(foldhash::fast::RandomState::default());
            for (k, v) in obj {
                map.insert(Rc::new(k), serde_to_jaq(v));
            }
            Val::Obj(Rc::new(map))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::monitor::types::{
        IssueRelationship, IssueState, IssueStatus, LinkedPr, LinkType, MatchedBranch, Milestone,
        PrState, RelationshipType, SubIssueSummary, TrackedIssueRef,
    };

    fn make_result(owner: &str, name: &str, issues: Vec<IssueStatus>) -> RepoMonitorResult {
        RepoMonitorResult {
            owner: owner.to_string(),
            name: name.to_string(),
            issues,
            rate_limit: None,
            checked_at: Utc::now(),
            error: None,
        }
    }

    fn make_issue(number: u64, title: &str, with_pr: bool) -> IssueStatus {
        IssueStatus {
            number,
            title: title.to_string(),
            url: format!("https://github.com/o/r/issues/{number}"),
            labels: vec!["bug".to_string()],
            linked_prs: if with_pr {
                vec![LinkedPr {
                    number: 100,
                    title: "Fix".to_string(),
                    state: PrState::Open,
                    author: "alice".to_string(),
                    head_branch: "fix-1".to_string(),
                    url: "https://github.com/o/r/pull/100".to_string(),
                    link_type: LinkType::CrossReference,
                    will_close: false,
                    is_draft: false,
                }]
            } else {
                vec![]
            },
            matched_branches: vec![],
            parent_issue_number: None,
            sub_issues: vec![],
            sub_issues_summary: None,
            milestone: None,
            relationships: vec![],
        }
    }

    fn make_rich_issue() -> IssueStatus {
        IssueStatus {
            number: 42,
            title: "Rich issue".to_string(),
            url: "https://github.com/o/r/issues/42".to_string(),
            labels: vec!["enhancement".to_string(), "priority:high".to_string()],
            linked_prs: vec![LinkedPr {
                number: 99,
                title: "Implement feature".to_string(),
                state: PrState::Open,
                author: "bob".to_string(),
                head_branch: "feat/42-feature".to_string(),
                url: "https://github.com/o/r/pull/99".to_string(),
                link_type: LinkType::Closing,
                will_close: true,
                is_draft: true,
            }],
            matched_branches: vec![MatchedBranch {
                name: "bob/42-wip".to_string(),
                likely_author: Some("bob".to_string()),
                has_pr: false,
            }],
            parent_issue_number: Some(10),
            sub_issues: vec![],
            sub_issues_summary: Some(SubIssueSummary {
                total: 3,
                completed: 1,
                percent_completed: 33.3,
            }),
            milestone: Some(Milestone {
                number: 5,
                title: "v1.0".to_string(),
                state: "open".to_string(),
                due_on: Some("2026-04-01".to_string()),
                progress_percentage: 60.0,
                url: "https://github.com/o/r/milestone/5".to_string(),
            }),
            relationships: vec![IssueRelationship {
                relationship_type: RelationshipType::Tracks,
                issue: TrackedIssueRef {
                    number: 50,
                    title: "Sub-task".to_string(),
                    url: "https://github.com/o/r/issues/50".to_string(),
                    state: IssueState::Open,
                },
            }],
        }
    }

    #[test]
    fn parse_valid_fields() {
        let specs = parse_fields("number,title,linkedPrs").unwrap();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].serde_name, "number");
        assert_eq!(specs[1].serde_name, "title");
        assert_eq!(specs[2].serde_name, "linked_prs");
    }

    #[test]
    fn parse_single_field() {
        let specs = parse_fields("url").unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].serde_name, "url");
    }

    #[test]
    fn parse_fields_trims_whitespace() {
        let specs = parse_fields("number, title , linkedPrs").unwrap();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].serde_name, "number");
        assert_eq!(specs[1].serde_name, "title");
        assert_eq!(specs[2].serde_name, "linked_prs");
    }

    #[test]
    fn parse_empty_string_returns_empty() {
        let specs = parse_fields("").unwrap();
        assert!(specs.is_empty());
    }

    #[test]
    fn parse_unknown_field_errors() {
        let err = parse_fields("number,bogusField").unwrap_err();
        assert!(err.to_string().contains("bogusField"));
        assert!(err.to_string().contains("Available fields"));
    }

    #[test]
    fn select_extracts_only_requested_fields() {
        let results = vec![make_result("owner", "repo", vec![make_issue(1, "Bug", false)])];
        let fields = parse_fields("number,title").unwrap();
        let Value::Array(arr) = select_fields(&results, &fields) else {
            panic!("expected array");
        };
        assert_eq!(arr.len(), 1);
        let obj = arr[0].as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["number"], 1);
        assert_eq!(obj["title"], "Bug");
        assert!(!obj.contains_key("url"));
    }

    #[test]
    fn select_injects_repository_field() {
        let results = vec![make_result("myorg", "myrepo", vec![make_issue(1, "T", false)])];
        let fields = parse_fields("number,repository").unwrap();
        let Value::Array(arr) = select_fields(&results, &fields) else {
            panic!("expected array");
        };
        let obj = arr[0].as_object().unwrap();
        assert_eq!(obj["repository"], "myorg/myrepo");
    }

    #[test]
    fn jq_identity_filter() {
        let input = serde_json::json!([{"a": 1}, {"a": 2}]);
        let out = apply_jq(".", &input).unwrap();
        let parsed: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, input);
    }

    #[test]
    fn jq_field_access() {
        let input = serde_json::json!([{"n": 1}, {"n": 2}]);
        let out = apply_jq(".[].n", &input).unwrap();
        let lines: Vec<&str> = out.trim().split('\n').collect();
        assert_eq!(lines, vec!["1", "2"]);
    }

    #[test]
    fn jq_invalid_syntax_returns_error() {
        let input = serde_json::json!(null);
        let err = apply_jq("definitely not valid jq !!!", &input).unwrap_err();
        assert!(err.to_string().contains("jq"));
    }
}
