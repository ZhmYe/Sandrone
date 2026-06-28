pub const DASHBOARD_HTML: &str = include_str!("../assets/dashboard/index.html");

pub const REQUEST_TEMPLATE: &str = include_str!("../templates/runtime/request.md");
pub const PLAN_TEMPLATE: &str = include_str!("../templates/runtime/plan.md");
pub const DECOMPOSITION_TEMPLATE: &str = include_str!("../templates/runtime/decomposition.md");
pub const DECOMPOSITION_JSON_TEMPLATE: &str =
    include_str!("../templates/runtime/decomposition.json");
pub const DAG_JSON_TEMPLATE: &str = include_str!("../templates/runtime/dag.json");
pub const CHANGE_DOC_TEMPLATE: &str = include_str!("../templates/runtime/change-doc.md");
pub const AGENT_JOURNAL_TEMPLATE: &str = include_str!("../templates/runtime/agent-journal.md");
pub const PR_DOC_TEMPLATE: &str = include_str!("../templates/runtime/pr-doc.md");
pub const OBSIDIAN_CHANGE_TEMPLATE: &str = include_str!("../templates/runtime/obsidian-change.md");
pub const OBSIDIAN_PROJECT_TEMPLATE: &str =
    include_str!("../templates/runtime/obsidian-project.md");
pub const OBSIDIAN_RELATIONS_TEMPLATE: &str =
    include_str!("../templates/runtime/obsidian-relations.md");
pub const OBSIDIAN_REQUESTS_BASE_TEMPLATE: &str =
    include_str!("../templates/runtime/obsidian-requests.base");
pub const OBSIDIAN_SLICES_BASE_TEMPLATE: &str =
    include_str!("../templates/runtime/obsidian-slices.base");

pub const ISSUE_UPDATE_SCRIPT: &str = include_str!("../templates/scripts/issue-update.sh");
pub const ISSUE_AGENT_SCRIPT: &str = include_str!("../templates/scripts/issue-agent.sh");
pub const REBASE_AGENT_SCRIPT: &str = include_str!("../templates/scripts/rebase-agent.sh");
pub const PR_CREATE_SCRIPT: &str = include_str!("../templates/scripts/pr-create.sh");
pub const PR_STATUS_SCRIPT: &str = include_str!("../templates/scripts/pr-status.sh");
pub const REQUEST_SCHEDULE_AGENT_SCRIPT: &str =
    include_str!("../templates/scripts/request-schedule-agent.sh");
pub const REQUEST_SCHEDULE_REVIEW_SCRIPT: &str =
    include_str!("../templates/scripts/request-schedule-review.sh");
pub const PR_MERGE_SCRIPT: &str = include_str!("../templates/scripts/pr-merge.sh");
pub const CHECK_FORMAT_SCRIPT: &str = include_str!("../templates/scripts/check-format.sh");
pub const REVIEW_TOOL_SCRIPT: &str = include_str!("../templates/scripts/review-tool.sh");
pub const CODEX_BIN_RESOLVER_SCRIPT: &str =
    include_str!("../templates/scripts/codex-bin-resolver.sh");
pub const WORKSPACE_ENV_EXAMPLE: &str = include_str!("../templates/.env.example");

pub const ISSUE_AGENT_PROMPT: &str = include_str!("../templates/prompts/issue-agent.md");
pub const DECOMPOSITION_AGENT_PROMPT: &str =
    include_str!("../templates/prompts/decomposition-agent.md");
pub const PLAN_AGENT_PROMPT: &str = include_str!("../templates/prompts/plan-agent.md");
pub const IMPLEMENTATION_AGENT_PROMPT: &str =
    include_str!("../templates/prompts/implementation-agent.md");
pub const REBASE_AGENT_PROMPT: &str = include_str!("../templates/prompts/rebase-agent.md");
pub const PLAN_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/plan-reviewer.md");
pub const DECOMPOSITION_REVIEWER_PROMPT: &str =
    include_str!("../templates/prompts/decomposition-reviewer.md");
pub const TEST_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/test-reviewer.md");
pub const DESIGN_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/design-reviewer.md");
pub const INTEGRATION_REVIEWER_PROMPT: &str =
    include_str!("../templates/prompts/integration-reviewer.md");
pub const REQUEST_SCHEDULE_REVIEWER_PROMPT: &str =
    include_str!("../templates/prompts/request-schedule-reviewer.md");

pub const REVIEW_RESULT_SCHEMA: &str =
    include_str!("../templates/schemas/review-result.schema.json");
