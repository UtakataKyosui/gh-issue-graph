# gh issue-graph

A `gh` CLI extension that analyzes and visualizes GitHub Issue/PR relationship graphs, and monitors multiple repositories for overlapping work.

## Installation

```bash
gh extension install UtakataKyosui/gh-issue-graph
```

## Commands

### Graph (default)

Analyze and visualize Issue/PR/Sub-Issue/Milestone/cross-reference relationships as an interactive graph.

```bash
# Interactive TUI (default)
gh issue-graph --repo owner/repo

# JSON output
gh issue-graph --repo owner/repo --json

# Tree format
gh issue-graph --repo owner/repo --format tree

# DOT format (Graphviz)
gh issue-graph --repo owner/repo --format dot

# Focus on a specific issue and its relationships
gh issue-graph --repo owner/repo --issue 42 --depth 3
```

#### Graph Flags

| Flag | Default | Description |
|---|---|---|
| `--repo <OWNER/REPO>` | — | Repository to analyze |
| `--issue <N>` | — | Focus on a specific issue number |
| `--depth <N>` | `2` | BFS traversal depth |
| `--json [FIELDS]` | — | JSON output; optionally specify fields |
| `--jq <EXPR>` | — | jq filter (requires `--json`) |
| `--format <FMT>` | `list` | Output format: `list`, `tree`, `dot` |
| `--milestone <NAME>` | — | Filter by milestone |
| `--label <LABEL>` | — | Filter by label |
| `--no-timeline` | false | Disable cross-reference fetching (faster) |
| `--no-sub-issues` | false | Disable sub-issue fetching |

---

### Monitor

Monitor multiple repositories simultaneously for overlapping issue work — TUI with real-time polling, filtering, and visual effects.

```bash
gh issue-graph monitor
```

The monitor reads from `~/.config/gh-issue-monitor/config.json` (or `$GH_ISSUE_MONITOR_CONFIG`).

#### Config file

```json
{
  "poll_interval": 60,
  "repositories": [
    { "owner": "my-org", "name": "my-repo" },
    { "owner": "my-org", "name": "another-repo", "labels": ["bug"] }
  ]
}
```

#### Monitor TUI Controls

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Tab` / `Shift+Tab` | Switch repository tab |
| `/` | Keyword search |
| `m` | Milestone picker |
| `C` | Clear all filters |
| `r` | Force refresh |
| `PageDown` / `PageUp` | Scroll detail panel |
| `Esc` | Quit (or cancel input) |

#### Monitor snapshot fields

Each issue shows: linked PRs, matched branches, sub-issue progress, milestone, and relationship tracking.

---

### Config subcommand

```bash
gh issue-graph config show   # Show current graph config
gh issue-graph config init   # Initialize config file
```

---

## Auth

Uses `gh auth` via the `GH_TOKEN` or `GITHUB_TOKEN` environment variables, falling back to `gh auth token`.
