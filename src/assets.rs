pub const DASHBOARD_HTML: &str = include_str!("../assets/dashboard/index.html");

pub const REQUEST_TEMPLATE: &str = include_str!("../templates/runtime/request.md");
pub const PLAN_TEMPLATE: &str = include_str!("../templates/runtime/plan.md");
pub const CHANGE_DOC_TEMPLATE: &str = include_str!("../templates/runtime/change-doc.md");
pub const AGENT_JOURNAL_TEMPLATE: &str = include_str!("../templates/runtime/agent-journal.md");

pub const ISSUE_UPDATE_SCRIPT: &str = include_str!("../templates/scripts/issue-update.sh");
pub const ISSUE_AGENT_SCRIPT: &str = include_str!("../templates/scripts/issue-agent.sh");
pub const REBASE_AGENT_SCRIPT: &str = include_str!("../templates/scripts/rebase-agent.sh");
pub const PR_CREATE_SCRIPT: &str = include_str!("../templates/scripts/pr-create.sh");
pub const PR_STATUS_SCRIPT: &str = include_str!("../templates/scripts/pr-status.sh");
pub const REVIEW_TOOL_SCRIPT: &str = include_str!("../templates/scripts/review-tool.sh");
pub const CODEX_BIN_RESOLVER_SCRIPT: &str =
    include_str!("../templates/scripts/codex-bin-resolver.sh");

pub const ISSUE_AGENT_PROMPT: &str = include_str!("../templates/prompts/issue-agent.md");
pub const PLAN_AGENT_PROMPT: &str = include_str!("../templates/prompts/plan-agent.md");
pub const IMPLEMENTATION_AGENT_PROMPT: &str =
    include_str!("../templates/prompts/implementation-agent.md");
pub const REBASE_AGENT_PROMPT: &str = include_str!("../templates/prompts/rebase-agent.md");
pub const PLAN_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/plan-reviewer.md");
pub const TEST_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/test-reviewer.md");
pub const DESIGN_REVIEWER_PROMPT: &str = include_str!("../templates/prompts/design-reviewer.md");
pub const INTEGRATION_REVIEWER_PROMPT: &str =
    include_str!("../templates/prompts/integration-reviewer.md");

pub const REVIEW_RESULT_SCHEMA: &str =
    include_str!("../templates/schemas/review-result.schema.json");
