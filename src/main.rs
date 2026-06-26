mod assets;
mod codegraph;
mod dashboard;
mod defaults;
mod delivery;
mod doc_status;
mod doctor;
mod jobs;
mod merge_plan;
mod obsidian;
mod registry;
mod review_gate;
mod slices;
mod state;
mod utils;

pub(crate) use codegraph::*;
pub(crate) use defaults::*;
pub(crate) use delivery::{
    deliver_finished_request, pr_merge_request, run_pr_merge_scheduler_from_tick,
};
pub(crate) use doc_status::*;
pub(crate) use doctor::doctor;
pub(crate) use jobs::*;
pub(crate) use merge_plan::*;
pub(crate) use obsidian::*;
pub(crate) use review_gate::{
    code_review, decomposition_review, integration_review, plan_review, refresh_review_stage,
    review_diagnostic_excerpt, review_worker,
};
pub(crate) use slices::*;
pub(crate) use state::*;
pub(crate) use utils::*;

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const CONFIG_PATH: &str = ".sandrone/config.toml";
const STATE_PATH: &str = ".sandrone/state/requests.tsv";
const EVENTS_PATH: &str = ".sandrone/state/events.ndjson";
const SESSIONS_PATH: &str = ".sandrone/sessions.json";
const LOCAL_STATE_DIR: &str = ".sandrone";
const LEGACY_LOCAL_STATE_DIR: &str = ".codex-auto-dev";
const GLOBAL_WORKSPACES_FILE: &str = "workspaces.json";
const FRAMEWORK_SCHEMA_VERSION: u32 = 4;
const DEV_REPO: &str = "dev/repo";
const WORKTREES: &str = "dev/worktrees";
const ISSUE_TOOL: &str = "tools/issue-update.sh";
const ISSUE_AGENT_TOOL: &str = "tools/issue-agent.sh";
const REBASE_AGENT_TOOL: &str = "tools/rebase-agent.sh";
const PR_TOOL: &str = "tools/pr-create.sh";
const PR_STATUS_TOOL: &str = "tools/pr-status.sh";
const MERGE_PLAN_TOOL: &str = "tools/merge-plan.sh";
const PR_MERGE_TOOL: &str = "tools/pr-merge.sh";
const CHECK_FORMAT_TOOL: &str = "tools/check-format.sh";
const PLAN_REVIEW_TOOL: &str = "tools/plan-review.sh";
const DECOMPOSITION_REVIEW_TOOL: &str = "tools/decomposition-review.sh";
const TEST_REVIEW_TOOL: &str = "tools/test-review.sh";
const DESIGN_REVIEW_TOOL: &str = "tools/design-review.sh";
const INTEGRATION_REVIEW_TOOL: &str = "tools/integration-review.sh";
const ISSUE_AGENT_PROMPT: &str = "tools/prompts/issue-agent.md";
const DECOMPOSITION_AGENT_PROMPT: &str = "tools/prompts/decomposition-agent.md";
const PLAN_AGENT_PROMPT: &str = "tools/prompts/plan-agent.md";
const IMPLEMENTATION_AGENT_PROMPT: &str = "tools/prompts/implementation-agent.md";
const REBASE_AGENT_PROMPT: &str = "tools/prompts/rebase-agent.md";
const PLAN_REVIEW_PROMPT: &str = "tools/prompts/plan-reviewer.md";
const DECOMPOSITION_REVIEW_PROMPT: &str = "tools/prompts/decomposition-reviewer.md";
const TEST_REVIEW_PROMPT: &str = "tools/prompts/test-reviewer.md";
const DESIGN_REVIEW_PROMPT: &str = "tools/prompts/design-reviewer.md";
const INTEGRATION_REVIEW_PROMPT: &str = "tools/prompts/integration-reviewer.md";
const REVIEW_SCHEMA: &str = "tools/schemas/review-result.schema.json";
const ISSUE_TOOL_EXAMPLE: &str = "tools/issue-update.example.sh";
const ISSUE_AGENT_TOOL_EXAMPLE: &str = "tools/issue-agent.example.sh";
const REBASE_AGENT_TOOL_EXAMPLE: &str = "tools/rebase-agent.example.sh";
const PR_TOOL_EXAMPLE: &str = "tools/pr-create.example.sh";
const PR_STATUS_TOOL_EXAMPLE: &str = "tools/pr-status.example.sh";
const MERGE_PLAN_TOOL_EXAMPLE: &str = "tools/merge-plan.example.sh";
const PR_MERGE_TOOL_EXAMPLE: &str = "tools/pr-merge.example.sh";
const CHECK_FORMAT_TOOL_EXAMPLE: &str = "tools/check-format.example.sh";
const PLAN_REVIEW_TOOL_EXAMPLE: &str = "tools/plan-review.example.sh";
const DECOMPOSITION_REVIEW_TOOL_EXAMPLE: &str = "tools/decomposition-review.example.sh";
const TEST_REVIEW_TOOL_EXAMPLE: &str = "tools/test-review.example.sh";
const DESIGN_REVIEW_TOOL_EXAMPLE: &str = "tools/design-review.example.sh";
const INTEGRATION_REVIEW_TOOL_EXAMPLE: &str = "tools/integration-review.example.sh";
const ISSUE_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/issue-agent.example.md";
const DECOMPOSITION_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/decomposition-agent.example.md";
const PLAN_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/plan-agent.example.md";
const IMPLEMENTATION_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/implementation-agent.example.md";
const REBASE_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/rebase-agent.example.md";
const PLAN_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/plan-reviewer.example.md";
const DECOMPOSITION_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/decomposition-reviewer.example.md";
const TEST_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/test-reviewer.example.md";
const DESIGN_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/design-reviewer.example.md";
const INTEGRATION_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/integration-reviewer.example.md";
const REVIEW_SCHEMA_EXAMPLE: &str = "tools/schemas/review-result.example.schema.json";
const WORKFLOW_SKILL: &str = "skills/sandrone/SKILL.md";
const WORKFLOW_SKILL_CONTENT: &str = include_str!("../skills/sandrone/SKILL.md");
const DEFAULT_DASHBOARD_HOST: &str = "127.0.0.1";
const DEFAULT_DASHBOARD_PORT: u16 = 47217;
const DEFAULT_DECOMPOSITION_MAX_ATTEMPTS: u32 = 5;
const DEFAULT_PLAN_MAX_ATTEMPTS: u32 = 5;
const DEFAULT_CODE_MAX_ATTEMPTS: u32 = 20;
const DEFAULT_INTEGRATION_MAX_ATTEMPTS: u32 = 20;
const STATUS_WAIT_UPDATE_PR: &str = "wait-update-pr";
const STATUS_WAIT_FINISH: &str = "wait-finish";
const STATUS_FINISHED: &str = "finished";
const LEGACY_WAITING_FINISH: &str = "waiting-finish";
const LEGACY_PR_PENDING: &str = "pr-pending";

#[derive(Clone, Debug)]
struct Config {
    schema_version: u32,
    repo_name: String,
    git_url: String,
    base_branch: String,
    parallel_limit: usize,
    auto_merge: bool,
}

#[derive(Clone, Debug)]
struct WorkspaceRecord {
    key: String,
    repo_name: String,
    git_url: String,
    workspace_path: String,
    target_repo: String,
    last_status: String,
    request_count: usize,
    status_counts: BTreeMap<String, usize>,
    updated_at: String,
}

#[derive(Clone, Debug)]
struct Request {
    request_id: String,
    external_id: String,
    source: String,
    title: String,
    body: String,
    url: String,
    status: String,
    change_name: String,
    change_path: String,
    branch: String,
    worktree_path: String,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug)]
struct SessionRecord {
    request_id: String,
    phase: String,
    status: String,
    thread_id: String,
    thread_url: String,
    workspace: String,
    target_repo: String,
    worktree: String,
    change_path: String,
    started_at: String,
    updated_at: String,
}

#[derive(Clone, Debug)]
struct PlanPreflight {
    notes: Vec<String>,
}

struct IntegrationRecord<'a> {
    mode: &'a str,
    base_branch: &'a str,
    base_ref: &'a str,
    before_head: &'a str,
    after_head: &'a str,
    pr_status: &'a str,
    detail: &'a str,
}

#[derive(Clone, Debug)]
pub(crate) struct PrStatusReport {
    status: String,
    url: String,
    detail: String,
    raw: String,
}

#[derive(Clone, Debug)]
struct DeliveryResult {
    commit_message: String,
    branch: String,
    committed: bool,
    pushed_with_force_lease: bool,
    pr_url: Option<String>,
    pr_status: String,
    compare_url: Option<String>,
    pr_error: String,
}

fn canonical_status(status: &str) -> &str {
    match status {
        LEGACY_WAITING_FINISH => STATUS_WAIT_UPDATE_PR,
        LEGACY_PR_PENDING => STATUS_WAIT_FINISH,
        _ => status,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DoctorStatus {
    Ok,
    Warn,
    Fail,
}

#[derive(Clone, Debug)]
struct DoctorCheck {
    name: &'static str,
    status: DoctorStatus,
    detail: String,
}

#[derive(Clone, Debug)]
enum GitPullOutcome {
    Skipped(String),
    AlreadyUpToDate,
    Updated,
}

impl DoctorCheck {
    fn status_label(&self) -> &'static str {
        match self.status {
            DoctorStatus::Ok => "OK",
            DoctorStatus::Warn => "WARN",
            DoctorStatus::Fail => "FAIL",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ReviewDefinition {
    name: &'static str,
    tool: &'static str,
    file_stem: &'static str,
}

#[derive(Clone, Debug)]
struct ReviewResult {
    reviewer: String,
    approved: bool,
    has_blocking_findings: bool,
    gate_unavailable: bool,
    recommended_next_phase: String,
    summary: String,
    diagnostic: String,
    path: String,
}

#[derive(Clone, Debug)]
struct ReviewFinding {
    title: String,
    evidence: String,
    impact: String,
    required_fix: String,
    suggested_change: String,
    verification: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgentPhase {
    Decomposition,
    Planning,
    Implementation,
    Rebase,
}

impl AgentPhase {
    fn as_str(self) -> &'static str {
        match self {
            AgentPhase::Decomposition => "decomposition",
            AgentPhase::Planning => "planning",
            AgentPhase::Implementation => "implementation",
            AgentPhase::Rebase => "rebase",
        }
    }

    fn running_status(self) -> &'static str {
        match self {
            AgentPhase::Decomposition => "decomposition-agent-running",
            AgentPhase::Planning => "planning-agent-running",
            AgentPhase::Implementation => "implementation-agent-running",
            AgentPhase::Rebase => "rebase-agent-running",
        }
    }

    fn review_rejected_status(self) -> &'static str {
        match self {
            AgentPhase::Decomposition => "decomposition-review-rejected",
            AgentPhase::Planning => "plan-review-rejected",
            AgentPhase::Implementation => "code-review-rejected",
            AgentPhase::Rebase => "integration-review-rejected",
        }
    }

    fn prompt_path(self) -> &'static str {
        match self {
            AgentPhase::Decomposition => DECOMPOSITION_AGENT_PROMPT,
            AgentPhase::Planning => PLAN_AGENT_PROMPT,
            AgentPhase::Implementation => IMPLEMENTATION_AGENT_PROMPT,
            AgentPhase::Rebase => REBASE_AGENT_PROMPT,
        }
    }

    fn tool_path(self) -> &'static str {
        match self {
            AgentPhase::Decomposition | AgentPhase::Planning | AgentPhase::Implementation => {
                ISSUE_AGENT_TOOL
            }
            AgentPhase::Rebase => REBASE_AGENT_TOOL,
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());
    let args: Vec<String> = args.collect();

    match command.as_str() {
        "new" => new_workspace(&args),
        "update" => update_requests(),
        "tick" => tick(&args),
        "advance" => advance_request(&args),
        "doctor" => doctor(&args),
        "doc-status" => show_doc_status(&args),
        "obsidian-refresh" => refresh_obsidian_command(&args),
        "plan" => create_plan_packet(&args),
        "decompose" => create_decomposition_packet(&args),
        "submit" => submit_approval(&args),
        "approve" => decide_approval(&args, "approved"),
        "reject" => decide_approval(&args, "rejected"),
        "gates" | "approvals" => show_gates(&args),
        "plan-review" => plan_review(&args),
        "decomposition-review" => decomposition_review(&args),
        "code-review" => code_review(&args),
        "integration-review" => integration_review(&args),
        "__review-worker" => review_worker(&args),
        "start" => start_worktree(&args),
        "finish" => finish_request(&args),
        "pr-status" => pr_status_request(&args),
        "pr-merge" => pr_merge_request(&args),
        "pr-refresh" => pr_refresh_request(&args),
        "block" => block_request(&args),
        "resume" => resume_request(&args),
        "session" => register_session(&args),
        "sessions" => list_sessions(&args),
        "upgrade" => upgrade_workspace(&args),
        "list" => list_requests(),
        "dashboard" => dashboard::dashboard(&args),
        "status" => status(&args),
        "validate" => validate(),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            print_help();
            Err(format!("unknown command: {command}").into())
        }
    }
}

fn new_workspace(args: &[String]) -> Result<()> {
    ensure_allowed_flags(args, &["--url", "--name"])?;
    ensure_not_framework_source_checkout()?;
    let url = flag_value(args, "--url")?;
    let name = flag_value(args, "--name")?;

    match (url, name) {
        (Some(_), Some(_)) => usage("new (--url <git-url> | --name <project-name>)"),
        (None, None) => usage("new (--url <git-url> | --name <project-name>)"),
        (Some(git_url), None) => initialize_cloned_workspace(&git_url),
        (None, Some(repo_name)) => initialize_empty_workspace(&repo_name),
    }
}

fn ensure_not_framework_source_checkout() -> Result<()> {
    if is_framework_source_checkout()? {
        return Err(
            "refusing to initialize sandrone source checkout as a managed workspace; run `sandrone new` from a separate outer directory"
                .into(),
        );
    }
    Ok(())
}

fn is_framework_source_checkout() -> Result<bool> {
    let cargo_toml = Path::new("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(false);
    }
    let cargo_content = fs::read_to_string(cargo_toml)?;
    Ok(cargo_content.contains("name = \"sandrone\"")
        && Path::new("src/main.rs").exists()
        && Path::new("templates").exists()
        && Path::new("skills/sandrone").exists())
}

fn initialize_cloned_workspace(git_url: &str) -> Result<()> {
    prepare_workspace_dirs()?;
    if !Path::new(DEV_REPO).exists() {
        run_command(
            Command::new("git")
                .args(["clone", git_url, DEV_REPO])
                .envs(proxy_env()),
        )?;
    }
    let repo_name = repo_name_from_url(git_url);
    write_config(&repo_name, git_url, "master")?;
    ensure_state_file()?;
    ensure_sessions_file()?;
    write_default_issue_tool()?;
    write_default_issue_agent_tool()?;
    write_default_pr_tool()?;
    write_default_pr_status_tool()?;
    write_default_merge_plan_tool()?;
    write_default_pr_merge_tool()?;
    write_default_review_tools()?;
    refresh_default_reference_examples()?;
    write_default_env_files()?;
    write_default_workflow_skill()?;

    println!("Created sandrone workspace");
    println!("  mode: clone");
    println!("  workspace naming: arbitrary outer workspace name is OK for cloned repositories");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  issue agent: {ISSUE_AGENT_TOOL}");
    println!("  PR tool: {PR_TOOL}");
    println!("  check tool: {CHECK_FORMAT_TOOL}");
    println!("  review tools: {PLAN_REVIEW_TOOL}, {TEST_REVIEW_TOOL}, {DESIGN_REVIEW_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    if repo_has_commits(DEV_REPO) {
        let codegraph_outcome = ensure_codegraph_initialized(DEV_REPO);
        print_codegraph_init_outcome("  ", &codegraph_outcome);
        let context_outcome = refresh_codegraph_context(DEV_REPO);
        print_codegraph_context_outcome("  ", &context_outcome);
        println!("  repository has content: CodeGraph context should be reviewed before planning");
        println!("  next: inspect obsidian/codegraph/context.md, then wait for a request");
        append_event(
            "codegraph_init_checked",
            "",
            "clone",
            codegraph_event_status(&codegraph_outcome),
            &codegraph_outcome_detail(&codegraph_outcome),
        )?;
    } else {
        println!("  repository is empty: skip CodeGraph until the first user request");
        println!(
            "  next: wait for a request, then sandrone plan --name <YYYY-MM-DD-name> --request_id <REQ-0001>"
        );
    }
    append_event(
        "workspace_initialized",
        "",
        "clone",
        "ready",
        &format!("repo={repo_name}; git_url={git_url}"),
    )?;
    registry::refresh_current_workspace_registry_or_warn("ready");
    println!(
        "  workspace registry: {}",
        registry::global_workspaces_path().display()
    );
    Ok(())
}

fn initialize_empty_workspace(repo_name: &str) -> Result<()> {
    if repo_name.trim().is_empty() {
        return Err("project name must not be empty".into());
    }

    prepare_workspace_dirs()?;
    if !Path::new(DEV_REPO).exists() {
        fs::create_dir_all(DEV_REPO)?;
        run_command(Command::new("git").arg("init").current_dir(DEV_REPO))?;
        run_command(
            Command::new("git")
                .args(["checkout", "-B", "master"])
                .current_dir(DEV_REPO),
        )?;
    }
    write_config(repo_name, &format!("local:{repo_name}"), "master")?;
    ensure_state_file()?;
    ensure_sessions_file()?;
    write_default_issue_tool()?;
    write_default_issue_agent_tool()?;
    write_default_pr_tool()?;
    write_default_pr_status_tool()?;
    write_default_merge_plan_tool()?;
    write_default_pr_merge_tool()?;
    write_default_review_tools()?;
    refresh_default_reference_examples()?;
    write_default_env_files()?;
    write_default_workflow_skill()?;

    println!("Created sandrone workspace");
    println!("  mode: empty");
    println!("  project name: {repo_name}");
    println!("  workspace naming: use an outer workspace directory named {repo_name}-auto-dev");
    println!("  target git repository name: {repo_name}");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  issue agent: {ISSUE_AGENT_TOOL}");
    println!("  PR tool: {PR_TOOL}");
    println!("  check tool: {CHECK_FORMAT_TOOL}");
    println!("  review tools: {PLAN_REVIEW_TOOL}, {TEST_REVIEW_TOOL}, {DESIGN_REVIEW_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    println!(
        "  next: sandrone plan --name {}-initial-plan --request_id REQ-0001",
        today()
    );
    append_event(
        "workspace_initialized",
        "",
        "empty",
        "ready",
        &format!("repo={repo_name}; git_url=local:{repo_name}"),
    )?;
    registry::refresh_current_workspace_registry_or_warn("ready");
    println!(
        "  workspace registry: {}",
        registry::global_workspaces_path().display()
    );
    Ok(())
}

fn update_requests() -> Result<()> {
    ensure_initialized()?;
    let mut requests = load_requests()?;
    let output = Command::new("sh").arg(ISSUE_TOOL).output()?;
    if !output.status.success() {
        return Err(format!(
            "{ISSUE_TOOL} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let mut by_external_id = requests
        .iter()
        .enumerate()
        .map(|(index, request)| (request.external_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let stdout = String::from_utf8(output.stdout)?;
    let mut created = 0;
    let mut updated = 0;

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<String> = line.split('\t').map(unescape_field).collect();
        if fields.len() < 5 {
            continue;
        }

        let external_id = fields[0].clone();
        if let Some(index) = by_external_id.get(&external_id).copied() {
            requests[index].source = fields[1].clone();
            requests[index].title = fields[2].clone();
            requests[index].body = fields[3].clone();
            requests[index].url = fields[4].clone();
            requests[index].updated_at = now_string();
            append_event(
                "request_refreshed",
                &requests[index].request_id,
                "update",
                "refreshed",
                &format!("external_id={external_id}; source={}", fields[1]),
            )?;
            updated += 1;
        } else {
            let request_id = next_request_id(&requests);
            by_external_id.insert(external_id.clone(), requests.len());
            requests.push(Request {
                request_id: request_id.clone(),
                external_id: external_id.clone(),
                source: fields[1].clone(),
                title: fields[2].clone(),
                body: fields[3].clone(),
                url: fields[4].clone(),
                status: "discovered".to_string(),
                change_name: String::new(),
                change_path: String::new(),
                branch: String::new(),
                worktree_path: String::new(),
                created_at: now_string(),
                updated_at: now_string(),
            });
            append_event(
                "request_discovered",
                &request_id,
                "update",
                "discovered",
                &format!(
                    "external_id={external_id}; source={}; title={}",
                    fields[1], fields[2]
                ),
            )?;
            created += 1;
        }
    }

    save_requests(&requests)?;
    registry::refresh_current_workspace_registry_or_warn("ready");
    println!("Update complete: {created} new, {updated} refreshed");
    for request in requests
        .iter()
        .filter(|request| request.status == "discovered")
    {
        println!("  {} {}", request.request_id, request.title);
    }
    Ok(())
}

fn tick(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--request_id",
            "--request-id",
            "--max-attempts",
            "--parallel-limit",
            "--parallel_limit",
            "--auto-merge",
            "--no-auto-merge",
        ],
    )?;
    let request_id = flag_value(args, "--request_id")?.or(flag_value(args, "--request-id")?);
    let config = load_config()?;
    let parallel_limit = parse_parallel_limit(
        flag_value(args, "--parallel-limit")?.or(flag_value(args, "--parallel_limit")?),
        config.parallel_limit,
    )?;
    let max_attempts = parse_max_attempts(flag_value(args, "--max-attempts")?)?;
    let auto_merge_enabled = resolve_tick_auto_merge(args, config.auto_merge)?;

    update_requests()?;

    let refreshed = refresh_tick_statuses()?;
    if refreshed > 0 {
        println!("Tick refreshed {refreshed} request status(es).");
    }

    let requests = load_requests()?;
    let request_ids = select_tick_requests(&requests, request_id.as_deref())?;
    if request_ids.is_empty() {
        if !run_pr_merge_scheduler_from_tick(request_id.as_deref(), auto_merge_enabled)? {
            println!("Tick complete: no pending request.");
        }
        return Ok(());
    }
    let running_count = running_issue_agent_count(&requests);
    if running_count >= parallel_limit {
        let merge_checked =
            run_pr_merge_scheduler_from_tick(request_id.as_deref(), auto_merge_enabled)?;
        println!(
            "Tick parallel limit reached: {running_count}/{parallel_limit} issue-agent(s) already running."
        );
        if merge_checked {
            println!("Tick merge scheduler ran despite issue-agent parallel limit.");
        }
        return Ok(());
    }
    let available_slots = parallel_limit - running_count;
    let mut remaining_slots = available_slots;
    let mut delayed_by_limit = 0usize;

    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        return Err(format!("{ISSUE_AGENT_TOOL} does not exist").into());
    }

    let mut preflight = None;
    let mut dispatched = Vec::new();
    let mut failures = Vec::new();
    for request_id in request_ids {
        if remaining_slots == 0 {
            delayed_by_limit += 1;
            continue;
        }
        let Some(_lock) = RequestLock::acquire(&request_id)? else {
            continue;
        };
        let outcome =
            match dispatch_next_slice_for_parent(&request_id, max_attempts, &mut preflight) {
                Ok(Some(dispatched)) => Ok(Some(dispatched)),
                Ok(None) => {
                    dispatch_next_agent_for_request(&request_id, max_attempts, &mut preflight)
                }
                Err(error) => Err(error),
            };
        match outcome {
            Ok(Some((request, phase, pid))) => {
                dispatched.push((request, phase, pid));
                remaining_slots = remaining_slots.saturating_sub(1);
            }
            Ok(None) => {}
            Err(error) => {
                failures.push(format!("{request_id}: {error}"));
            }
        }
    }

    let merge_checked =
        run_pr_merge_scheduler_from_tick(request_id.as_deref(), auto_merge_enabled)?;

    if dispatched.is_empty() && failures.is_empty() && !merge_checked {
        println!("Tick complete: no pending request.");
        return Ok(());
    }

    if !dispatched.is_empty() {
        println!("Tick dispatched {} issue-agent(s).", dispatched.len());
        if delayed_by_limit > 0 {
            println!(
                "Tick parallel limit {parallel_limit}: {delayed_by_limit} pending request(s) left for future tick(s)."
            );
        }
        for (request, phase, pid) in dispatched {
            println!(
                "  Dispatched {} phase {} pid {}",
                request.request_id,
                phase.as_str(),
                pid
            );
            println!("    change path: {}", request.change_path);
            println!(
                "    logs: {} | {}",
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "tick failed to dispatch some issue-agent(s): {}",
            failures.join("; ")
        )
        .into())
    }
}

fn advance_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id", "--max-attempts"])?;
    let request_id = required_request_id(args)?;
    let max_attempts = parse_max_attempts(flag_value(args, "--max-attempts")?)?;
    let Some(_lock) = RequestLock::acquire(&request_id)? else {
        println!("Advance skipped for {request_id}: request lock is already held.");
        return Ok(());
    };
    let mut progressed = false;
    let mut preflight = None;

    loop {
        if refresh_request_status_by_id(&request_id)? {
            progressed = true;
            if let Some((request, phase, pid)) =
                dispatch_next_slice_for_parent(&request_id, max_attempts, &mut preflight)?
            {
                println!(
                    "Advance dispatched {} phase {} pid {}",
                    request.request_id,
                    phase.as_str(),
                    pid
                );
                println!("  change path: {}", request.change_path);
                println!(
                    "  logs: {} | {}",
                    agent_stdout_path(&request.request_id).display(),
                    agent_stderr_path(&request.request_id).display()
                );
                break;
            }
            continue;
        }
        if let Some((request, phase, pid)) =
            dispatch_next_slice_for_parent(&request_id, max_attempts, &mut preflight)?
        {
            progressed = true;
            println!(
                "Advance dispatched {} phase {} pid {}",
                request.request_id,
                phase.as_str(),
                pid
            );
            println!("  change path: {}", request.change_path);
            println!(
                "  logs: {} | {}",
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
            break;
        }
        if let Some((request, phase, pid)) =
            dispatch_next_agent_for_request(&request_id, max_attempts, &mut preflight)?
        {
            progressed = true;
            println!(
                "Advance dispatched {} phase {} pid {}",
                request.request_id,
                phase.as_str(),
                pid
            );
            println!("  change path: {}", request.change_path);
            println!(
                "  logs: {} | {}",
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
            break;
        }
        break;
    }

    if progressed {
        println!("Advance complete for {request_id}.");
    } else {
        println!("Advance complete for {request_id}: no pending action.");
    }
    Ok(())
}

fn create_plan_packet(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--name", "--request_id", "--request-id"])?;
    let change_name = required_flag(args, "--name")?;
    let request_id = required_request_id(args)?;
    validate_change_name(&change_name)?;
    let preflight = assess_repository_before_planning()?;

    let mut requests = load_requests()?;
    let index = match find_request_index(&requests, &request_id) {
        Some(index) => index,
        None => {
            requests.push(manual_request(&request_id, &change_name));
            requests.len() - 1
        }
    };

    let request = create_plan_packet_for_index(&mut requests, index, &change_name, &preflight)?;

    println!("Planning packet ready for {}", request.request_id);
    for note in preflight.notes {
        println!("  preflight: {note}");
    }
    println!("  change path: {}", request.change_path);
    println!(
        "  plan template: {}",
        request_artifact_path_string(&request, "plan.md")
    );
    println!(
        "  request: {}",
        request_artifact_path_string(&request, "request.md")
    );
    println!("  Codex or planning agent must fill the plan; outer tick runs review gates.");
    Ok(())
}

fn refresh_obsidian_command(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &[])?;
    let requests = load_requests()?;
    refresh_obsidian_artifacts(&requests)?;
    println!("Obsidian artifacts refreshed");
    println!("  project: {OBSIDIAN_PROJECT_NOTE}");
    println!("  relations: {OBSIDIAN_RELATIONS_NOTE}");
    println!("  requests: obsidian/derived/requests.json");
    println!("  slices: obsidian/derived/slices.json");
    println!("  canvas: {OBSIDIAN_PROJECT_CANVAS}");
    println!("  bases: obsidian/views/requests.base, obsidian/views/slices.base");
    Ok(())
}

fn show_doc_status(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id", "--phase"])?;
    let request_id = required_request_id(args)?;
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let phase = match flag_value(args, "--phase")?.as_deref() {
        Some("decomposition") => AgentPhase::Decomposition,
        Some("planning") | Some("plan") => AgentPhase::Planning,
        Some("implementation") | Some("impl") => AgentPhase::Implementation,
        Some("rebase") => AgentPhase::Rebase,
        Some(other) => {
            return Err(format!(
                "unsupported phase `{other}`. Use decomposition, planning, implementation, or rebase."
            )
            .into());
        }
        None => inferred_document_phase(request),
    };
    print!("{}", render_doc_status(request, phase)?);
    Ok(())
}

fn inferred_document_phase(request: &Request) -> AgentPhase {
    match canonical_status(&request.status) {
        "decomposition"
        | "decomposition-agent-running"
        | "decomposition-submitted"
        | "decomposition-review-rejected" => AgentPhase::Decomposition,
        "planning" | "planning-agent-running" | "plan-submitted" | "plan-review-rejected" => {
            AgentPhase::Planning
        }
        "rebase-agent-running" | "integration-review-submitted" | "integration-review-rejected" => {
            AgentPhase::Rebase
        }
        _ => AgentPhase::Implementation,
    }
}

fn create_decomposition_packet(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--name", "--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let change_name = flag_value(args, "--name")?;
    if let Some(change_name) = &change_name {
        validate_change_name(change_name)?;
    }
    let preflight = assess_repository_before_planning()?;

    let mut requests = load_requests()?;
    let index = match find_request_index(&requests, &request_id) {
        Some(index) => index,
        None => {
            let Some(change_name) = &change_name else {
                return Err(format!(
                    "unknown request_id: {request_id}. Use --name <YYYY-MM-DD-short-name> to create a manual decomposition packet."
                )
                .into());
            };
            requests.push(manual_request(&request_id, change_name));
            requests.len() - 1
        }
    };

    let request = create_decomposition_packet_for_index(
        &mut requests,
        index,
        change_name.as_deref(),
        &preflight,
    )?;

    println!("Decomposition packet ready for {}", request.request_id);
    for note in preflight.notes {
        println!("  preflight: {note}");
    }
    println!("  change path: {}", request.change_path);
    println!(
        "  decomposition: {}",
        request_artifact_path_string(&request, "decomposition.md")
    );
    println!(
        "  decomposition json: {}/decomposition.json",
        request.change_path
    );
    println!("  dag: {}/dag.json", request.change_path);
    println!(
        "  next: fill decomposition artifacts, then run sandrone decomposition-review --request_id {}",
        request.request_id
    );
    Ok(())
}

fn create_decomposition_packet_for_index(
    requests: &mut [Request],
    index: usize,
    change_name: Option<&str>,
    preflight: &PlanPreflight,
) -> Result<Request> {
    let mut request = requests[index].clone();
    if request.change_path.trim().is_empty() {
        let Some(change_name) = change_name else {
            return Err(format!(
                "{} has no decomposition packet. Run: sandrone decompose --name {}-short-name --request_id {}",
                request.request_id,
                today(),
                request.request_id
            )
            .into());
        };
        request.change_name = change_name.to_string();
        request.change_path = change_artifact_path(change_name);
        request.status = "decomposition".to_string();
        request.updated_at = now_string();
        generate_decomposition_packet(&request, preflight)?;
        write_status_json(
            &request,
            "decomposition",
            "decomposition",
            "decomposition ready",
        )?;
    } else {
        ensure_decomposition_artifacts(&request)?;
        request.status = "decomposition".to_string();
        request.updated_at = now_string();
        write_status_json(
            &request,
            "decomposition",
            "decomposition",
            "decomposition ready",
        )?;
    }

    requests[index] = request.clone();
    save_requests(requests)?;
    upsert_session_for_request(&request, "decomposition", "handoff-ready")?;
    append_event(
        "decomposition_packet_created",
        &request.request_id,
        "decomposition",
        &request.status,
        &format!("change_path={}", request.change_path),
    )?;
    Ok(request)
}

fn create_plan_packet_for_index(
    requests: &mut [Request],
    index: usize,
    change_name: &str,
    preflight: &PlanPreflight,
) -> Result<Request> {
    let mut request = requests[index].clone();
    request.change_name = change_name.to_string();
    request.change_path = change_artifact_path(change_name);
    request.status = "planning".to_string();
    request.updated_at = now_string();
    generate_plan_packet(&request, preflight)?;
    requests[index] = request.clone();
    save_requests(requests)?;
    upsert_session_for_request(&request, "planning", "handoff-ready")?;
    append_event(
        "change_packet_created",
        &request.request_id,
        "planning",
        &request.status,
        &format!("change_path={}", request.change_path),
    )?;
    Ok(request)
}

fn submit_approval(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id", "--gate"])?;
    let request_id = required_request_id(args)?;
    let gate = required_gate(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    request.status = format!("{}-submitted", gate_status_prefix(&gate));
    request.updated_at = now_string();
    mark_phase_document_submitted(&request, gate_agent_phase(&gate))?;
    write_approval_record(&request, &gate, "submitted", "", "manual-cli", "")?;
    requests[index] = request.clone();
    save_requests(&requests)?;
    update_gate_session(&request, &gate, "waiting-approval")?;

    println!("Gate submitted for {request_id}");
    println!("  gate: {gate}");
    println!("  status: {}", request.status);
    println!("  state: {}/status.json", request.change_path);
    Ok(())
}

fn decide_approval(args: &[String], decision: &str) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--request_id",
            "--request-id",
            "--gate",
            "--by",
            "--comment",
            "--source",
        ],
    )?;
    let request_id = required_request_id(args)?;
    let gate = required_gate(args)?;
    let by = required_flag(args, "--by")?;
    let comment = flag_value(args, "--comment")?.unwrap_or_default();
    let source = flag_value(args, "--source")?.unwrap_or_else(|| "manual-cli".to_string());
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    request.status = format!("{}-{decision}", gate_status_prefix(&gate));
    request.updated_at = now_string();
    write_approval_record(&request, &gate, decision, &by, &source, &comment)?;
    requests[index] = request.clone();
    save_requests(&requests)?;
    update_gate_session(&request, &gate, decision)?;

    println!("Gate decision recorded for {request_id}");
    println!("  gate: {gate}");
    println!("  status: {decision}");
    println!("  state: {}/status.json", request.change_path);
    Ok(())
}

fn show_gates(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id", "--gate", "--json"])?;
    let request_id = required_request_id(args)?;
    let gate = flag_value(args, "--gate")?;
    if let Some(gate) = &gate {
        validate_gate(gate)?;
    }
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    ensure_change_packet(request)?;

    let gates = gate.map_or_else(
        || {
            vec![
                "decomposition".to_string(),
                "plan".to_string(),
                "change-doc".to_string(),
            ]
        },
        |value| vec![value],
    );
    if flag_present(args, "--json") {
        println!("{{");
        println!("  \"request_id\": \"{}\",", json_escape(&request_id));
        println!("  \"gates\": [");
        for (index, gate) in gates.iter().enumerate() {
            if index > 0 {
                println!(",");
            }
            print!(
                "{}",
                indent_json_object(&render_gate_record_json(request, gate)?, 4)
            );
        }
        println!();
        println!("  ]");
        println!("}}");
    } else {
        for gate in gates {
            let record = render_gate_record_json(request, &gate)?;
            let status = json_value(&record, "status").unwrap_or_else(|| "missing".to_string());
            let artifact = json_value(&record, "artifact").unwrap_or_default();
            println!(
                "{:<14} {:<12} {}",
                gate,
                status,
                fallback_empty(&artifact, "n/a")
            );
        }
    }
    Ok(())
}

fn start_worktree(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let _lock = acquire_request_lock_wait(&request_id, "start")?;
    start_worktree_inner(args)
}

fn start_worktree_inner(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let config = load_config()?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();

    if request.change_path.is_empty() {
        return Err(format!(
            "{} has no change packet. Run: sandrone plan --name {}-short-name --request_id {}",
            request.request_id,
            today(),
            request.request_id
        )
        .into());
    }
    if is_parent_request(&request)
        && existing_or_preferred_request_artifact_path(&request, "decomposition.md").exists()
    {
        ensure_gate_approved(&request, "decomposition").map_err(|error| {
            format!(
                "{} requires decomposition gate before direct implementation: {error}",
                request.request_id
            )
        })?;
        return Err(format!(
            "{} is a parent request with an approved decomposition. Start a materialized slice request instead.",
            request.request_id
        )
        .into());
    }
    ensure_gate_approved(&request, "plan")?;

    let branch = format!("codex/{}", request.request_id.to_lowercase());
    let worktree_path = Path::new(WORKTREES).join(&request.request_id);
    fs::create_dir_all(WORKTREES)?;
    let absolute_worktree = env::current_dir()?.join(&worktree_path);
    let absolute_worktree_string = absolute_worktree.to_string_lossy().to_string();

    let existing = git_output(DEV_REPO, &["worktree", "list", "--porcelain"])?;
    if !existing.contains(&format!("worktree {absolute_worktree_string}")) {
        match pull_target_repo_before_worktree_creation() {
            Ok(outcome) => print_worktree_pull_outcome(&outcome),
            Err(error) => {
                let reason = error.to_string();
                mark_blocked(
                    &mut requests,
                    index,
                    &mut request,
                    "implementation",
                    &reason,
                )?;
                return Err(reason.into());
            }
        }
        if git_output(DEV_REPO, &["rev-parse", "--verify", "HEAD"]).is_ok() {
            let base_ref =
                if git_output(DEV_REPO, &["rev-parse", "--verify", &config.base_branch]).is_ok() {
                    config.base_branch.clone()
                } else {
                    "HEAD".to_string()
                };
            run_command(
                Command::new("git")
                    .args([
                        "worktree",
                        "add",
                        "-B",
                        &branch,
                        &absolute_worktree_string,
                        &base_ref,
                    ])
                    .current_dir(DEV_REPO),
            )?;
        } else {
            fs::create_dir_all(&absolute_worktree)?;
            run_command(
                Command::new("git")
                    .arg("init")
                    .current_dir(&absolute_worktree),
            )?;
            run_command(
                Command::new("git")
                    .args(["checkout", "-B", &branch])
                    .current_dir(&absolute_worktree),
            )?;
            if let Ok(origin) = git_output(DEV_REPO, &["remote", "get-url", "origin"]) {
                run_command(
                    Command::new("git")
                        .args(["remote", "add", "origin", &origin])
                        .current_dir(&absolute_worktree),
                )?;
            }
        }
    }

    request.branch = branch;
    request.worktree_path = worktree_path.to_string_lossy().to_string();
    request.status = "in-progress".to_string();
    request.updated_at = now_string();
    generate_start_packet(&request)?;
    requests[index] = request.clone();
    save_requests(&requests)?;
    upsert_session_for_request(&request, "implementation", "handoff-ready")?;

    println!("Worktree ready for {}", request.request_id);
    println!("  worktree: {}", request.worktree_path);
    println!("  branch: {}", request.branch);
    println!(
        "  change doc: {}",
        request_artifact_path_string(&request, "change-doc.md")
    );
    println!("  Codex must implement in the worktree and stop before finish.");
    Ok(())
}

fn finish_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id", "--message"])?;
    let request_id = required_request_id(args)?;
    let commit_message = flag_value(args, "--message")?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if matches!(
        canonical_status(&request.status),
        STATUS_WAIT_FINISH | STATUS_FINISHED
    ) {
        let report = run_delivery_pr_status_check(&mut requests, index, &mut request)?;
        print_pr_status_result(&request, &report);
        return Ok(());
    }
    ensure_gate_approved(&request, "change-doc")?;
    let commit_message = commit_message.unwrap_or_else(|| default_commit_message(&request));
    validate_commit_message(&commit_message)?;
    let delivery = deliver_finished_request(&request, &commit_message)?;
    let next_status = if delivery.pr_url.is_some() {
        STATUS_WAIT_FINISH
    } else {
        STATUS_WAIT_UPDATE_PR
    };
    request.status = next_status.to_string();
    request.updated_at = now_string();
    let worktree_path = request.worktree_path.clone();
    let branch = request.branch.clone();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(
        &request,
        "delivery",
        next_status,
        if delivery.pr_url.is_some() {
            "PR created or reused; waiting for merge check"
        } else {
            "PR creation failed or skipped; waiting for PR creation/update retry"
        },
    )?;
    append_event(
        "finish_delivery",
        &request.request_id,
        "delivery",
        next_status,
        if delivery.pr_url.is_some() {
            "PR created or reused; waiting for merge check"
        } else {
            "PR creation failed or skipped"
        },
    )?;
    upsert_session_for_request(&request, "implementation", next_status)?;

    if next_status == STATUS_WAIT_FINISH {
        println!("{request_id} marked wait-finish.");
    } else {
        println!("{request_id} remains wait-update-pr.");
    }
    println!(
        "  change doc: {}",
        request_artifact_path_string(&request, "change-doc.md")
    );
    println!("  worktree: {worktree_path}");
    println!("  branch: {branch}");
    if delivery.committed {
        println!("  committed: {}", delivery.commit_message);
    } else {
        println!("  no new commit: worktree had no file changes");
    }
    if delivery.pushed_with_force_lease {
        println!(
            "  pushed branch with --force-with-lease: {}",
            delivery.branch
        );
    } else {
        println!("  pushed branch: {}", delivery.branch);
    }
    if let Some(pr_url) = delivery.pr_url {
        if delivery.pr_status == "existing" {
            println!("  PR already exists: {pr_url}");
        } else {
            println!("  PR created: {pr_url}");
        }
    } else if let Some(compare_url) = delivery.compare_url {
        println!("  PR creation failed or skipped.");
        println!("  manual PR link: {compare_url}");
        if !delivery.pr_error.is_empty() {
            println!("  PR error: {}", delivery.pr_error);
        }
    } else {
        println!("  PR creation skipped: {}", delivery.pr_error);
    }
    Ok(())
}

fn pr_status_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    let report = run_delivery_pr_status_check(&mut requests, index, &mut request)?;
    print_pr_status_result(&request, &report);
    Ok(())
}

fn run_delivery_pr_status_check(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
) -> Result<PrStatusReport> {
    ensure_refreshable_request(request)?;
    let config = load_config()?;
    let report = run_pr_status_tool(request, &config)?;
    let (next_status, reason) = if report.status == "merged" {
        (
            STATUS_FINISHED.to_string(),
            format!(
                "PR status confirmed merged by {PR_STATUS_TOOL}: {}",
                report.raw
            ),
        )
    } else if report.status == "open" {
        (
            STATUS_WAIT_FINISH.to_string(),
            format!("PR status is open; waiting for merge: {}", report.raw),
        )
    } else if matches!(report.status.as_str(), "missing" | "closed") {
        (
            STATUS_WAIT_UPDATE_PR.to_string(),
            format!(
                "PR status is {}; PR needs creation or update: {}",
                report.status, report.raw
            ),
        )
    } else {
        (
            request.status.clone(),
            format!(
                "PR status could not confirm merge; keeping current state: {}",
                report.raw
            ),
        )
    };
    if request.status != next_status {
        request.status = next_status.clone();
        request.updated_at = now_string();
        requests[index] = request.clone();
        save_requests(requests)?;
        write_status_json(request, "delivery", &next_status, &reason)?;
        append_event(
            "pr_status_checked",
            &request.request_id,
            "delivery",
            &next_status,
            &reason,
        )?;
        upsert_session_for_request(request, "implementation", &next_status)?;
    } else {
        append_event(
            "pr_status_checked",
            &request.request_id,
            "delivery",
            &request.status,
            &reason,
        )?;
    }
    Ok(report)
}

fn print_pr_status_result(request: &Request, report: &PrStatusReport) {
    println!("PR status for {}: {}", request.request_id, report.status);
    if !report.url.trim().is_empty() {
        println!("  url: {}", report.url);
    }
    if !report.detail.trim().is_empty() {
        println!("  detail: {}", report.detail);
    }
    println!("  request status: {}", request.status);
    if request.status == STATUS_FINISHED {
        println!("  merged PR confirmed; request marked finished.");
    } else if request.status == STATUS_WAIT_FINISH {
        println!("  PR is not merged yet; request remains wait-finish.");
    } else if request.status == STATUS_WAIT_UPDATE_PR {
        println!("  PR needs creation or update; request remains wait-update-pr.");
    } else {
        println!("  PR merge was not confirmed; request state was not promoted.");
    }
}

fn pr_refresh_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &["--request_id", "--request-id", "--mode", "--max-attempts"],
    )?;
    let request_id = required_request_id(args)?;
    let mode = flag_value(args, "--mode")?.unwrap_or_else(|| "start".to_string());
    let max_attempts = parse_max_attempts(flag_value(args, "--max-attempts")?)?;
    let Some(_lock) = RequestLock::acquire(&request_id)? else {
        println!("PR refresh skipped for {request_id}: request lock is already held.");
        return Ok(());
    };

    match mode.as_str() {
        "start" => start_pr_refresh(&request_id, max_attempts),
        "continue" => continue_pr_refresh(&request_id),
        _ => Err("--mode must be `start` or `continue`".into()),
    }
}

fn start_pr_refresh(request_id: &str, max_attempts: Option<u32>) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_refreshable_request(&request)?;
    ensure_gate_approved(&request, "change-doc")?;
    let resolved_max_attempts = resolve_max_attempts(AgentPhase::Rebase, max_attempts);
    if review_attempts_exhausted(&request, AgentPhase::Rebase, resolved_max_attempts)? {
        let reason = format!(
            "integration-review failed after {resolved_max_attempts} attempt(s); manual recovery is required"
        );
        mark_blocked(&mut requests, index, &mut request, "rebase", &reason)?;
        return Err(reason.into());
    }

    let config = load_config()?;
    let pr_status = run_pr_status_tool(&request, &config)?;
    if pr_status.status == "merged" {
        request.status = STATUS_FINISHED.to_string();
        request.updated_at = now_string();
        requests[index] = request.clone();
        save_requests(&requests)?;
        write_status_json(
            &request,
            "delivery",
            STATUS_FINISHED,
            &format!(
                "PR already merged before refresh; confirmed by {PR_STATUS_TOOL}: {}",
                pr_status.raw
            ),
        )?;
        append_event(
            "pr_refresh_skipped_merged",
            &request.request_id,
            "delivery",
            STATUS_FINISHED,
            &pr_status.raw,
        )?;
        upsert_session_for_request(&request, "implementation", STATUS_FINISHED)?;
        println!("PR refresh skipped for {request_id}: PR is already merged.");
        if !pr_status.url.trim().is_empty() {
            println!("  url: {}", pr_status.url);
        }
        return Ok(());
    }
    let worktree = Path::new(&request.worktree_path);
    ensure_clean_worktree_for_rebase(worktree)?;
    let base_ref = fetch_base_ref(worktree, &config.base_branch)?;
    let before_head = git_output(&request.worktree_path, &["rev-parse", "HEAD"])?;
    let before_patch = integration_patch_stat(&request.worktree_path, &base_ref);
    let output = Command::new("git")
        .args(["rebase", &base_ref])
        .current_dir(worktree)
        .envs(proxy_env())
        .output()?;

    if output.status.success() {
        let after_head = git_output(&request.worktree_path, &["rev-parse", "HEAD"])?;
        let after_patch = integration_patch_stat(&request.worktree_path, &base_ref);
        let detail = format!(
            "Clean rebase completed without conflicts.\n\nBefore patch stat:\n{}\n\nAfter patch stat:\n{}",
            before_patch, after_patch
        );
        append_integration_record(
            &request,
            &IntegrationRecord {
                mode: "clean-rebase",
                base_branch: &config.base_branch,
                base_ref: &base_ref,
                before_head: &before_head,
                after_head: &after_head,
                pr_status: &pr_status.raw,
                detail: &detail,
            },
        )?;
        mark_integration_review_submitted(
            &mut requests,
            index,
            &mut request,
            "clean rebase completed",
        )?;
        println!("PR refresh clean rebase completed for {request_id}.");
        run_integration_review_from_tick(request_id)?;
        println!("Integration review completed for {request_id}.");
        Ok(())
    } else {
        let detail = review_diagnostic_excerpt(&format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
        let record_detail = format!(
            "Rebase stopped with conflicts or another integration error.\n\nDiagnostic: {detail}\n\nRebaseAgent must preserve both base/master changes and request branch changes."
        );
        let attempt = append_pr_conflict_record(
            &request,
            &IntegrationRecord {
                mode: "conflict-rebase",
                base_branch: &config.base_branch,
                base_ref: &base_ref,
                before_head: &before_head,
                after_head: &before_head,
                pr_status: &pr_status.raw,
                detail: &record_detail,
            },
        )?;
        request.status = AgentPhase::Rebase.running_status().to_string();
        request.updated_at = now_string();
        requests[index] = request.clone();
        save_requests(&requests)?;
        write_status_json(
            &request,
            "rebase",
            AgentPhase::Rebase.running_status(),
            &detail,
        )?;
        upsert_session_for_request(&request, "rebase", AgentPhase::Rebase.running_status())?;
        reset_phase_document_for_agent_dispatch(&request, AgentPhase::Rebase)?;
        let pid = spawn_issue_agent(&request, resolved_max_attempts, AgentPhase::Rebase, None)?;
        append_event(
            "rebase_agent_dispatched",
            &request.request_id,
            "rebase",
            AgentPhase::Rebase.running_status(),
            &format!("pid={pid}; base_ref={base_ref}; detail={detail}"),
        )?;
        println!("PR refresh rebase conflict for {request_id}.");
        println!("  conflict record: {attempt:03}");
        println!("  rebase-agent pid: {pid}");
        println!("  worktree: {}", request.worktree_path);
        println!(
            "  logs: {} | {}",
            agent_stdout_path(request_id).display(),
            agent_stderr_path(request_id).display()
        );
        Ok(())
    }
}

fn continue_pr_refresh(request_id: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_refreshable_request(&request)?;
    ensure_rebase_ready_for_integration_review(&request)?;
    append_integration_record(
        &request,
        &IntegrationRecord {
            mode: "manual-continue",
            base_branch: "",
            base_ref: "",
            before_head: "",
            after_head: "",
            pr_status: "manual continue",
            detail: "Manual or external rebase conflict resolution completed; running IntegrationReviewer.",
        },
    )?;
    mark_integration_review_submitted(
        &mut requests,
        index,
        &mut request,
        "manual pr-refresh continue",
    )?;
    run_integration_review_from_tick(request_id)?;
    println!("PR refresh continue completed for {request_id}.");
    Ok(())
}

fn ensure_refreshable_request(request: &Request) -> Result<()> {
    ensure_change_packet(request)?;
    ensure_gate_approved(request, "plan")?;
    if request.worktree_path.trim().is_empty() {
        return Err(format!(
            "{} has no worktree. Run sandrone start first.",
            request.request_id
        )
        .into());
    }
    if request.branch.trim().is_empty() {
        return Err(format!(
            "{} has no branch. Run sandrone start first.",
            request.request_id
        )
        .into());
    }
    if !Path::new(&request.worktree_path).exists() {
        return Err(format!("worktree does not exist: {}", request.worktree_path).into());
    }
    Ok(())
}

fn ensure_clean_worktree_for_rebase(worktree: &Path) -> Result<()> {
    let worktree_string = worktree.to_string_lossy().to_string();
    let changes = git_output(&worktree_string, &["status", "--porcelain"])?;
    if !changes.trim().is_empty() {
        return Err(format!(
            "worktree must be clean before pr-refresh rebase. Commit, discard, or resolve changes first:\n{changes}"
        )
        .into());
    }
    Ok(())
}

fn fetch_base_ref(worktree: &Path, base_branch: &str) -> Result<String> {
    run_command(
        Command::new("git")
            .args(["fetch", "origin", base_branch])
            .current_dir(worktree)
            .envs(proxy_env()),
    )?;
    let remote_ref = format!("origin/{base_branch}");
    let worktree_string = worktree.to_string_lossy().to_string();
    if git_output(&worktree_string, &["rev-parse", "--verify", &remote_ref]).is_ok() {
        Ok(remote_ref)
    } else {
        Ok(base_branch.to_string())
    }
}

fn integration_patch_stat(worktree: &str, base_ref: &str) -> String {
    git_output(worktree, &["diff", "--stat", &format!("{base_ref}...HEAD")])
        .unwrap_or_else(|error| format!("unable to compute patch stat: {error}"))
}

fn run_pr_status_tool(request: &Request, config: &Config) -> Result<PrStatusReport> {
    if !Path::new(PR_STATUS_TOOL).exists() {
        let raw = format!("unknown\t\t{PR_STATUS_TOOL} missing");
        return Ok(parse_pr_status_report(&raw));
    }
    let compare_url = github_compare_url(&config.git_url, &config.base_branch, &request.branch)
        .unwrap_or_default();
    let output = Command::new("sh")
        .arg(PR_STATUS_TOOL)
        .current_dir(".")
        .env("SANDRONE_REQUEST_ID", &request.request_id)
        .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("SANDRONE_REQUEST_SOURCE", &request.source)
        .env("SANDRONE_REQUEST_TITLE", &request.title)
        .env("SANDRONE_REQUEST_URL", &request.url)
        .env("SANDRONE_CHANGE_PATH", &request.change_path)
        .env("SANDRONE_WORKTREE", &request.worktree_path)
        .env("SANDRONE_PR_BASE", &config.base_branch)
        .env("SANDRONE_PR_HEAD", &request.branch)
        .env("SANDRONE_PR_COMPARE_URL", compare_url)
        .envs(proxy_env())
        .output();
    let status_path = Path::new(".sandrone")
        .join("state")
        .join(format!("{}-pr-status.tsv", request.request_id));
    let status = match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8(output.stdout)?;
            stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("unknown\t\tpr-status returned no output")
                .to_string()
        }
        Ok(output) => format!(
            "unknown\t\t{}",
            review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr))
        ),
        Err(error) => format!("unknown\t\t{error}"),
    };
    fs::write(status_path, ensure_trailing_newline(&status))?;
    Ok(parse_pr_status_report(&status))
}

fn parse_pr_status_report(line: &str) -> PrStatusReport {
    let fields: Vec<&str> = line.split('\t').collect();
    PrStatusReport {
        status: fields
            .first()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string()),
        url: fields
            .get(1)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        detail: fields
            .get(2)
            .map(|value| value.trim().to_string())
            .unwrap_or_default(),
        raw: line.trim().to_string(),
    }
}

fn append_integration_record(request: &Request, record: &IntegrationRecord<'_>) -> Result<()> {
    let timestamp = now_string();
    let path = existing_or_preferred_request_artifact_path(request, "change-doc.md");
    let mut content = fs::read_to_string(&path)?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n## PR 集成刷新记录\n\n- 时间: `{}`\n- 模式: `{}`\n- Base branch: `{}`\n- Base ref: `{}`\n- Rebase 前 HEAD: `{}`\n- Rebase 后 HEAD: `{}`\n- PR 状态脚本: `{}`\n\n### 集成处理要求\n\n- RebaseAgent 和 IntegrationReviewer 必须确认冲突解决保留 base/master 新代码，也保留 request 分支已通过 review 的实现语义。\n- 不能为了自己分支的修改删除 base/master 新代码；如果必须替换，需要在本节写明原因、影响和验证证据。\n\n### 集成细节\n\n{}\n",
        timestamp,
        markdown_inline(record.mode),
        markdown_inline(record.base_branch),
        markdown_inline(record.base_ref),
        markdown_inline(record.before_head),
        markdown_inline(record.after_head),
        markdown_inline(record.pr_status),
        record.detail.trim_end(),
    ));
    fs::write(path, content)?;
    Ok(())
}

fn append_pr_conflict_record(request: &Request, record: &IntegrationRecord<'_>) -> Result<u32> {
    let timestamp = now_string();
    let attempt = next_pr_conflict_attempt(request)?;
    let attempt_path = pr_conflict_attempt_path(request, attempt);
    if let Some(parent) = attempt_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let detail = record.detail.trim_end();
    let record_content = format!(
        "# PR 冲突记录 Attempt {attempt:03}\n\n- Request ID: `{}`\n- 时间: `{}`\n- 模式: `{}`\n- Base branch: `{}`\n- Base ref: `{}`\n- Rebase 前 HEAD: `{}`\n- PR 状态脚本: `{}`\n\n## 冲突诊断\n\n{}\n\n## 处理约束\n\n- RebaseAgent 必须同时保留 base/master 新代码和 request 分支已通过 review 的实现语义。\n- 不得为了消除冲突直接删除 master 上的新代码；如确需替换，必须说明原因、影响和验证证据。\n- 冲突解决后必须通过 IntegrationReviewer，并在 `change-doc.md` 中记录解决方式、实现前后对比和验证结果。\n",
        request.request_id,
        timestamp,
        markdown_inline(record.mode),
        markdown_inline(record.base_branch),
        markdown_inline(record.base_ref),
        markdown_inline(record.before_head),
        markdown_inline(record.pr_status),
        detail,
    );
    fs::write(&attempt_path, record_content)?;

    let change_doc_path = existing_or_preferred_request_artifact_path(request, "change-doc.md");
    let mut change_doc = fs::read_to_string(&change_doc_path)?;
    if !change_doc.ends_with('\n') {
        change_doc.push('\n');
    }
    change_doc.push_str(&format!(
        "\n## PR 冲突记录 (Attempt {attempt:03})\n\n- 冲突记录: `{}`\n- 时间: `{}`\n- Base branch: `{}`\n- Base ref: `{}`\n- Rebase 前 HEAD: `{}`\n- PR 状态脚本: `{}`\n\n### 冲突诊断\n\n{}\n",
        markdown_inline(&attempt_path.to_string_lossy()),
        timestamp,
        markdown_inline(record.base_branch),
        markdown_inline(record.base_ref),
        markdown_inline(record.before_head),
        markdown_inline(record.pr_status),
        detail,
    ));
    fs::write(change_doc_path, change_doc)?;
    Ok(attempt)
}

fn next_pr_conflict_attempt(request: &Request) -> Result<u32> {
    let dir = pr_conflict_attempts_dir(request);
    if !dir.exists() {
        return Ok(1);
    }
    let mut max_attempt = 0u32;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        let Some((attempt_text, _rest)) = filename.split_once('-') else {
            continue;
        };
        let Ok(attempt) = attempt_text.parse::<u32>() else {
            continue;
        };
        max_attempt = max_attempt.max(attempt);
    }
    Ok(max_attempt + 1)
}

fn pr_conflict_attempts_dir(request: &Request) -> PathBuf {
    Path::new(&request.change_path)
        .join("pr-conflicts")
        .join("attempts")
}

fn pr_conflict_attempt_path(request: &Request, attempt: u32) -> PathBuf {
    pr_conflict_attempts_dir(request).join(format!("{attempt:03}-rebase-conflict.md"))
}

fn mark_integration_review_submitted(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    reason: &str,
) -> Result<()> {
    request.status = "integration-review-submitted".to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, "rebase", "integration-review-submitted", reason)?;
    append_event(
        "integration_review_submitted",
        &request.request_id,
        "rebase",
        "integration-review-submitted",
        reason,
    )?;
    upsert_session_for_request(request, "rebase", "waiting-review")
}

fn ensure_rebase_ready_for_integration_review(request: &Request) -> Result<()> {
    if git_internal_path(&request.worktree_path, "rebase-merge").exists()
        || git_internal_path(&request.worktree_path, "rebase-apply").exists()
    {
        return Err(
            "rebase is still in progress; complete or abort it before integration-review".into(),
        );
    }
    let unmerged = git_output(
        &request.worktree_path,
        &["diff", "--name-only", "--diff-filter=U"],
    )?;
    if !unmerged.trim().is_empty() {
        return Err(format!("unmerged conflict files remain:\n{unmerged}").into());
    }
    Ok(())
}

fn git_internal_path(worktree: &str, name: &str) -> PathBuf {
    let path = git_output(worktree, &["rev-parse", "--git-path", name])
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(".git").join(name));
    if path.is_absolute() {
        path
    } else {
        Path::new(worktree).join(path)
    }
}

fn block_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &["--request_id", "--request-id", "--stage", "--reason"],
    )?;
    let request_id = required_request_id(args)?;
    let stage = required_flag(args, "--stage")?;
    let reason = required_flag(args, "--reason")?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    mark_blocked(&mut requests, index, &mut request, &stage, &reason)?;
    println!("Request {request_id} blocked.");
    println!("  stage: {stage}");
    println!("  reason: {reason}");
    println!(
        "  recovery: {}",
        request_artifact_path_string(&request, "recovery.md")
    );
    Ok(())
}

fn resume_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    let resumed_phase = if request.status == "blocked" {
        let target = blocked_resume_target(&request)?;
        request.status = target.status.to_string();
        request.updated_at = now_string();
        requests[index] = request.clone();
        save_requests(&requests)?;
        write_status_json(
            &request,
            target.phase.as_str(),
            target.status,
            &target.reason,
        )?;
        append_event(
            "request_resumed",
            &request.request_id,
            target.phase.as_str(),
            target.status,
            &target.reason,
        )?;
        upsert_session_for_request(&request, target.phase.as_str(), target.status)?;
        Some((target.phase, target.status.to_string()))
    } else {
        None
    };
    println!("Resume package for {}", request.request_id);
    println!(
        "  request: {}",
        request_artifact_path_string(&request, "request.md")
    );
    println!(
        "  plan: {}",
        request_artifact_path_string(&request, "plan.md")
    );
    println!(
        "  change doc: {}",
        request_artifact_path_string(&request, "change-doc.md")
    );
    println!(
        "  agent journal: {}",
        request_artifact_path_string(&request, "agent-journal.md")
    );
    println!("  status: {}/status.json", request.change_path);
    println!(
        "  recovery: {}",
        request_artifact_path_string(&request, "recovery.md")
    );
    println!(
        "  reviews: {}/reviews/plan-review/summary.json and {}/reviews/code-review/summary.json",
        request.change_path, request.change_path
    );
    println!(
        "  worktree: {}",
        fallback_empty(&request.worktree_path, "not started")
    );
    println!(
        "  branch: {}",
        fallback_empty(&request.branch, "not started")
    );
    if let Some((phase, status)) = resumed_phase {
        println!("  resumed phase: {}", phase.as_str());
        println!("  resumed status: {status}");
    } else {
        println!("  resumed status: {}", request.status);
    }
    println!("  next: sandrone tick --request_id {}", request.request_id);
    Ok(())
}

fn register_session(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--request_id",
            "--request-id",
            "--phase",
            "--thread_id",
            "--thread-id",
            "--thread_url",
            "--thread-url",
            "--status",
        ],
    )?;
    let request_id = required_request_id(args)?;
    let phase = required_flag(args, "--phase")?;
    validate_session_phase(&phase)?;
    let thread_id = flag_value(args, "--thread_id")?
        .or(flag_value(args, "--thread-id")?)
        .unwrap_or_default();
    let thread_url = flag_value(args, "--thread_url")?
        .or(flag_value(args, "--thread-url")?)
        .unwrap_or_default();
    let status = flag_value(args, "--status")?.unwrap_or_else(|| "registered".to_string());
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;

    let mut session = session_from_request(request, &phase, &status)?;
    session.thread_id = thread_id;
    session.thread_url = thread_url;
    upsert_session(session)?;
    println!("Session registered for {request_id}");
    println!("  phase: {phase}");
    println!("  status: {status}");
    Ok(())
}

fn assess_repository_before_planning() -> Result<PlanPreflight> {
    let mut notes = Vec::new();
    if !repo_has_commits(DEV_REPO) {
        notes.push("目标仓库为空: 不需要 git pull，跳过 CodeGraph，等待用户需求即可。".to_string());
        return Ok(PlanPreflight { notes });
    }

    if remote_exists(DEV_REPO) {
        if let Err(error) = fetch_if_remote_exists() {
            notes.push(format!(
                "git fetch 未成功，Codex 必须在计划前人工判断是否需要 git pull: {error}"
            ));
        } else if upstream_is_ahead(DEV_REPO)? {
            return Err("git pull required before planning: remote contains commits that are not in dev/repo".into());
        } else {
            notes.push("git pull 检查通过: 当前本地分支没有落后于 upstream。".to_string());
        }
    } else {
        notes.push("未检测到 git remote: Codex 必须人工判断是否需要同步代码。".to_string());
    }

    let codegraph_outcome = ensure_codegraph_initialized(DEV_REPO);
    notes.push(codegraph_preflight_note(&codegraph_outcome));

    if codegraph_refresh_required()? {
        let context_outcome = refresh_codegraph_context(DEV_REPO);
        notes.push(codegraph_context_preflight_note(&context_outcome));
    } else {
        notes.push(
            "CodeGraph 检查通过: obsidian/codegraph/context.md 看起来不早于最新提交。".to_string(),
        );
    }

    Ok(PlanPreflight { notes })
}

fn list_sessions(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--json"])?;
    ensure_sessions_file()?;
    if flag_present(args, "--json") {
        println!("{}", fs::read_to_string(SESSIONS_PATH)?);
        return Ok(());
    }

    let sessions = load_sessions()?;
    if sessions.is_empty() {
        println!("No sessions yet.");
        return Ok(());
    }
    for session in sessions {
        println!(
            "{:<9} {:<15} {:<18} {}",
            session.request_id,
            session.phase,
            session.status,
            fallback_empty(&session.thread_url, "n/a")
        );
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct ResumeTarget {
    phase: AgentPhase,
    status: &'static str,
    reason: String,
}

fn blocked_resume_target(request: &Request) -> Result<ResumeTarget> {
    let status_content =
        fs::read_to_string(Path::new(&request.change_path).join("status.json")).unwrap_or_default();
    let blocked_stage = json_value(&status_content, "stage").unwrap_or_default();
    let blocked_reason = json_value(&status_content, "reason").unwrap_or_default();
    let phase = blocked_stage_to_agent_phase(request, &blocked_stage);

    if phase == AgentPhase::Implementation && ensure_gate_approved(request, "plan").is_err() {
        return Ok(ResumeTarget {
            phase: AgentPhase::Planning,
            status: "planning",
            reason: "resumed from blocked; plan gate is not approved, returning to planning"
                .to_string(),
        });
    }
    if phase == AgentPhase::Rebase && ensure_gate_approved(request, "change-doc").is_err() {
        return Ok(ResumeTarget {
            phase: AgentPhase::Implementation,
            status: "code-review-rejected",
            reason:
                "resumed from blocked; change-doc gate is not approved, returning to implementation"
                    .to_string(),
        });
    }

    if blocked_came_from_review_gate_unavailable(request, phase, &blocked_reason) {
        let status = submitted_status_for_phase(phase);
        return Ok(ResumeTarget {
            phase,
            status,
            reason: format!(
                "resumed from blocked; reviewer gate was unavailable, rerunning {}",
                review_stage_for_phase(phase)
            ),
        });
    }

    if blocked_stage == "blocked" || blocked_stage.is_empty() {
        return fallback_resume_target(request);
    }

    Ok(ResumeTarget {
        phase,
        status: rejected_status_for_phase(phase),
        reason: format!(
            "resumed from blocked; previous block requires {}",
            phase.as_str()
        ),
    })
}

fn fallback_resume_target(request: &Request) -> Result<ResumeTarget> {
    if is_parent_request(request) && ensure_gate_approved(request, "decomposition").is_err() {
        return Ok(ResumeTarget {
            phase: AgentPhase::Planning,
            status: "planning",
            reason: "resumed from blocked; decomposition gate is not approved".to_string(),
        });
    }
    if ensure_gate_approved(request, "plan").is_ok() {
        Ok(ResumeTarget {
            phase: AgentPhase::Implementation,
            status: "code-review-rejected",
            reason: "resumed from blocked; implementation must be repaired or resubmitted"
                .to_string(),
        })
    } else {
        Ok(ResumeTarget {
            phase: AgentPhase::Planning,
            status: "planning",
            reason: "resumed from blocked; plan gate is not approved".to_string(),
        })
    }
}

fn blocked_stage_to_agent_phase(request: &Request, stage: &str) -> AgentPhase {
    match stage {
        "decomposition" | "decomposition-review" => AgentPhase::Decomposition,
        "planning" | "plan-review" => AgentPhase::Planning,
        "rebase" | "integration-review" => AgentPhase::Rebase,
        "implementation" | "code-review" | "agent" => AgentPhase::Implementation,
        _ if is_parent_request(request) => AgentPhase::Decomposition,
        _ if ensure_gate_approved(request, "plan").is_ok() => AgentPhase::Implementation,
        _ => AgentPhase::Planning,
    }
}

fn blocked_came_from_review_gate_unavailable(
    request: &Request,
    phase: AgentPhase,
    blocked_reason: &str,
) -> bool {
    let reason = blocked_reason.to_lowercase();
    if reason.contains("gate unavailable") || reason.contains("reviewer backend") {
        return true;
    }
    let summary_path = Path::new(&request.change_path)
        .join("reviews")
        .join(review_stage_for_phase(phase))
        .join("summary.json");
    fs::read_to_string(summary_path)
        .map(|content| json_bool(&content, "gate_unavailable").unwrap_or(false))
        .unwrap_or(false)
}

fn submitted_status_for_phase(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Decomposition => "decomposition-submitted",
        AgentPhase::Planning => "plan-submitted",
        AgentPhase::Implementation => "change-doc-submitted",
        AgentPhase::Rebase => "integration-review-submitted",
    }
}

fn rejected_status_for_phase(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Decomposition => "decomposition-review-rejected",
        AgentPhase::Planning => "plan-review-rejected",
        AgentPhase::Implementation => "code-review-rejected",
        AgentPhase::Rebase => "integration-review-rejected",
    }
}

fn review_stage_for_phase(phase: AgentPhase) -> &'static str {
    match phase {
        AgentPhase::Decomposition => "decomposition-review",
        AgentPhase::Planning => "plan-review",
        AgentPhase::Implementation => "code-review",
        AgentPhase::Rebase => "integration-review",
    }
}

fn upgrade_workspace(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--dry-run", "--default"])?;
    let dry_run = flag_present(args, "--dry-run");
    let install_defaults = flag_present(args, "--default");
    let config = load_config()?;
    let mut requests = load_requests()?;

    if dry_run {
        println!("Upgrade dry run:");
    } else {
        println!("Upgrade started:");
        prepare_workspace_dirs()?;
        ensure_state_file()?;
    }

    if config.schema_version < FRAMEWORK_SCHEMA_VERSION {
        if dry_run {
            println!(
                "Would update {CONFIG_PATH} schema_version: {} -> {}",
                config.schema_version, FRAMEWORK_SCHEMA_VERSION
            );
        } else {
            rewrite_config(&config)?;
            println!("Updated {CONFIG_PATH} schema_version to {FRAMEWORK_SCHEMA_VERSION}");
        }
    }

    if !Path::new(SESSIONS_PATH).exists() {
        if dry_run {
            println!("Would create {SESSIONS_PATH}");
        } else {
            ensure_sessions_file()?;
            println!("Created {SESSIONS_PATH}");
        }
    }

    let skill_needs_update = fs::read_to_string(WORKFLOW_SKILL)
        .map(|content| content != WORKFLOW_SKILL_CONTENT)
        .unwrap_or(true);
    if skill_needs_update {
        if dry_run {
            println!("Would update {WORKFLOW_SKILL}");
        } else {
            fs::create_dir_all(Path::new(WORKFLOW_SKILL).parent().unwrap_or(Path::new(".")))?;
            fs::write(WORKFLOW_SKILL, WORKFLOW_SKILL_CONTENT)?;
            println!("Updated {WORKFLOW_SKILL}");
        }
    }

    if dry_run {
        for path in default_reference_example_paths() {
            println!("Would refresh {path}");
        }
        if install_defaults {
            for asset in default_managed_assets() {
                println!("Would replace {} from {}", asset.path, asset.example_path);
            }
        } else {
            print_upgrade_default_asset_guidance();
        }
    } else {
        refresh_default_reference_examples()?;
        write_default_env_files()?;
        println!("Refreshed framework reference examples");
        if install_defaults {
            replace_default_runtime_assets_from_examples()?;
            println!("Replaced default runtime assets from refreshed examples");
        } else {
            print_upgrade_default_asset_guidance();
        }
    }

    migrate_legacy_change_paths(&mut requests, dry_run)?;
    if !dry_run {
        save_requests(&requests)?;
    }

    remove_legacy_agent_success_markers(dry_run)?;

    for request in &requests {
        if request.change_path.is_empty() {
            continue;
        }
        if dry_run {
            remove_legacy_approvals_dir(request, true)?;
        } else {
            migrate_legacy_approval_records(request)?;
            normalize_legacy_gate_records(request, false)?;
            remove_legacy_approvals_dir(request, false)?;
        }

        upgrade_change_artifacts(request, dry_run)?;
        migrate_request_document_status(request, dry_run)?;
        remove_obsolete_format_check_record(request, dry_run)?;
        if dry_run {
            normalize_legacy_gate_records(request, true)?;
        }

        if !dry_run {
            upsert_session_for_request(request, "planning", "handoff-ready")?;
            if !request.worktree_path.is_empty() {
                upsert_session_for_request(request, "implementation", "handoff-ready")?;
            }
        }
    }

    if !dry_run {
        ensure_sessions_file()?;
        registry::refresh_current_workspace_registry_or_warn("ready");
    }
    println!("Upgrade complete.");
    Ok(())
}

fn list_requests() -> Result<()> {
    ensure_initialized()?;
    sync_all_requests_from_status_json()?;
    registry::refresh_current_workspace_registry_or_warn("ready");
    let requests = load_requests()?;
    if requests.is_empty() {
        println!("No requests yet. Run: sandrone update");
        return Ok(());
    }
    for request in requests {
        println!(
            "{:<9} {:<12} {}",
            request.request_id, request.status, request.title
        );
    }
    Ok(())
}

fn status(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    sync_all_requests_from_status_json()?;
    registry::refresh_current_workspace_registry_or_warn("ready");
    let requests = load_requests()?;
    if args.is_empty() {
        let config = load_config()?;
        println!("repo_name: {}", config.repo_name);
        println!("git_url: {}", config.git_url);
        println!("base_branch: {}", config.base_branch);
        let mut counts = BTreeMap::<String, usize>::new();
        for request in requests {
            *counts.entry(request.status).or_default() += 1;
        }
        for (status, count) in counts {
            println!("{status}: {count}");
        }
        return Ok(());
    }

    let request_id = &args[0];
    let request = requests
        .iter()
        .find(|request| request.request_id == *request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    println!("request_id: {}", request.request_id);
    println!("external_id: {}", request.external_id);
    println!("source: {}", request.source);
    println!("status: {}", request.status);
    println!("title: {}", request.title);
    println!("url: {}", fallback_empty(&request.url, "n/a"));
    println!("change_name: {}", request.change_name);
    println!("change_path: {}", request.change_path);
    println!("branch: {}", request.branch);
    println!("worktree_path: {}", request.worktree_path);
    Ok(())
}

fn validate() -> Result<()> {
    ensure_initialized()?;
    let requests = load_requests()?;
    for request in requests
        .iter()
        .filter(|request| !request.change_path.is_empty())
    {
        for file in request_required_runtime_artifacts(request) {
            let path = if file == "status.json" {
                Path::new(&request.change_path).join(file)
            } else {
                existing_or_preferred_request_artifact_path(request, file)
            };
            if !path.exists() {
                return Err(format!(
                    "{} missing required artifact: {}",
                    request.request_id,
                    path.display()
                )
                .into());
            }
        }
    }
    println!("validated {} request(s)", requests.len());
    Ok(())
}

fn select_tick_requests(requests: &[Request], request_id: Option<&str>) -> Result<Vec<String>> {
    if let Some(request_id) = request_id {
        let index = find_request_index(requests, request_id)
            .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
        if is_terminal_status(&requests[index].status) {
            return Ok(Vec::new());
        }
        if is_agent_running_status(&requests[index].status) {
            return Ok(Vec::new());
        }
        if is_slice_request(&requests[index])
            && !slice_dependencies_ready(&requests[index], requests)?
        {
            return Ok(Vec::new());
        }
        return Ok(vec![requests[index].request_id.clone()]);
    }
    let mut selected = Vec::new();
    for request in requests {
        if is_agent_running_status(&request.status) || is_terminal_status(&request.status) {
            continue;
        }
        if is_slice_request(request) && !slice_dependencies_ready(request, requests)? {
            continue;
        }
        selected.push(request.request_id.clone());
    }
    Ok(selected)
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        canonical_status(status),
        STATUS_FINISHED
            | STATUS_WAIT_FINISH
            | STATUS_WAIT_UPDATE_PR
            | STATUS_SLICE_FINISHED
            | "blocked"
    )
}

fn is_agent_running_status(status: &str) -> bool {
    matches!(
        status,
        "agent-running"
            | "decomposition-agent-running"
            | "decomposition-review-running"
            | "planning-agent-running"
            | "plan-review-running"
            | "implementation-agent-running"
            | "code-review-running"
            | "rebase-agent-running"
            | "integration-review-running"
    )
}

fn running_issue_agent_count(requests: &[Request]) -> usize {
    requests
        .iter()
        .filter(|request| is_agent_running_status(&request.status))
        .count()
}

fn sync_all_requests_from_status_json() -> Result<usize> {
    let mut synced = 0;
    loop {
        let mut requests = load_requests()?;
        let mut progressed = false;
        for index in 0..requests.len() {
            if sync_request_from_status_json(&mut requests, index)? {
                synced += 1;
                progressed = true;
                break;
            }
        }
        if !progressed {
            break;
        }
    }
    Ok(synced)
}

fn sync_request_from_status_json(requests: &mut [Request], index: usize) -> Result<bool> {
    let request = requests[index].clone();
    if request.change_path.trim().is_empty() {
        return Ok(false);
    }
    let status_path = Path::new(&request.change_path).join("status.json");
    if !status_path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(&status_path)?;
    let runtime_request_id = json_value(&content, "request_id").unwrap_or_default();
    if runtime_request_id != request.request_id {
        return Ok(false);
    }
    let runtime_status =
        canonical_status(&json_value(&content, "status").unwrap_or_default()).to_string();
    if runtime_status.trim().is_empty() || status_progress_rank(&runtime_status).is_none() {
        return Ok(false);
    }
    let runtime_branch = json_value(&content, "branch").unwrap_or_default();
    let runtime_worktree = json_value(&content, "worktree").unwrap_or_default();
    let runtime_updated_at = json_value(&content, "updated_at").unwrap_or_else(now_string);

    let mut synced = request.clone();
    let mut changed = false;
    if should_sync_runtime_status(&request.status, &runtime_status) {
        synced.status = runtime_status.clone();
        changed = true;
    }
    if synced.branch.trim().is_empty() && !runtime_branch.trim().is_empty() {
        synced.branch = runtime_branch;
        changed = true;
    }
    if synced.worktree_path.trim().is_empty() && !runtime_worktree.trim().is_empty() {
        synced.worktree_path = runtime_worktree;
        changed = true;
    }
    if !changed {
        return Ok(false);
    }

    synced.updated_at = runtime_updated_at;
    requests[index] = synced.clone();
    save_requests(requests)?;
    append_event(
        "request_state_synced",
        &synced.request_id,
        json_value(&content, "stage")
            .as_deref()
            .unwrap_or("runtime-status"),
        &synced.status,
        &format!("source={}", status_path.display()),
    )?;
    Ok(true)
}

fn should_sync_runtime_status(central_status: &str, runtime_status: &str) -> bool {
    let Some(runtime_rank) = status_progress_rank(runtime_status) else {
        return false;
    };
    let central_rank = status_progress_rank(central_status).unwrap_or(0);
    runtime_rank > central_rank
}

fn status_progress_rank(status: &str) -> Option<u8> {
    match canonical_status(status) {
        "discovered" => Some(1),
        "decomposition" => Some(5),
        "decomposition-agent-running" => Some(8),
        "decomposition-submitted" => Some(10),
        "decomposition-review-running" => Some(11),
        "decomposition-review-rejected" => Some(12),
        "decomposition-approved" => Some(15),
        STATUS_SLICES_READY => Some(16),
        STATUS_SLICES_RUNNING => Some(17),
        "planning" => Some(20),
        "planning-agent-running" | "agent-running" => Some(30),
        "plan-submitted" => Some(40),
        "plan-review-running" => Some(42),
        "plan-review-rejected" => Some(45),
        "plan-approved" => Some(50),
        "in-progress" => Some(55),
        "implementation-agent-running" => Some(60),
        "change-doc-submitted" => Some(70),
        "code-review-running" => Some(72),
        "code-review-rejected" => Some(75),
        "change-doc-approved" => Some(80),
        STATUS_SLICE_FINISHED => Some(81),
        "integration-review-submitted" => Some(82),
        "integration-review-running" => Some(83),
        "integration-review-rejected" => Some(84),
        "rebase-agent-running" => Some(86),
        STATUS_WAIT_UPDATE_PR => Some(90),
        STATUS_WAIT_FINISH => Some(95),
        STATUS_FINISHED => Some(100),
        "blocked" => Some(110),
        _ => None,
    }
}

struct RequestLock {
    path: PathBuf,
}

impl RequestLock {
    fn acquire(request_id: &str) -> Result<Option<Self>> {
        let path = request_lock_path(request_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::create_dir(&path) {
            Ok(()) => {
                fs::write(path.join("pid"), format!("{}\n", std::process::id()))?;
                Ok(Some(Self { path }))
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                if request_lock_is_stale(&path)? {
                    remove_request_lock_dir(&path)?;
                    return Self::acquire(request_id);
                }
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }
}

fn acquire_request_lock_wait(request_id: &str, operation: &str) -> Result<RequestLock> {
    const ATTEMPTS: usize = 200;
    for _ in 0..ATTEMPTS {
        if let Some(lock) = RequestLock::acquire(request_id)? {
            return Ok(lock);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!("{operation} could not acquire request lock for {request_id}").into())
}

impl Drop for RequestLock {
    fn drop(&mut self) {
        let _ = remove_request_lock_dir(&self.path);
    }
}

fn request_lock_path(request_id: &str) -> PathBuf {
    Path::new(".sandrone/state/locks").join(format!("{request_id}.lock"))
}

fn request_lock_is_stale(path: &Path) -> Result<bool> {
    let pid_path = path.join("pid");
    if !pid_path.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(pid_path)?;
    let Some(pid) = content.trim().parse::<u32>().ok() else {
        return Ok(false);
    };
    Ok(!process_is_running(pid))
}

fn remove_request_lock_dir(path: &Path) -> Result<()> {
    let pid_path = path.join("pid");
    if pid_path.exists() {
        fs::remove_file(pid_path)?;
    }
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn refresh_tick_statuses() -> Result<usize> {
    let mut changed = 0;
    loop {
        let requests = load_requests()?;
        let mut progressed = false;
        for request in requests {
            if request.change_path.is_empty() || is_terminal_status(&request.status) {
                continue;
            }
            let Some(_lock) = RequestLock::acquire(&request.request_id)? else {
                continue;
            };
            if refresh_request_status_by_id(&request.request_id)? {
                progressed = true;
                break;
            }
        }
        if !progressed {
            break;
        }
        changed += 1;
    }
    Ok(changed)
}

fn refresh_request_status_by_id(request_id: &str) -> Result<bool> {
    let mut requests = load_requests()?;
    let Some(index) = find_request_index(&requests, request_id) else {
        return Err(format!("unknown request_id: {request_id}").into());
    };
    if sync_request_from_status_json(&mut requests, index)? {
        return Ok(true);
    }
    let request = requests[index].clone();
    if request.change_path.is_empty() || is_terminal_status(&request.status) {
        return Ok(false);
    }

    if ensure_gate_approved(&request, "change-doc").is_ok() {
        if is_slice_request(&request) {
            mark_slice_finished_by_id(
                &request.request_id,
                "slice change-doc gate is valid; marking slice finished",
            )?;
            return Ok(true);
        }
        mark_wait_update_pr_by_id(
            &request.request_id,
            "change-doc gate is valid; waiting for PR creation or update",
        )?;
        return Ok(true);
    }

    if is_parent_request(&request) && ensure_gate_approved(&request, "decomposition").is_ok() {
        let mut requests = load_requests()?;
        let index = find_request_index(&requests, &request.request_id)
            .ok_or_else(|| format!("unknown request_id: {}", request.request_id))?;
        let changed = materialize_slices_for_parent(
            &mut requests,
            index,
            &assess_repository_before_planning()?,
        )?;
        save_requests(&requests)?;
        if changed {
            return Ok(true);
        }
        return refresh_parent_slice_status(&request.request_id);
    }

    match request.status.as_str() {
        "decomposition-submitted" => run_decomposition_review_from_tick(&request.request_id),
        "decomposition-review-running" => {
            refresh_review_stage(&request.request_id, "decomposition-review")
        }
        "decomposition-agent-running" => refresh_agent_phase(&request, AgentPhase::Decomposition),
        "plan-submitted" => run_plan_review_from_tick(&request.request_id),
        "plan-review-running" => refresh_review_stage(&request.request_id, "plan-review"),
        "change-doc-submitted" => run_code_review_from_tick(&request.request_id),
        "code-review-running" => refresh_review_stage(&request.request_id, "code-review"),
        "planning-agent-running" => refresh_agent_phase(&request, AgentPhase::Planning),
        "implementation-agent-running" => refresh_agent_phase(&request, AgentPhase::Implementation),
        "rebase-agent-running" => refresh_agent_phase(&request, AgentPhase::Rebase),
        "integration-review-submitted" => run_integration_review_from_tick(&request.request_id),
        "integration-review-running" => {
            refresh_review_stage(&request.request_id, "integration-review")
        }
        "agent-running" => refresh_legacy_agent_status(&request),
        _ => Ok(false),
    }
}

fn dispatch_next_agent_for_request(
    request_id: &str,
    max_attempts: Option<u32>,
    preflight: &mut Option<PlanPreflight>,
) -> Result<Option<(Request, AgentPhase, u32)>> {
    let mut requests = load_requests()?;
    let Some(mut index) = find_request_index(&requests, request_id) else {
        return Err(format!("unknown request_id: {request_id}").into());
    };
    if sync_request_from_status_json(&mut requests, index)? {
        requests = load_requests()?;
        index = find_request_index(&requests, request_id)
            .ok_or_else(|| format!("selected request disappeared after sync: {request_id}"))?;
    }
    if is_agent_running_status(&requests[index].status)
        || is_terminal_status(&requests[index].status)
    {
        return Ok(None);
    }
    if is_slice_request(&requests[index]) && !slice_dependencies_ready(&requests[index], &requests)?
    {
        return Ok(None);
    }
    if requests[index].change_path.is_empty() {
        let change_name = auto_change_name(&requests[index]);
        if preflight.is_none() {
            *preflight = Some(assess_repository_before_planning()?);
        }
        let request = if is_parent_request(&requests[index]) {
            create_decomposition_packet_for_index(
                &mut requests,
                index,
                Some(&change_name),
                preflight
                    .as_ref()
                    .ok_or("planning preflight was not initialized")?,
            )?
        } else {
            create_plan_packet_for_index(
                &mut requests,
                index,
                &change_name,
                preflight
                    .as_ref()
                    .ok_or("planning preflight was not initialized")?,
            )?
        };
        println!("Created change packet for {}", request.request_id);
        println!("  change path: {}", request.change_path);
    }

    requests = load_requests()?;
    index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("selected request disappeared: {request_id}"))?;
    let mut request = requests[index].clone();
    if is_slice_request(&request) && !slice_dependencies_ready(&request, &requests)? {
        return Ok(None);
    }
    let Some(phase) = next_agent_phase(&request)? else {
        return Ok(None);
    };
    if !Path::new(phase.tool_path()).exists() {
        return Err(format!("{} does not exist", phase.tool_path()).into());
    }
    let resolved_max_attempts = resolve_max_attempts(phase, max_attempts);
    if review_attempts_exhausted(&request, phase, resolved_max_attempts)? {
        let stage = phase.as_str();
        let reason = format!(
            "{stage} review failed after {resolved_max_attempts} attempt(s); manual recovery is required"
        );
        mark_blocked(&mut requests, index, &mut request, stage, &reason)?;
        return Ok(None);
    }
    if phase == AgentPhase::Implementation && request.worktree_path.trim().is_empty() {
        start_worktree_inner(&["--request_id".to_string(), request.request_id.clone()])?;
        requests = load_requests()?;
        index = find_request_index(&requests, request_id)
            .ok_or_else(|| format!("selected request disappeared after start: {request_id}"))?;
        request = requests[index].clone();
    } else if phase == AgentPhase::Rebase && request.worktree_path.trim().is_empty() {
        return Err(format!("{} has no worktree for rebase agent", request.request_id).into());
    }

    let resume_session_id = reusable_agent_session_id(&request, phase);
    reset_phase_document_for_agent_dispatch(&request, phase)?;
    let phase_name = phase.as_str();
    request.status = phase.running_status().to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(&request, phase_name, phase.running_status(), "")?;
    upsert_session_for_request(&request, phase_name, phase.running_status())?;

    match spawn_issue_agent(&request, resolved_max_attempts, phase, resume_session_id) {
        Ok(pid) => {
            append_event(
                "agent_dispatched",
                &request.request_id,
                phase.as_str(),
                phase.running_status(),
                &format!("pid={pid}; change_path={}", request.change_path),
            )?;
            Ok(Some((request, phase, pid)))
        }
        Err(error) => {
            let mut requests = load_requests()?;
            if let Some(index) = find_request_index(&requests, &request.request_id) {
                let mut blocked = requests[index].clone();
                mark_blocked(
                    &mut requests,
                    index,
                    &mut blocked,
                    phase.as_str(),
                    &error.to_string(),
                )?;
            }
            Err(error)
        }
    }
}

fn dispatch_next_slice_for_parent(
    parent_id: &str,
    max_attempts: Option<u32>,
    preflight: &mut Option<PlanPreflight>,
) -> Result<Option<(Request, AgentPhase, u32)>> {
    let requests = load_requests()?;
    let Some(parent) = requests
        .iter()
        .find(|request| request.request_id == parent_id)
    else {
        return Ok(None);
    };
    if is_slice_request(parent) {
        return Ok(None);
    }
    for request in &requests {
        if !is_slice_request(request) || slice_parent_id(request).as_deref() != Some(parent_id) {
            continue;
        }
        if is_agent_running_status(&request.status)
            || is_terminal_status(&request.status)
            || !slice_dependencies_ready(request, &requests)?
        {
            continue;
        }
        let Some(_lock) = RequestLock::acquire(&request.request_id)? else {
            continue;
        };
        return dispatch_next_agent_for_request(&request.request_id, max_attempts, preflight);
    }
    Ok(None)
}

fn next_agent_phase(request: &Request) -> Result<Option<AgentPhase>> {
    if request.change_path.is_empty()
        || is_terminal_status(&request.status)
        || is_agent_running_status(&request.status)
        || matches!(
            request.status.as_str(),
            "decomposition-submitted"
                | "decomposition-review-running"
                | "plan-submitted"
                | "plan-review-running"
                | "change-doc-submitted"
                | "code-review-running"
                | "integration-review-submitted"
                | "integration-review-running"
        )
    {
        return Ok(None);
    }
    if request.status == "plan-review-rejected" {
        return Ok(Some(AgentPhase::Planning));
    }
    if request.status == "integration-review-rejected" {
        return Ok(Some(AgentPhase::Rebase));
    }
    if ensure_gate_approved(request, "plan").is_ok() {
        if ensure_gate_approved(request, "change-doc").is_ok() {
            return Ok(None);
        }
        return Ok(Some(AgentPhase::Implementation));
    }
    if is_parent_request(request) {
        if request.status == "decomposition-review-rejected" {
            return Ok(Some(AgentPhase::Decomposition));
        }
        if ensure_gate_approved(request, "decomposition").is_err() {
            return Ok(Some(AgentPhase::Decomposition));
        }
        return Ok(None);
    }
    if ensure_gate_approved(request, "plan").is_err() {
        return Ok(Some(AgentPhase::Planning));
    }
    if ensure_gate_approved(request, "change-doc").is_ok() {
        return Ok(None);
    }
    Ok(Some(AgentPhase::Implementation))
}

fn review_attempts_exhausted(
    request: &Request,
    phase: AgentPhase,
    max_attempts: u32,
) -> Result<bool> {
    let stage = match phase {
        AgentPhase::Decomposition => "decomposition-review",
        AgentPhase::Planning => "plan-review",
        AgentPhase::Implementation => "code-review",
        AgentPhase::Rebase => "integration-review",
    };
    let attempts = review_attempt_count(request, stage)?;
    Ok(attempts >= max_attempts
        && matches!(
            request.status.as_str(),
            "decomposition-review-rejected"
                | "plan-review-rejected"
                | "code-review-rejected"
                | "integration-review-rejected"
        ))
}

fn review_attempt_count(request: &Request, stage: &str) -> Result<u32> {
    let details_dir = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("details");
    if !details_dir.exists() {
        return Ok(0);
    }
    let mut attempts = Vec::new();
    for entry in fs::read_dir(details_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Some((prefix, _)) = name.split_once('-') else {
            continue;
        };
        if let Ok(attempt) = prefix.parse::<u32>()
            && !attempts.contains(&attempt)
        {
            attempts.push(attempt);
        }
    }
    Ok(attempts.len() as u32)
}

fn refresh_agent_phase(request: &Request, phase: AgentPhase) -> Result<bool> {
    let Some(exit_code) = read_agent_exit_code(&request.request_id)? else {
        return refresh_missing_agent_exit(request, phase.as_str());
    };
    record_agent_session_id(&request.request_id, phase)?;
    if exit_code != "0" {
        if let Some(artifact) = agent_document_status_is_submitted(request, phase)? {
            append_event(
                "agent_document_status_used",
                &request.request_id,
                phase.as_str(),
                &request.status,
                &format!("exit_code={exit_code}; artifact={}", artifact.display()),
            )?;
        } else {
            let reason = format!(
                "{} agent exited with code {exit_code}. See {} and {}",
                phase.as_str(),
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
            block_request_by_id(&request.request_id, phase.as_str(), &reason)?;
            return Ok(true);
        }
    }

    match phase {
        AgentPhase::Decomposition => {
            submit_gate_from_tick(&request.request_id, "decomposition")?;
            run_decomposition_review_from_tick(&request.request_id)
        }
        AgentPhase::Planning => {
            submit_gate_from_tick(&request.request_id, "plan")?;
            run_plan_review_from_tick(&request.request_id)
        }
        AgentPhase::Implementation => {
            submit_gate_from_tick(&request.request_id, "change-doc")?;
            run_code_review_from_tick(&request.request_id)
        }
        AgentPhase::Rebase => {
            ensure_rebase_ready_for_integration_review(request)?;
            let mut requests = load_requests()?;
            let index = find_request_index(&requests, &request.request_id)
                .ok_or_else(|| format!("unknown request_id: {}", request.request_id))?;
            let mut refreshed = requests[index].clone();
            append_integration_record(
                &refreshed,
                &IntegrationRecord {
                    mode: "rebase-agent-completed",
                    base_branch: "",
                    base_ref: "",
                    before_head: "",
                    after_head: "",
                    pr_status: "agent completed",
                    detail: "RebaseAgent exited successfully; running IntegrationReviewer.",
                },
            )?;
            mark_integration_review_submitted(
                &mut requests,
                index,
                &mut refreshed,
                "rebase agent completed",
            )?;
            run_integration_review_from_tick(&request.request_id)
        }
    }
}

fn refresh_legacy_agent_status(request: &Request) -> Result<bool> {
    let Some(exit_code) = read_agent_exit_code(&request.request_id)? else {
        return refresh_missing_agent_exit(request, "agent");
    };
    let reason = if exit_code == "0" {
        "legacy issue-agent exited successfully but change-doc gate is missing or stale".to_string()
    } else {
        format!(
            "legacy issue-agent exited with code {exit_code}. See {} and {}",
            agent_stdout_path(&request.request_id).display(),
            agent_stderr_path(&request.request_id).display()
        )
    };
    block_request_by_id(&request.request_id, "agent", &reason)?;
    Ok(true)
}

fn refresh_missing_agent_exit(request: &Request, stage: &str) -> Result<bool> {
    match read_agent_pid(&request.request_id)? {
        Some(pid) if process_is_running(pid) => Ok(false),
        Some(pid) => {
            let reason = format!(
                "{stage} agent pid {pid} is no longer running and no exit code was written. See {} and {}",
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
            block_request_by_id(&request.request_id, stage, &reason)?;
            Ok(true)
        }
        None => {
            let reason = format!(
                "{stage} agent is marked running but no pid or exit code was recorded. See {} and {}",
                agent_stdout_path(&request.request_id).display(),
                agent_stderr_path(&request.request_id).display()
            );
            block_request_by_id(&request.request_id, stage, &reason)?;
            Ok(true)
        }
    }
}

fn submit_gate_from_tick(request_id: &str, gate: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    request.status = format!("{}-submitted", gate_status_prefix(gate));
    request.updated_at = now_string();
    mark_phase_document_submitted(&request, gate_agent_phase(gate))?;
    write_approval_record(
        &request,
        gate,
        "submitted",
        "",
        "outer-tick",
        "submitted by outer tick after agent phase completed",
    )?;
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(
        &request,
        match gate {
            "decomposition" => "decomposition",
            "plan" => "planning",
            _ => "implementation",
        },
        &request.status,
        "submitted by outer tick",
    )?;
    append_event(
        "gate_submitted",
        &request.request_id,
        match gate {
            "decomposition" => "decomposition",
            "plan" => "planning",
            _ => "implementation",
        },
        &request.status,
        &format!("gate={gate}; source=outer-tick"),
    )?;
    update_gate_session(&request, gate, "waiting-review")
}

fn gate_agent_phase(gate: &str) -> AgentPhase {
    match gate {
        "decomposition" => AgentPhase::Decomposition,
        "plan" => AgentPhase::Planning,
        "change-doc" => AgentPhase::Implementation,
        _ => AgentPhase::Implementation,
    }
}

fn run_decomposition_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match decomposition_review(&args) {
        Ok(()) => Ok(true),
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn run_plan_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match plan_review(&args) {
        Ok(()) => Ok(true),
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn run_code_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match code_review(&args) {
        Ok(()) => Ok(true),
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn run_integration_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match integration_review(&args) {
        Ok(()) => Ok(true),
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn is_review_terminal_error(message: &str) -> bool {
    message.contains("rejected decomposition review")
        || message.contains("rejected plan review")
        || message.contains("rejected code review")
        || message.contains("rejected integration review")
        || message.contains("format check failed before code-review")
        || message.contains("review gate unavailable")
        || message.contains("gate unavailable")
}

fn block_request_by_id(request_id: &str, stage: &str, reason: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    mark_blocked(&mut requests, index, &mut request, stage, reason)
}

fn mark_wait_update_pr_by_id(request_id: &str, reason: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if canonical_status(&request.status) == STATUS_WAIT_UPDATE_PR {
        return Ok(());
    }
    ensure_gate_approved(&request, "change-doc")?;
    request.status = STATUS_WAIT_UPDATE_PR.to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(&request, "delivery", STATUS_WAIT_UPDATE_PR, reason)?;
    append_event(
        "waiting_pr_update",
        &request.request_id,
        "delivery",
        STATUS_WAIT_UPDATE_PR,
        reason,
    )?;
    upsert_session_for_request(&request, "implementation", STATUS_WAIT_UPDATE_PR)
}

fn parse_max_attempts(value: Option<String>) -> Result<Option<u32>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let parsed = value
        .parse::<u32>()
        .map_err(|_| "--max-attempts must be a positive integer")?;
    if parsed == 0 {
        return Err("--max-attempts must be greater than 0".into());
    }
    Ok(Some(parsed))
}

fn parse_parallel_limit(value: Option<String>, default_limit: usize) -> Result<usize> {
    let Some(value) = value else {
        return Ok(default_limit.max(1));
    };
    let parsed = value
        .parse::<usize>()
        .map_err(|_| "--parallel-limit must be a positive integer")?;
    if parsed == 0 {
        return Err("--parallel-limit must be greater than 0".into());
    }
    Ok(parsed)
}

fn resolve_tick_auto_merge(args: &[String], config_default: bool) -> Result<bool> {
    let enabled_by_flag = flag_present(args, "--auto-merge");
    let disabled_by_flag = flag_present(args, "--no-auto-merge");
    if enabled_by_flag && disabled_by_flag {
        return Err("--auto-merge and --no-auto-merge cannot be used together".into());
    }
    if enabled_by_flag {
        return Ok(true);
    }
    if disabled_by_flag {
        return Ok(false);
    }
    match env::var("SANDRONE_AUTO_MERGE") {
        Ok(value) if !value.trim().is_empty() => parse_bool_value(&value, "SANDRONE_AUTO_MERGE"),
        _ => Ok(config_default),
    }
}

fn parse_bool_value(value: &str, name: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("{name} must be one of true/false, 1/0, yes/no, or on/off").into()),
    }
}

fn phase_default_max_attempts(phase: AgentPhase) -> u32 {
    match phase {
        AgentPhase::Decomposition => DEFAULT_DECOMPOSITION_MAX_ATTEMPTS,
        AgentPhase::Planning => DEFAULT_PLAN_MAX_ATTEMPTS,
        AgentPhase::Implementation => DEFAULT_CODE_MAX_ATTEMPTS,
        AgentPhase::Rebase => DEFAULT_INTEGRATION_MAX_ATTEMPTS,
    }
}

fn resolve_max_attempts(phase: AgentPhase, max_attempts: Option<u32>) -> u32 {
    max_attempts.unwrap_or_else(|| phase_default_max_attempts(phase))
}

fn auto_change_name(request: &Request) -> String {
    let mut slug = slugify(&request.title);
    if slug.is_empty() {
        slug = request.request_id.to_lowercase();
    }
    format!("{}-{}-{}", today(), request.request_id.to_lowercase(), slug)
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in value.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn find_request_index(requests: &[Request], request_id: &str) -> Option<usize> {
    requests
        .iter()
        .position(|request| request.request_id == request_id)
}

fn manual_request(request_id: &str, change_name: &str) -> Request {
    let now = now_string();
    Request {
        request_id: request_id.to_string(),
        external_id: format!("manual:{request_id}"),
        source: "manual".to_string(),
        title: change_name.split('-').skip(3).collect::<Vec<_>>().join(" "),
        body: "Codex should fill this request from the user conversation.".to_string(),
        url: String::new(),
        status: "discovered".to_string(),
        change_name: String::new(),
        change_path: String::new(),
        branch: String::new(),
        worktree_path: String::new(),
        created_at: now.clone(),
        updated_at: now,
    }
}

fn ensure_initialized() -> Result<()> {
    registry::migrate_legacy_current_workspace_state_if_needed()?;
    if !Path::new(CONFIG_PATH).exists() {
        return Err("not initialized. Run: sandrone new --url <git-url> or sandrone new --name <project-name>".into());
    }
    Ok(())
}

fn repo_has_commits(cwd: &str) -> bool {
    git_output(cwd, &["rev-parse", "--verify", "HEAD"]).is_ok()
}

fn pull_target_repo_before_worktree_creation() -> Result<GitPullOutcome> {
    if !repo_has_commits(DEV_REPO) {
        return Ok(GitPullOutcome::Skipped(
            "target repo has no commits".to_string(),
        ));
    }
    if !remote_exists(DEV_REPO) {
        return Ok(GitPullOutcome::Skipped(
            "target repo has no git remote".to_string(),
        ));
    }

    let before = git_output(DEV_REPO, &["rev-parse", "HEAD"])?;
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(DEV_REPO)
        .envs(proxy_env())
        .output()?;
    if !output.status.success() {
        let detail = review_diagnostic_excerpt(&format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
        return Err(format!("git pull failed before worktree creation: {detail}").into());
    }
    let after = git_output(DEV_REPO, &["rev-parse", "HEAD"])?;
    if before == after {
        Ok(GitPullOutcome::AlreadyUpToDate)
    } else {
        Ok(GitPullOutcome::Updated)
    }
}

fn print_worktree_pull_outcome(outcome: &GitPullOutcome) {
    match outcome {
        GitPullOutcome::Skipped(reason) => {
            println!("  git pull: skipped before worktree creation ({reason})");
        }
        GitPullOutcome::AlreadyUpToDate => {
            println!("  git pull: dev/repo already up to date before worktree creation");
        }
        GitPullOutcome::Updated => {
            println!("  git pull: updated dev/repo before worktree creation");
        }
    }
}

fn remote_exists(cwd: &str) -> bool {
    git_output(cwd, &["remote"])
        .map(|remotes| !remotes.trim().is_empty())
        .unwrap_or(false)
}

fn upstream_is_ahead(cwd: &str) -> Result<bool> {
    if git_output(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .is_err()
    {
        return Ok(false);
    }
    let count = git_output(cwd, &["rev-list", "--count", "HEAD..@{u}"])?;
    Ok(count.trim().parse::<u32>().unwrap_or(0) > 0)
}

fn fetch_if_remote_exists() -> Result<()> {
    let remotes = git_output(DEV_REPO, &["remote"]).unwrap_or_default();
    if remotes.trim().is_empty() {
        return Ok(());
    }
    run_command(
        Command::new("git")
            .args(["fetch", "--all", "--prune"])
            .current_dir(DEV_REPO)
            .envs(proxy_env()),
    )
}

fn spawn_issue_agent(
    request: &Request,
    max_attempts: u32,
    phase: AgentPhase,
    resume_session_id: Option<String>,
) -> Result<u32> {
    let tool_path = phase.tool_path();
    if !Path::new(tool_path).exists() {
        return Err(format!("{tool_path} does not exist").into());
    }
    let session_path = agent_session_path(&request.request_id, phase);
    create_agent_run_state_dir(&request.request_id, phase.as_str())?;
    if resume_session_id.is_none() {
        remove_runtime_file(&session_path, None)?;
    }
    let stdout_path = agent_stdout_path(&request.request_id);
    let legacy_stdout_path = legacy_agent_stdout_path(&request.request_id);
    let stderr_path = agent_stderr_path(&request.request_id);
    let legacy_stderr_path = legacy_agent_stderr_path(&request.request_id);
    let stdout = create_truncated_runtime_file(&stdout_path, Some(&legacy_stdout_path))?;
    let stderr = create_truncated_runtime_file(&stderr_path, Some(&legacy_stderr_path))?;
    let exit_path = agent_exit_path(&request.request_id);
    let legacy_exit_path = legacy_agent_exit_path(&request.request_id);
    let hook_log_path = agent_hook_log_path(&request.request_id);
    let legacy_hook_log_path = legacy_agent_hook_log_path(&request.request_id);
    let events_log_path = agent_events_log_path(&request.request_id);
    drop(create_truncated_runtime_file(
        &hook_log_path,
        Some(&legacy_hook_log_path),
    )?);
    drop(create_truncated_runtime_file(&events_log_path, None)?);
    remove_runtime_file(&exit_path, Some(&legacy_exit_path))?;
    let wrapper_script = r#"tool=$1
exit_path=$2
legacy_exit_path=$3
hook_log=$4
runtime_log=$5
session_path=$6
stdout_log=$7
stderr_log=$8

resolve_sandrone_bin() {
  if [ -n "${SANDRONE_BIN:-}" ]; then
    case "$(basename "$SANDRONE_BIN")" in
      codex-auto-dev) ;;
      *)
        if [ -x "$SANDRONE_BIN" ]; then
          printf '%s\n' "$SANDRONE_BIN"
          return 0
        fi
        ;;
    esac
  fi
  if command -v sandrone >/dev/null 2>&1; then
    command -v sandrone
    return 0
  fi
  if command -v sdr >/dev/null 2>&1; then
    command -v sdr
    return 0
  fi
  return 1
}

write_runtime_event() {
  event=$1
  detail=${2:-}
  printf '%s\t%s\t%s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "$event" "$detail" >> "$runtime_log" 2>/dev/null || true
}

run_hook() {
  code=$1
  if [ -n "${SANDRONE_REQUEST_ID:-}" ]; then
    if sandrone_bin=$(resolve_sandrone_bin); then
      "$sandrone_bin" advance --request_id "$SANDRONE_REQUEST_ID" --max-attempts "$SANDRONE_MAX_ATTEMPTS" >> "$hook_log" 2>&1 || true
    else
      printf 'Sandrone-agent-wrapper: sandrone CLI not found; skip advance hook\n' >> "$hook_log"
    fi
  fi
}

record_session_id() {
  [ -n "$session_path" ] || return 0
  session_id=""
  for log_path in "$stdout_log" "$stderr_log"; do
    [ -f "$log_path" ] || continue
    candidate=$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([0-9A-Fa-f-]{16,})".*/\1/p; s/.*"sessionId"[[:space:]]*:[[:space:]]*"([0-9A-Fa-f-]{16,})".*/\1/p; s/.*session id:[[:space:]]*"?([0-9A-Fa-f-]{16,})"?.*/\1/p' "$log_path" 2>/dev/null | tail -n 1)
    case "$candidate" in
      *-*) session_id=$candidate ;;
    esac
    [ -n "$session_id" ] && break
  done
  [ -n "$session_id" ] || return 0
  mkdir -p "$(dirname "$session_path")" 2>/dev/null || return 0
  printf '%s\n' "$session_id" > "$session_path" 2>/dev/null || return 0
  write_runtime_event agent-session-recorded "session_id=$session_id"
}

write_exit() {
  code=$1
  printf '%s\n' "$code" > "$exit_path"
  [ -n "$legacy_exit_path" ] && printf '%s\n' "$code" > "$legacy_exit_path"
  record_session_id
  write_runtime_event wrapper-exited "exit=$code"
  run_hook "$code"
  exit "$code"
}

trap 'write_exit 129' HUP
trap 'write_exit 130' INT
trap 'write_exit 143' TERM
write_runtime_event wrapper-started "tool=$tool"
write_runtime_event tool-started "tool=$tool"
sh "$tool"
code=$?
write_runtime_event tool-exited "exit=$code"
write_exit "$code"
"#;
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(wrapper_script)
        .arg("Sandrone-agent-wrapper")
        .arg(tool_path)
        .arg(&exit_path)
        .arg(&legacy_exit_path)
        .arg(&hook_log_path)
        .arg(&events_log_path)
        .arg(&session_path)
        .arg(&stdout_path)
        .arg(&stderr_path)
        .current_dir(".")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    command.process_group(0);
    apply_issue_agent_env(&mut command, request, max_attempts, phase)?;
    if let Some(resume_session_id) = resume_session_id {
        command.env("SANDRONE_AGENT_RESUME_SESSION_ID", resume_session_id);
    }
    write_job_runtime(
        agent_runtime_path(&request.request_id),
        &JobRuntime {
            kind: "agent",
            request_id: &request.request_id,
            stage: phase.as_str(),
            attempt: "current",
            worker: "issue-agent",
            tool: tool_path,
            pid: None,
            status: "spawning",
        },
    )?;
    append_job_event(
        &events_log_path,
        "dispatched",
        &format!("stage={}; tool={tool_path}", phase.as_str()),
    )?;
    let child = command.spawn()?;
    let pid_text = format!("{}\n", child.id());
    write_runtime_text(
        agent_pid_path(&request.request_id),
        &pid_text,
        Some(&legacy_agent_pid_path(&request.request_id)),
    )?;
    write_job_runtime(
        agent_runtime_path(&request.request_id),
        &JobRuntime {
            kind: "agent",
            request_id: &request.request_id,
            stage: phase.as_str(),
            attempt: "current",
            worker: "issue-agent",
            tool: tool_path,
            pid: Some(child.id()),
            status: "running",
        },
    )?;
    Ok(child.id())
}

fn reusable_agent_session_id(request: &Request, phase: AgentPhase) -> Option<String> {
    if request.status != phase.review_rejected_status() {
        return None;
    }
    if let Ok(content) = fs::read_to_string(agent_session_path(&request.request_id, phase)) {
        let session_id = content.trim();
        if looks_like_codex_session_id(session_id) {
            return Some(session_id.to_string());
        }
    }
    agent_log_session_id(&request.request_id)
}

fn record_agent_session_id(request_id: &str, phase: AgentPhase) -> Result<()> {
    let Some(session_id) = agent_log_session_id(request_id) else {
        return Ok(());
    };
    let path = agent_session_path(request_id, phase);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("{session_id}\n"))?;
    append_event(
        "agent_session_recorded",
        request_id,
        phase.as_str(),
        "recorded",
        &format!("session_id={session_id}"),
    )?;
    Ok(())
}

fn agent_log_session_id(request_id: &str) -> Option<String> {
    for path in [
        agent_stdout_path(request_id),
        agent_stderr_path(request_id),
        legacy_agent_stdout_path(request_id),
        legacy_agent_stderr_path(request_id),
    ] {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        if let Some(session_id) = extract_codex_session_id(&content) {
            return Some(session_id);
        }
    }
    None
}

fn agent_session_path(request_id: &str, phase: AgentPhase) -> PathBuf {
    legacy_agent_state_dir().join(format!("{}.{}.session", request_id, phase.as_str()))
}

fn extract_codex_session_id(content: &str) -> Option<String> {
    for line in content.lines().rev() {
        if let Some(session_id) = json_value(line, "session_id")
            .or_else(|| json_value(line, "sessionId"))
            .filter(|value| looks_like_codex_session_id(value))
        {
            return Some(session_id);
        }
        if let Some((_, raw)) = line.split_once("session id:") {
            let session_id = raw
                .trim()
                .split(|character: char| character.is_whitespace() || character == ',')
                .next()
                .unwrap_or_default()
                .trim_matches(|character| character == '"' || character == '\'')
                .to_string();
            if looks_like_codex_session_id(&session_id) {
                return Some(session_id);
            }
        }
    }
    None
}

fn looks_like_codex_session_id(value: &str) -> bool {
    value.len() >= 16
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() || character == '-')
        && value.contains('-')
}

fn apply_issue_agent_env(
    command: &mut Command,
    request: &Request,
    max_attempts: u32,
    phase: AgentPhase,
) -> Result<()> {
    let current_exe = env::current_exe()?;
    let request_artifact = request_handoff_artifact_path_string(request, "request.md");
    let plan_artifact = request_handoff_artifact_path_string(request, "plan.md");
    let decomposition_artifact = request_handoff_artifact_path_string(request, "decomposition.md");
    let dag_artifact = request_handoff_artifact_path_string(request, "dag.json");
    let change_doc_artifact = request_handoff_artifact_path_string(request, "change-doc.md");
    let agent_journal_artifact = request_handoff_artifact_path_string(request, "agent-journal.md");
    let agent_kind = agent_kind_for_phase(phase.as_str());
    command
        .env("SANDRONE_BIN", current_exe.to_string_lossy().to_string())
        .env("SANDRONE_WORKSPACE", absolute_path_string("."))
        .env("SANDRONE_ENV_FILE", absolute_path_string(".env"))
        .env("SANDRONE_TARGET_REPO", absolute_path_string(DEV_REPO))
        .env("SANDRONE_REQUEST_ID", &request.request_id)
        .env("SANDRONE_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("SANDRONE_REQUEST_SOURCE", &request.source)
        .env("SANDRONE_REQUEST_TITLE", &request.title)
        .env("SANDRONE_REQUEST_BODY", &request.body)
        .env("SANDRONE_REQUEST_URL", &request.url)
        .env("SANDRONE_BRANCH", &request.branch)
        .env(
            "SANDRONE_WORKTREE",
            absolute_path_string(request.worktree_path.as_str()),
        )
        .env("SANDRONE_MAX_ATTEMPTS", max_attempts.to_string())
        .env("SANDRONE_AGENT_PHASE", phase.as_str())
        .env("SANDRONE_AGENT_KIND", agent_kind)
        .env(
            "SANDRONE_AGENT_CONFIG_DIR",
            absolute_path_string("agents/config"),
        )
        .env(
            "SANDRONE_AGENT_CONFIG_PATH",
            absolute_path_string(format!("agents/config/{agent_kind}.json")),
        )
        .env(
            "SANDRONE_AGENT_STATUS_DOC",
            absolute_path_string(phase_document_path(request, phase)),
        )
        .env(
            "SANDRONE_CHANGE_PATH",
            absolute_path_string(request.change_path.as_str()),
        )
        .env(
            "SANDRONE_REQUEST",
            absolute_path_string_or_empty(request_artifact),
        )
        .env(
            "SANDRONE_PLAN",
            absolute_path_string_or_empty(plan_artifact),
        )
        .env(
            "SANDRONE_DECOMPOSITION",
            absolute_path_string_or_empty(decomposition_artifact),
        )
        .env("SANDRONE_DAG", absolute_path_string_or_empty(dag_artifact))
        .env(
            "SANDRONE_CHANGE_DOC",
            absolute_path_string_or_empty(change_doc_artifact),
        )
        .env(
            "SANDRONE_AGENT_JOURNAL",
            absolute_path_string_or_empty(agent_journal_artifact),
        )
        .env(
            "SANDRONE_STATUS",
            absolute_path_string(Path::new(&request.change_path).join("status.json")),
        )
        .env(
            "SANDRONE_CODEGRAPH_CONTEXT",
            absolute_path_string("obsidian/codegraph/context.md"),
        )
        .env(
            "SANDRONE_OBSIDIAN_NOTE",
            absolute_path_string(obsidian_request_note_path(request)),
        )
        .env(
            "SANDRONE_OBSIDIAN_PROJECT",
            absolute_path_string(OBSIDIAN_PROJECT_NOTE),
        )
        .env(
            "SANDRONE_ISSUE_AGENT_SHARED_PROMPT",
            absolute_path_string(ISSUE_AGENT_PROMPT),
        )
        .env(
            "SANDRONE_ISSUE_AGENT_PROMPT",
            absolute_path_string(phase.prompt_path()),
        )
        .env(
            "SANDRONE_REBASE_AGENT_PROMPT",
            absolute_path_string(REBASE_AGENT_PROMPT),
        )
        .env(
            "SANDRONE_CHECK_FORMAT_TOOL",
            absolute_path_string(CHECK_FORMAT_TOOL),
        )
        .env(
            "SANDRONE_AGENT_PROMPT",
            absolute_path_string(phase.prompt_path()),
        )
        .envs(proxy_env());
    Ok(())
}

fn read_agent_exit_code(request_id: &str) -> Result<Option<String>> {
    let exit_code = read_runtime_text(
        agent_exit_path(request_id),
        Some(&legacy_agent_exit_path(request_id)),
    )?
    .trim()
    .to_string();
    if exit_code.is_empty() {
        return Ok(None);
    }
    Ok(Some(exit_code))
}

fn read_agent_pid(request_id: &str) -> Result<Option<u32>> {
    let pid = read_runtime_text(
        agent_pid_path(request_id),
        Some(&legacy_agent_pid_path(request_id)),
    )?
    .trim()
    .parse::<u32>()
    .ok();
    Ok(pid)
}

fn process_is_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn agent_job_state_dir(request_id: &str) -> PathBuf {
    runtime_agent_job_state_dir(request_id)
}

fn agent_pid_path(request_id: &str) -> PathBuf {
    job_pid_path(&agent_job_state_dir(request_id))
}

fn agent_stdout_path(request_id: &str) -> PathBuf {
    job_stdout_path(&agent_job_state_dir(request_id))
}

fn agent_stderr_path(request_id: &str) -> PathBuf {
    job_stderr_path(&agent_job_state_dir(request_id))
}

fn agent_hook_log_path(request_id: &str) -> PathBuf {
    job_hook_log_path(&agent_job_state_dir(request_id))
}

fn agent_exit_path(request_id: &str) -> PathBuf {
    job_exit_path(&agent_job_state_dir(request_id))
}

fn agent_runtime_path(request_id: &str) -> PathBuf {
    job_runtime_path(&agent_job_state_dir(request_id))
}

fn agent_events_log_path(request_id: &str) -> PathBuf {
    job_events_log_path(&agent_job_state_dir(request_id))
}

fn legacy_agent_pid_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.pid"))
}

fn legacy_agent_stdout_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.stdout.log"))
}

fn legacy_agent_stderr_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.stderr.log"))
}

fn legacy_agent_hook_log_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.hook.log"))
}

fn legacy_agent_exit_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.exit"))
}

fn run_command(command: &mut Command) -> Result<()> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .into());
    }
    Ok(())
}

fn git_output(cwd: &str, args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn default_commit_message(request: &Request) -> String {
    let subject = if !request.title.trim().is_empty() {
        request.title.trim()
    } else if !request.change_name.trim().is_empty() {
        request.change_name.trim()
    } else {
        &request.request_id
    };
    format!("feat: {}", subject.replace(['\n', '\r', '\t'], " "))
}

fn validate_commit_message(message: &str) -> Result<()> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err("commit message must not be empty".into());
    }
    if !(trimmed.contains(": ") || trimmed.contains('：')) {
        return Err("commit message must be conventional, for example: feat: add workflow".into());
    }
    Ok(())
}

fn github_compare_url(git_url: &str, base_branch: &str, head_branch: &str) -> Option<String> {
    let without_suffix = git_url.strip_suffix(".git").unwrap_or(git_url);
    let repo = if let Some(rest) = without_suffix.strip_prefix("https://github.com/") {
        rest.to_string()
    } else if let Some(rest) = without_suffix.strip_prefix("git@github.com:") {
        rest.to_string()
    } else {
        return None;
    };
    if repo.split('/').count() < 2 {
        return None;
    }
    Some(format!(
        "https://github.com/{repo}/compare/{base}...{head}?expand=1",
        repo = repo,
        base = url_path_escape(base_branch),
        head = url_path_escape(head_branch),
    ))
}

fn render_pr_issue_reference(request: &Request) -> String {
    let mut lines = Vec::new();
    lines.push(format!("- Request ID: `{}`", request.request_id));
    if !request.external_id.trim().is_empty() {
        lines.push(format!("- External ID: `{}`", request.external_id));
    }
    if !request.url.trim().is_empty() {
        lines.push(format!("- URL: {}", request.url));
    }
    if let Some(closing) = github_issue_closing_reference(&request.external_id) {
        lines.push(format!("- GitHub auto-link: {closing}"));
    }
    lines.join("\n")
}

fn github_issue_closing_reference(external_id: &str) -> Option<String> {
    let rest = external_id.strip_prefix("github:")?;
    if !rest.contains('#') {
        return None;
    }
    Some(format!("Closes {rest}"))
}

fn url_path_escape(value: &str) -> String {
    value.replace('/', "%2F")
}

fn render_preflight_notes(preflight: &PlanPreflight) -> String {
    if preflight.notes.is_empty() {
        return "- 未记录计划前检查。".to_string();
    }
    preflight
        .notes
        .iter()
        .map(|note| format!("- {note}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn ensure_allowed_flags(args: &[String], allowed: &[&str]) -> Result<()> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if !arg.starts_with("--") {
            return Err(format!("unexpected positional argument: {arg}").into());
        }
        if !allowed.iter().any(|allowed| *allowed == arg) {
            return Err(format!("unknown flag: {arg}").into());
        }
        index += 2;
    }
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Result<Option<String>> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == flag {
            let Some(value) = args.get(index + 1) else {
                return Err(format!("{flag} requires a value").into());
            };
            if value.starts_with("--") {
                return Err(format!("{flag} requires a value").into());
            }
            return Ok(Some(value.clone()));
        }
        index += 2;
    }
    Ok(None)
}

fn required_flag(args: &[String], flag: &str) -> Result<String> {
    flag_value(args, flag)?.ok_or_else(|| format!("{flag} is required").into())
}

fn required_request_id(args: &[String]) -> Result<String> {
    if let Some(value) = flag_value(args, "--request_id")? {
        return Ok(value);
    }
    required_flag(args, "--request-id")
}

fn required_gate(args: &[String]) -> Result<String> {
    let gate = required_flag(args, "--gate")?;
    validate_gate(&gate)?;
    Ok(gate)
}

fn validate_gate(gate: &str) -> Result<()> {
    match gate {
        "decomposition" | "plan" | "change-doc" => Ok(()),
        _ => Err("gate must be `decomposition`, `plan`, or `change-doc`".into()),
    }
}

fn validate_session_phase(phase: &str) -> Result<()> {
    match phase {
        "decomposition" | "planning" | "implementation" | "rebase" => Ok(()),
        _ => Err("phase must be `decomposition`, `planning`, `implementation`, or `rebase`".into()),
    }
}

fn flag_present(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn validate_change_name(change_name: &str) -> Result<()> {
    let bytes = change_name.as_bytes();
    let date_shape = bytes.len() > 11
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'-');
    if !date_shape {
        return Err("change name must use YYYY-MM-DD-short-english-name".into());
    }
    if !change_name
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(
            "change name must use lowercase ASCII letters, digits, and hyphens only".into(),
        );
    }
    if change_name.contains("--") || change_name.ends_with('-') {
        return Err("change name must not contain empty hyphen segments".into());
    }
    Ok(())
}

fn upgrade_change_artifacts(request: &Request, dry_run: bool) -> Result<()> {
    ensure_prefixed_change_artifact_names(request, dry_run)?;
    remove_obsolete_change_artifacts(request, dry_run)?;
    let preflight = PlanPreflight {
        notes: vec!["upgrade 迁移生成的模板；正式计划前必须重新运行 plan preflight。".to_string()],
    };
    let artifacts = [
        ("request.md", render_request(request)),
        ("plan.md", render_plan_template(request, &preflight)),
        ("change-doc.md", render_change_doc_template(request)),
        ("pr-doc.md", render_pr_doc_template(request)),
        ("agent-journal.md", render_agent_journal_template(request)),
    ];

    for (file, content) in artifacts {
        if !request_generates_markdown_artifact(request, file) {
            continue;
        }
        let path = request_artifact_path_buf(request, file);
        if should_write_managed_artifact(&path)? {
            if dry_run {
                println!("Would update {}", path.display());
            } else {
                fs::write(&path, content)?;
                println!("Updated {}", path.display());
            }
        }
    }
    let status_path = Path::new(&request.change_path).join("status.json");
    if dry_run {
        println!("Would refresh {}", status_path.display());
    } else {
        let existed = status_path.exists();
        write_status_json(
            request,
            stage_for_status_json(&request.status),
            &request.status,
            "upgrade refreshed status paths",
        )?;
        if existed {
            println!("Refreshed {}", status_path.display());
        } else {
            println!("Created {}", status_path.display());
        }
    }
    Ok(())
}

fn migrate_legacy_change_paths(requests: &mut [Request], dry_run: bool) -> Result<()> {
    for request in requests {
        let Some(change_name) = legacy_change_name(&request.change_path, &request.change_name)
        else {
            continue;
        };
        let new_path = change_artifact_path(&change_name);
        if request.change_path == new_path {
            continue;
        }
        if dry_run {
            println!(
                "Would migrate {} change path: {} -> {}",
                request.request_id, request.change_path, new_path
            );
            continue;
        }
        let old_path = Path::new(&request.change_path);
        let target_path = Path::new(&new_path);
        if old_path.exists() && !target_path.exists() {
            copy_dir_recursive(old_path, target_path)?;
            println!("Copied {} to {}", old_path.display(), target_path.display());
        } else if !target_path.exists() {
            fs::create_dir_all(target_path)?;
            println!("Created {}", target_path.display());
        } else {
            println!("Using existing {}", target_path.display());
        }
        request.change_name = change_name;
        request.change_path = new_path;
        request.updated_at = now_string();
    }
    Ok(())
}

fn legacy_change_name(change_path: &str, change_name: &str) -> Option<String> {
    let trimmed = change_path.trim();
    if !trimmed.starts_with("docs/changes/") {
        return None;
    }
    if !change_name.trim().is_empty() {
        return Some(change_name.trim().to_string());
    }
    Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else if file_type.is_file() && !destination_path.exists() {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn stage_for_status_json(status: &str) -> &'static str {
    match canonical_status(status) {
        "decomposition"
        | "decomposition-agent-running"
        | "decomposition-submitted"
        | "decomposition-review-running"
        | "decomposition-review-rejected"
        | "decomposition-approved" => "decomposition",
        "discovered"
        | "planning"
        | "planning-agent-running"
        | "plan-submitted"
        | "plan-review-running"
        | "plan-review-rejected"
        | "plan-approved" => "planning",
        "in-progress"
        | "implementation-agent-running"
        | "change-doc-submitted"
        | "change-doc-approved"
        | "code-review-running"
        | "code-review-rejected" => "implementation",
        "rebase-agent-running" | "integration-review-rejected" => "rebase",
        "integration-review-submitted" | "integration-review-running" => "integration-review",
        STATUS_WAIT_UPDATE_PR | STATUS_WAIT_FINISH | STATUS_FINISHED => "delivery",
        "blocked" => "blocked",
        _ => "planning",
    }
}

fn should_write_managed_artifact(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    let content = fs::read_to_string(path)?;
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.ends_with("agent-journal.md"))
        .unwrap_or(false)
    {
        return Ok(content.trim().is_empty()
            || content.contains("# Thread Handoff")
            || content.contains("Codex Plan Prompt")
            || content.contains("Codex Start Prompt"));
    }
    Ok(content.contains("This is a template. Codex")
        || content.contains("This HTML file is a visual planning template")
        || content.contains("Start a new Codex thread")
        || content.contains("# Thread Handoff")
        || content.contains("Codex Plan Prompt")
        || content.contains("Codex Start Prompt")
        || content.contains("这是空白计划模板")
        || content.contains("待填写")
        || content.contains("这是计划模板")
        || content.contains("这是规格模板")
        || content.contains("这是任务模板")
        || content.contains("这是变更文档模板"))
}

fn repo_name_from_url(git_url: &str) -> String {
    let without_suffix = git_url.strip_suffix(".git").unwrap_or(git_url);
    without_suffix
        .rsplit(['/', ':'])
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("repo")
        .to_string()
}

fn usage(command: &str) -> Result<()> {
    Err(format!("usage: sandrone {command}").into())
}

fn print_help() {
    println!(
        "Usage: sandrone <command>\n\nCommands:\n  new (--url <git-url> | --name <project-name>)\n  update\n  list\n  dashboard [--host 127.0.0.1] [--port 47217] [--json]\n  status [REQ-0001]\n  validate\n  tick [--request_id <REQ-0001>] [--max-attempts <n>] [--parallel-limit 1] [--auto-merge]\n  advance --request_id <REQ-0001> [--max-attempts <n>]\n  doctor\n  doc-status --request_id <REQ-0001> [--phase <decomposition|planning|implementation|rebase>]\n  obsidian-refresh\n  decompose --name <YYYY-MM-DD-short-name> --request_id <REQ-0001>\n  plan --name <YYYY-MM-DD-short-name> --request_id <REQ-0001>\n  submit --request_id <REQ-0001> --gate <decomposition|plan|change-doc>\n  approve --request_id <REQ-0001> --gate <decomposition|plan|change-doc> --by <actor>\n  reject --request_id <REQ-0001> --gate <decomposition|plan|change-doc> --by <actor>\n  gates --request_id <REQ-0001> [--json]\n  decomposition-review --request_id <REQ-0001>\n  plan-review --request_id <REQ-0001>\n  code-review --request_id <REQ-0001>\n  integration-review --request_id <REQ-0001>\n  start --request_id <REQ-0001>\n  finish --request_id <REQ-0001> [--message \"feat: ...\"]\n  pr-status --request_id <REQ-0001>\n  pr-merge --request_id <REQ-0001> [--queue-decision ready_for_merge] [--auto-merge]\n  pr-refresh --request_id <REQ-0001> [--mode <start|continue>] [--max-attempts <n>]\n  block --request_id <REQ-0001> --stage <stage> --reason <reason>\n  resume --request_id <REQ-0001>\n  session --request_id <REQ-0001> --phase <decomposition|planning|implementation|rebase> [--thread_id <id>] [--thread_url <url>] [--status <status>]\n  sessions [--json]\n  upgrade [--dry-run] [--default]\n\nAliases:\n  approvals -> gates\n\nReview attempt defaults:\n  decomposition-review: {DEFAULT_DECOMPOSITION_MAX_ATTEMPTS}\n  plan-review: {DEFAULT_PLAN_MAX_ATTEMPTS}\n  code-review: {DEFAULT_CODE_MAX_ATTEMPTS}\n  integration-review: {DEFAULT_INTEGRATION_MAX_ATTEMPTS}\n\n--max-attempts <n> overrides the default for the current automatic run."
    );
}

fn dashboard_html() -> &'static str {
    assets::DASHBOARD_HTML
}

#[cfg(test)]
mod tests {
    use super::{
        AgentPhase, DEFAULT_CODE_MAX_ATTEMPTS, DEFAULT_DECOMPOSITION_MAX_ATTEMPTS,
        DEFAULT_INTEGRATION_MAX_ATTEMPTS, DEFAULT_PLAN_MAX_ATTEMPTS, dashboard_html,
        resolve_max_attempts,
    };

    #[test]
    fn review_attempt_defaults_are_phase_specific_and_overridable() {
        assert_eq!(
            resolve_max_attempts(AgentPhase::Decomposition, None),
            DEFAULT_DECOMPOSITION_MAX_ATTEMPTS
        );
        assert_eq!(
            resolve_max_attempts(AgentPhase::Planning, None),
            DEFAULT_PLAN_MAX_ATTEMPTS
        );
        assert_eq!(
            resolve_max_attempts(AgentPhase::Implementation, None),
            DEFAULT_CODE_MAX_ATTEMPTS
        );
        assert_eq!(
            resolve_max_attempts(AgentPhase::Rebase, None),
            DEFAULT_INTEGRATION_MAX_ATTEMPTS
        );
        assert_eq!(resolve_max_attempts(AgentPhase::Planning, Some(9)), 9);
    }

    #[test]
    fn dashboard_html_uses_list_requests_and_rich_artifact_renderers() {
        let html = dashboard_html();
        assert!(html.contains("display: flex;"));
        assert!(html.contains("class=\"request-list\""));
        assert!(html.contains("timeline-track"));
        assert!(html.contains("timeline-main"));
        assert!(html.contains("timeline-branch"));
        assert!(html.contains("hasIntegrationFlow"));
        assert!(html.contains("isLowerTimelineStage"));
        assert!(html.contains("orderedLowerTimelineStages"));
        assert!(html.contains("lowerStageMarker"));
        assert!(html.contains("updateIntegrationConnector"));
        assert!(html.contains("request.pr?.stages?.length"));
        assert!(html.contains("prPaneSubtitle"));
        assert!(html.contains("kind: \"pr\""));
        assert!(html.contains("pane.kind === \"pr\""));
        assert!(html.contains("visibleTimelineItems(indexedStages, hasIntegration, pane.kind)"));
        assert!(html.contains("renderArtifactTabs"));
        assert!(html.contains("Review 结果"));
        assert!(html.contains("integration-connector-path"));
        assert!(html.contains("stroke: var(--line-strong);"));
        assert!(html.contains(".stage.branch .dot { border-color: var(--line-strong);"));
        assert!(!html.contains("stroke: #d6a31f;"));
        assert!(!html.contains("background: #e7c46a;"));
        assert!(html.contains("marked.min.js"));
        assert!(html.contains("DOMPurify"));
        assert!(html.contains("highlight.js"));
        assert!(html.contains("jsoneditor.min.js"));
        assert!(html.contains("renderMarkdownContent"));
        assert!(html.contains("mountJsonViewer"));
        assert!(html.contains("data-json-detail"));
        assert!(html.contains("reviewerDisplayStatus(item)"));
        assert!(html.contains("reviewerHasDetail(reviewer)"));
        assert!(!html.contains("item.runtime_status || item.decision"));
        assert!(html.contains("orderedRequests(project)"));
        assert!(html.contains("request.status === \"finished\" ? 1 : 0"));
        assert!(html.contains("PR 待合并"));
        assert!(html.contains("tag(\"blocked\", \"blocked\""));
        assert!(html.contains("tag(\"pending\", \"pending\""));
        assert!(html.contains("tag(\"finish\", \"finish\""));
        assert!(html.contains("if (status === \"finished\") return \"done\";"));
        assert!(html.contains("querySelector('[data-stage-id=\"code-review\"]')"));
        assert!(!html.contains("querySelector('[data-stage-id=\"implementation\"]')"));
        assert!(!html.contains(
            "status === \"wait-update-pr\" || status === \"change-doc-approved\") return \"done\""
        ));
        assert!(!html.contains("tag(\"waiting\""));
        assert!(!html.contains("tag(\"running\""));
        assert!(!html.contains("tag(\"\", \"requests\""));
    }
}
