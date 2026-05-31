use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const CONFIG_PATH: &str = ".codex-auto-dev/config.toml";
const STATE_PATH: &str = ".codex-auto-dev/state/requests.tsv";
const EVENTS_PATH: &str = ".codex-auto-dev/state/events.ndjson";
const SESSIONS_PATH: &str = ".codex-auto-dev/sessions.json";
const FRAMEWORK_SCHEMA_VERSION: u32 = 3;
const DEV_REPO: &str = "dev/repo";
const WORKTREES: &str = "dev/worktrees";
const ISSUE_TOOL: &str = "tools/issue-update.sh";
const ISSUE_AGENT_TOOL: &str = "tools/issue-agent.sh";
const PR_TOOL: &str = "tools/pr-create.sh";
const PLAN_REVIEW_TOOL: &str = "tools/plan-review.sh";
const TEST_REVIEW_TOOL: &str = "tools/test-review.sh";
const DESIGN_REVIEW_TOOL: &str = "tools/design-review.sh";
const ISSUE_AGENT_PROMPT: &str = "tools/prompts/issue-agent.md";
const PLAN_AGENT_PROMPT: &str = "tools/prompts/plan-agent.md";
const IMPLEMENTATION_AGENT_PROMPT: &str = "tools/prompts/implementation-agent.md";
const PLAN_REVIEW_PROMPT: &str = "tools/prompts/plan-reviewer.md";
const TEST_REVIEW_PROMPT: &str = "tools/prompts/test-reviewer.md";
const DESIGN_REVIEW_PROMPT: &str = "tools/prompts/design-reviewer.md";
const REVIEW_SCHEMA: &str = "tools/schemas/review-result.schema.json";
const ISSUE_TOOL_EXAMPLE: &str = "tools/issue-update.example.sh";
const ISSUE_AGENT_TOOL_EXAMPLE: &str = "tools/issue-agent.example.sh";
const PR_TOOL_EXAMPLE: &str = "tools/pr-create.example.sh";
const PLAN_REVIEW_TOOL_EXAMPLE: &str = "tools/plan-review.example.sh";
const TEST_REVIEW_TOOL_EXAMPLE: &str = "tools/test-review.example.sh";
const DESIGN_REVIEW_TOOL_EXAMPLE: &str = "tools/design-review.example.sh";
const ISSUE_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/issue-agent.example.md";
const PLAN_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/plan-agent.example.md";
const IMPLEMENTATION_AGENT_PROMPT_EXAMPLE: &str = "tools/prompts/implementation-agent.example.md";
const PLAN_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/plan-reviewer.example.md";
const TEST_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/test-reviewer.example.md";
const DESIGN_REVIEW_PROMPT_EXAMPLE: &str = "tools/prompts/design-reviewer.example.md";
const REVIEW_SCHEMA_EXAMPLE: &str = "tools/schemas/review-result.example.schema.json";
const WORKFLOW_SKILL: &str = "skills/codex-auto-dev-workflow/SKILL.md";
const WORKFLOW_SKILL_CONTENT: &str = include_str!("../skills/codex-auto-dev-workflow/SKILL.md");

#[derive(Clone, Debug)]
struct Config {
    schema_version: u32,
    repo_name: String,
    git_url: String,
    base_branch: String,
    parallel_limit: usize,
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

#[derive(Clone, Debug)]
struct DeliveryResult {
    commit_message: String,
    branch: String,
    pr_url: Option<String>,
    pr_status: String,
    compare_url: Option<String>,
    pr_error: String,
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
enum CodegraphInitOutcome {
    SkippedEmptyRepo,
    AlreadyInitialized,
    Initialized,
    CommandUnavailable(String),
    Failed(String),
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
    Planning,
    Implementation,
}

impl AgentPhase {
    fn as_str(self) -> &'static str {
        match self {
            AgentPhase::Planning => "planning",
            AgentPhase::Implementation => "implementation",
        }
    }

    fn running_status(self) -> &'static str {
        match self {
            AgentPhase::Planning => "planning-agent-running",
            AgentPhase::Implementation => "implementation-agent-running",
        }
    }

    fn prompt_path(self) -> &'static str {
        match self {
            AgentPhase::Planning => PLAN_AGENT_PROMPT,
            AgentPhase::Implementation => IMPLEMENTATION_AGENT_PROMPT,
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
        "plan" => create_plan_packet(&args),
        "submit" => submit_approval(&args),
        "approve" => decide_approval(&args, "approved"),
        "reject" => decide_approval(&args, "rejected"),
        "approvals" => show_approvals(&args),
        "plan-review" => plan_review(&args),
        "code-review" => code_review(&args),
        "start" => start_worktree(&args),
        "finish" => finish_request(&args),
        "block" => block_request(&args),
        "resume" => resume_request(&args),
        "session" => register_session(&args),
        "sessions" => list_sessions(&args),
        "upgrade" => upgrade_workspace(&args),
        "list" => list_requests(),
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
    let url = flag_value(args, "--url")?;
    let name = flag_value(args, "--name")?;

    match (url, name) {
        (Some(_), Some(_)) => usage("new (--url <git-url> | --name <project-name>)"),
        (None, None) => usage("new (--url <git-url> | --name <project-name>)"),
        (Some(git_url), None) => initialize_cloned_workspace(&git_url),
        (None, Some(repo_name)) => initialize_empty_workspace(&repo_name),
    }
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
    write_default_review_tools()?;
    refresh_default_reference_examples()?;
    write_default_workflow_skill()?;

    println!("Created codex-auto-dev workspace");
    println!("  mode: clone");
    println!("  workspace naming: arbitrary outer workspace name is OK for cloned repositories");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  issue agent: {ISSUE_AGENT_TOOL}");
    println!("  PR tool: {PR_TOOL}");
    println!("  review tools: {PLAN_REVIEW_TOOL}, {TEST_REVIEW_TOOL}, {DESIGN_REVIEW_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    if repo_has_commits(DEV_REPO) {
        let codegraph_outcome = ensure_codegraph_initialized(DEV_REPO);
        print_codegraph_init_outcome("  ", &codegraph_outcome);
        println!("  repository has content: CodeGraph project preview required before planning");
        println!(
            "  next: run the codegraph-project-preview skill to generate docs/codegraph/context.md, then wait for a request"
        );
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
            "  next: wait for a request, then codex-auto-dev plan --name <YYYY-MM-DD-name> --request_id <REQ-0001>"
        );
    }
    append_event(
        "workspace_initialized",
        "",
        "clone",
        "ready",
        &format!("repo={repo_name}; git_url={git_url}"),
    )?;
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
    write_default_review_tools()?;
    refresh_default_reference_examples()?;
    write_default_workflow_skill()?;

    println!("Created codex-auto-dev workspace");
    println!("  mode: empty");
    println!("  project name: {repo_name}");
    println!("  workspace naming: use an outer workspace directory named {repo_name}-auto-dev");
    println!("  target git repository name: {repo_name}");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  issue agent: {ISSUE_AGENT_TOOL}");
    println!("  PR tool: {PR_TOOL}");
    println!("  review tools: {PLAN_REVIEW_TOOL}, {TEST_REVIEW_TOOL}, {DESIGN_REVIEW_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    println!(
        "  next: codex-auto-dev plan --name {}-initial-plan --request_id REQ-0001",
        today()
    );
    append_event(
        "workspace_initialized",
        "",
        "empty",
        "ready",
        &format!("repo={repo_name}; git_url=local:{repo_name}"),
    )?;
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
        ],
    )?;
    let request_id = flag_value(args, "--request_id")?.or(flag_value(args, "--request-id")?);
    let config = load_config()?;
    let parallel_limit = parse_parallel_limit(
        flag_value(args, "--parallel-limit")?.or(flag_value(args, "--parallel_limit")?),
        config.parallel_limit,
    )?;
    let max_attempts = parse_max_attempts(flag_value(args, "--max-attempts")?)?;

    update_requests()?;

    let refreshed = refresh_tick_statuses()?;
    if refreshed > 0 {
        println!("Tick refreshed {refreshed} request status(es).");
    }

    let requests = load_requests()?;
    let mut request_ids = select_tick_requests(&requests, request_id.as_deref())?;
    if request_ids.is_empty() {
        println!("Tick complete: no pending request.");
        return Ok(());
    }
    let running_count = running_issue_agent_count(&requests);
    if running_count >= parallel_limit {
        println!(
            "Tick parallel limit reached: {running_count}/{parallel_limit} issue-agent(s) already running."
        );
        return Ok(());
    }
    let available_slots = parallel_limit - running_count;
    let delayed_by_limit = request_ids.len().saturating_sub(available_slots);
    request_ids.truncate(available_slots);

    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        return Err(format!("{ISSUE_AGENT_TOOL} does not exist").into());
    }

    let mut preflight = None;
    let mut dispatched = Vec::new();
    let mut failures = Vec::new();
    for request_id in request_ids {
        let Some(_lock) = RequestLock::acquire(&request_id)? else {
            continue;
        };
        match dispatch_next_agent_for_request(&request_id, max_attempts, &mut preflight) {
            Ok(Some((request, phase, pid))) => dispatched.push((request, phase, pid)),
            Ok(None) => {}
            Err(error) => {
                failures.push(format!("{request_id}: {error}"));
            }
        }
    }

    if dispatched.is_empty() && failures.is_empty() {
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
            continue;
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

fn doctor(args: &[String]) -> Result<()> {
    ensure_allowed_flags(args, &[])?;
    let codegraph_bin = codegraph_bin();
    let checks = vec![
        doctor_check(
            "Workspace",
            Path::new(CONFIG_PATH).exists() && Path::new(STATE_PATH).exists(),
            "workspace metadata exists",
            "run codex-auto-dev new first",
            true,
        ),
        doctor_command_check("Git", "git", &["--version"], false),
        doctor_command_check("Codex CLI", "codex", &["--version"], true),
        doctor_command_check("GitHub CLI", "gh", &["--version"], true),
        doctor_command_check("CodeGraph CLI", &codegraph_bin, &["--version"], true),
        doctor_check(
            "Target repo",
            Path::new(DEV_REPO).join(".git").exists(),
            "target repository exists",
            "target repository is missing",
            false,
        ),
        doctor_check(
            "Agent tools",
            Path::new(ISSUE_TOOL).exists() && Path::new(ISSUE_AGENT_TOOL).exists(),
            "issue update and issue agent connectors exist",
            "missing issue update or issue agent connector",
            false,
        ),
        doctor_check(
            "Reviewer tools",
            Path::new(PLAN_REVIEW_TOOL).exists()
                && Path::new(TEST_REVIEW_TOOL).exists()
                && Path::new(DESIGN_REVIEW_TOOL).exists(),
            "plan/test/design reviewer connectors exist",
            "missing one or more reviewer connectors",
            false,
        ),
        doctor_check(
            "Review schema",
            review_schema_ready(),
            "strict review-result schema contains required gate fields",
            "review schema is missing required strict fields",
            false,
        ),
        doctor_check(
            "Events stream",
            Path::new(EVENTS_PATH).parent().is_some_and(Path::exists),
            "state directory is ready for events.ndjson",
            "state directory is missing",
            false,
        ),
        doctor_check(
            "CodeGraph index",
            !repo_has_commits(DEV_REPO) || codegraph_index_ready(DEV_REPO),
            "target repo is empty or dev/repo/.codegraph exists",
            "target repo has commits but dev/repo/.codegraph is missing; run codex-auto-dev plan or codegraph init -i dev/repo",
            true,
        ),
    ];

    println!("Codex Auto Dev Doctor Report");
    println!();
    for check in &checks {
        println!(
            "{:<15} {:<5} {}",
            check.name,
            check.status_label(),
            check.detail
        );
    }

    let failures = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Warn)
        .count();
    println!();
    println!("Summary: {failures} failed, {warnings} warning(s)");
    if failures > 0 {
        Err("doctor found blocking issue(s)".into())
    } else {
        Ok(())
    }
}

fn doctor_check(
    name: &'static str,
    ok: bool,
    ok_detail: &str,
    problem_detail: &str,
    blocking: bool,
) -> DoctorCheck {
    DoctorCheck {
        name,
        status: if ok {
            DoctorStatus::Ok
        } else if blocking {
            DoctorStatus::Fail
        } else {
            DoctorStatus::Warn
        },
        detail: if ok {
            ok_detail.to_string()
        } else {
            problem_detail.to_string()
        },
    }
}

fn doctor_command_check(
    name: &'static str,
    program: &str,
    args: &[&str],
    optional: bool,
) -> DoctorCheck {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("installed")
                .trim()
                .to_string();
            DoctorCheck {
                name,
                status: DoctorStatus::Ok,
                detail: fallback_empty(&version, "installed").to_string(),
            }
        }
        Ok(output) => {
            let stderr = review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
            DoctorCheck {
                name,
                status: if optional {
                    DoctorStatus::Warn
                } else {
                    DoctorStatus::Fail
                },
                detail: format!("{program} failed: {stderr}"),
            }
        }
        Err(error) => DoctorCheck {
            name,
            status: if optional {
                DoctorStatus::Warn
            } else {
                DoctorStatus::Fail
            },
            detail: format!("{program} unavailable: {error}"),
        },
    }
}

fn review_schema_ready() -> bool {
    let Ok(content) = fs::read_to_string(REVIEW_SCHEMA) else {
        return false;
    };
    [
        "\"reviewer\"",
        "\"approved\"",
        "\"gate_unavailable\"",
        "\"decision\"",
        "\"recommended_next_phase\"",
        "\"summary\"",
        "\"process\"",
        "\"critical\"",
        "\"high\"",
        "\"warning\"",
        "\"info\"",
        "\"impact\"",
        "\"suggested_change\"",
        "\"verification\"",
        "\"additionalProperties\": false",
    ]
    .iter()
    .all(|needle| content.contains(needle))
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
    println!("  plan template: {}/plan.md", request.change_path);
    println!("  request: {}/request.md", request.change_path);
    println!("  Codex or planning agent must fill the plan; outer tick runs review gates.");
    Ok(())
}

fn create_plan_packet_for_index(
    requests: &mut [Request],
    index: usize,
    change_name: &str,
    preflight: &PlanPreflight,
) -> Result<Request> {
    let mut request = requests[index].clone();
    request.change_name = change_name.to_string();
    request.change_path = format!("docs/changes/{change_name}");
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
    write_approval_record(&request, &gate, "submitted", "", "manual-cli", "")?;
    request.status = format!("{}-submitted", gate_status_prefix(&gate));
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    update_gate_session(&request, &gate, "waiting-approval")?;

    println!("Approval submitted for {request_id}");
    println!("  gate: {gate}");
    println!(
        "  approval: {}",
        approval_file_path(&request, &gate).display()
    );
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
    write_approval_record(&request, &gate, decision, &by, &source, &comment)?;
    request.status = format!("{}-{decision}", gate_status_prefix(&gate));
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    update_gate_session(&request, &gate, decision)?;

    println!("Approval decision recorded for {request_id}");
    println!("  gate: {gate}");
    println!("  status: {decision}");
    println!(
        "  approval: {}",
        approval_file_path(&request, &gate).display()
    );
    Ok(())
}

fn show_approvals(args: &[String]) -> Result<()> {
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
        || vec!["plan".to_string(), "change-doc".to_string()],
        |value| vec![value],
    );
    if flag_present(args, "--json") {
        println!("{{");
        println!("  \"request_id\": \"{}\",", json_escape(&request_id));
        println!("  \"approvals\": [");
        for (index, gate) in gates.iter().enumerate() {
            let path = approval_file_path(request, gate);
            if index > 0 {
                println!(",");
            }
            if path.exists() {
                let content = fs::read_to_string(path)?;
                print!("{}", indent_json_object(&content, 4));
            } else {
                println!(
                    "    {{ \"gate\": \"{}\", \"status\": \"missing\" }}",
                    json_escape(gate)
                );
            }
        }
        println!();
        println!("  ]");
        println!("}}");
    } else {
        for gate in gates {
            let path = approval_file_path(request, &gate);
            let status = if path.exists() {
                let content = fs::read_to_string(&path)?;
                json_value(&content, "status").unwrap_or_else(|| "unknown".to_string())
            } else {
                "missing".to_string()
            };
            println!("{:<10} {:<12} {}", gate, status, path.display());
        }
    }
    Ok(())
}

fn plan_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;

    let reviewers = [ReviewDefinition {
        name: "PlanReviewer",
        tool: PLAN_REVIEW_TOOL,
        file_stem: "plan-reviewer",
    }];
    let results = run_review_stage(&request, "plan-review", &reviewers)?;
    if reviews_approved(&results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "plan",
            "PlanReviewer",
            "plan-review",
            "PlanReviewer approved the plan review gate",
        )?;
        println!("Plan review approved for {request_id}");
        println!(
            "  review summary: {}/reviews/plan-review/summary.json",
            request.change_path
        );
        println!(
            "  approval: {}",
            approval_file_path(&request, "plan").display()
        );
        Ok(())
    } else {
        if review_gate_unavailable(&results) {
            let reason = review_gate_unavailable_reason("plan-review", &results);
            mark_blocked(&mut requests, index, &mut request, "planning", &reason)?;
            return Err(format!(
                "{} review gate unavailable: {reason}",
                rejected_reviewers(&results).join(", ")
            )
            .into());
        }
        mark_review_rejected(
            &mut requests,
            index,
            &mut request,
            "planning",
            "plan-review",
            "plan-review rejected; return to planning",
        )?;
        let rejected = rejected_reviewers(&results);
        Err(format!("{} rejected plan review", rejected.join(", ")).into())
    }
}

fn code_review(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    ensure_change_packet(&request)?;
    ensure_gate_approved(&request, "plan")?;
    if request.worktree_path.trim().is_empty() {
        return Err(
            format!("{request_id} has no worktree. Run codex-auto-dev start first.").into(),
        );
    }

    let reviewers = [
        ReviewDefinition {
            name: "TestReviewer",
            tool: TEST_REVIEW_TOOL,
            file_stem: "test-reviewer",
        },
        ReviewDefinition {
            name: "DesignReviewer",
            tool: DESIGN_REVIEW_TOOL,
            file_stem: "design-reviewer",
        },
    ];
    let results = run_review_stage(&request, "code-review", &reviewers)?;
    if reviews_approved(&results) {
        approve_gate_from_review(
            &mut requests,
            index,
            &mut request,
            "change-doc",
            "code-review",
            "code-review",
            "TestReviewer and DesignReviewer approved the code review gate",
        )?;
        println!("Code review approved for {request_id}");
        println!(
            "  review summary: {}/reviews/code-review/summary.json",
            request.change_path
        );
        println!(
            "  approval: {}",
            approval_file_path(&request, "change-doc").display()
        );
        Ok(())
    } else {
        if review_gate_unavailable(&results) {
            let reason = review_gate_unavailable_reason("code-review", &results);
            mark_blocked(
                &mut requests,
                index,
                &mut request,
                "implementation",
                &reason,
            )?;
            return Err(format!(
                "{} review gate unavailable: {reason}",
                rejected_reviewers(&results).join(", ")
            )
            .into());
        }
        match recommended_next_phase(&results, "implementation").as_str() {
            "planning" => {
                mark_review_rejected(
                    &mut requests,
                    index,
                    &mut request,
                    "planning",
                    "plan-review",
                    "code-review requested planning; reviewer findings require plan revision",
                )?;
            }
            "blocked" => {
                mark_blocked(
                    &mut requests,
                    index,
                    &mut request,
                    "implementation",
                    "code-review recommended blocking; manual recovery is required",
                )?;
            }
            _ => {
                mark_review_rejected(
                    &mut requests,
                    index,
                    &mut request,
                    "implementation",
                    "code-review",
                    "code-review rejected; return to implementation",
                )?;
            }
        }
        let rejected = rejected_reviewers(&results);
        Err(format!("{} rejected code review", rejected.join(", ")).into())
    }
}

fn run_review_stage(
    request: &Request,
    stage: &str,
    reviewers: &[ReviewDefinition],
) -> Result<Vec<ReviewResult>> {
    let review_dir = Path::new(&request.change_path).join("reviews").join(stage);
    let details_dir = review_dir.join("details");
    fs::create_dir_all(&review_dir)?;
    fs::create_dir_all(&details_dir)?;
    let attempt = next_review_attempt(&details_dir)?;
    let mut results = Vec::new();
    for reviewer in reviewers {
        results.push(run_single_reviewer(
            request,
            stage,
            reviewer,
            &details_dir,
            attempt,
        )?);
    }
    write_review_summary(request, stage, attempt, &results)?;
    update_change_doc_review_section(request)?;
    Ok(results)
}

fn run_single_reviewer(
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    details_dir: &Path,
    attempt: u32,
) -> Result<ReviewResult> {
    let output_path = details_dir.join(format!("{attempt:03}-{}.json", reviewer.file_stem));
    let review_context = prepare_review_context(request, stage, reviewer, attempt)?;
    let review_context_string = absolute_path_string(&review_context);
    let forbidden_review_paths = format!(
        "{};{}",
        absolute_path_string(Path::new(&request.change_path).join("reviews")),
        absolute_path_string(Path::new(&request.change_path).join("reviews").join(stage)),
    );
    let (content, tool_unavailable, diagnostic) = if !Path::new(reviewer.tool).exists() {
        let diagnostic = format!("{} does not exist", reviewer.tool);
        (
            rejected_review_json(reviewer.name, "review tool missing", &diagnostic),
            true,
            diagnostic,
        )
    } else {
        let output = Command::new("sh")
            .arg(reviewer.tool)
            .current_dir(".")
            .env("CODEX_AUTO_DEV_REVIEW_STAGE", stage)
            .env("CODEX_AUTO_DEV_REVIEWER", reviewer.name)
            .env("CODEX_AUTO_DEV_WORKSPACE", absolute_path_string("."))
            .env("CODEX_AUTO_DEV_TARGET_REPO", absolute_path_string(DEV_REPO))
            .env("CODEX_AUTO_DEV_REQUEST_ID", &request.request_id)
            .env("CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID", &request.external_id)
            .env("CODEX_AUTO_DEV_REQUEST_SOURCE", &request.source)
            .env("CODEX_AUTO_DEV_REQUEST_TITLE", &request.title)
            .env("CODEX_AUTO_DEV_REQUEST_BODY", &request.body)
            .env("CODEX_AUTO_DEV_REQUEST_URL", &request.url)
            .env("CODEX_AUTO_DEV_CHANGE_PATH", &review_context_string)
            .env("CODEX_AUTO_DEV_REVIEW_CONTEXT", &review_context_string)
            .env(
                "CODEX_AUTO_DEV_CANONICAL_CHANGE_PATH",
                absolute_path_string(request.change_path.as_str()),
            )
            .env(
                "CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS",
                forbidden_review_paths,
            )
            .env(
                "CODEX_AUTO_DEV_REQUEST",
                absolute_path_string(review_context.join("request.md")),
            )
            .env(
                "CODEX_AUTO_DEV_ISSUE",
                absolute_path_string(review_context.join("request.md")),
            )
            .env(
                "CODEX_AUTO_DEV_SPEC",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_PLAN",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_TASKS",
                absolute_path_string(review_context.join("plan.md")),
            )
            .env(
                "CODEX_AUTO_DEV_CHANGE_DOC",
                absolute_path_string(review_context.join("change-doc.md")),
            )
            .env(
                "CODEX_AUTO_DEV_WORKTREE",
                absolute_path_string(request.worktree_path.as_str()),
            )
            .env(
                "CODEX_AUTO_DEV_REVIEW_SCHEMA",
                absolute_path_string(REVIEW_SCHEMA),
            )
            .output();
        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8(output.stdout)?;
                if stdout.trim().is_empty() {
                    let diagnostic = "review tool succeeded without JSON output".to_string();
                    (
                        rejected_review_json(reviewer.name, "empty review output", &diagnostic),
                        true,
                        diagnostic,
                    )
                } else {
                    (stdout, false, String::new())
                }
            }
            Ok(output) => {
                let diagnostic =
                    review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
                (
                    rejected_review_json(reviewer.name, "review tool failed", &diagnostic),
                    true,
                    diagnostic,
                )
            }
            Err(error) => {
                let diagnostic = error.to_string();
                (
                    rejected_review_json(reviewer.name, "review tool could not run", &diagnostic),
                    true,
                    diagnostic,
                )
            }
        }
    };

    let (normalized, invalid_json) = normalize_review_json(reviewer.name, &content);
    fs::write(&output_path, ensure_trailing_newline(&normalized))?;
    let approved = json_bool(&normalized, "approved").unwrap_or(false);
    let has_blocking_findings = review_has_blocking_findings(&normalized);
    let reviewer_declared_unavailable = json_bool(&normalized, "gate_unavailable").unwrap_or(false);
    let recommended_next_phase = normalize_recommended_next_phase(
        &json_value(&normalized, "recommended_next_phase").unwrap_or_else(|| {
            default_recommended_next_phase(reviewer.name, approved, reviewer_declared_unavailable)
                .to_string()
        }),
        reviewer.name,
        approved,
        reviewer_declared_unavailable,
    );
    let summary = json_value(&normalized, "summary").unwrap_or_else(|| "no summary".to_string());
    let diagnostic = if invalid_json && diagnostic.is_empty() {
        review_diagnostic_excerpt(&content)
    } else {
        diagnostic
    };
    Ok(ReviewResult {
        reviewer: reviewer.name.to_string(),
        approved,
        has_blocking_findings,
        gate_unavailable: tool_unavailable || invalid_json || reviewer_declared_unavailable,
        recommended_next_phase,
        summary,
        diagnostic,
        path: output_path.to_string_lossy().to_string(),
    })
}

fn prepare_review_context(
    request: &Request,
    stage: &str,
    reviewer: &ReviewDefinition,
    attempt: u32,
) -> Result<PathBuf> {
    let context = Path::new(".codex-auto-dev/state/review-contexts")
        .join(&request.request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(slugify(reviewer.name));
    if context.exists() {
        fs::remove_dir_all(&context)?;
    }
    fs::create_dir_all(&context)?;
    for artifact in ["request.md", "plan.md", "change-doc.md", "status.json"] {
        let source = Path::new(&request.change_path).join(artifact);
        if source.exists() {
            fs::copy(&source, context.join(artifact))?;
        }
    }
    let approvals_source = Path::new(&request.change_path).join("approvals");
    if approvals_source.exists() {
        let approvals_target = context.join("approvals");
        fs::create_dir_all(&approvals_target)?;
        for entry in fs::read_dir(approvals_source)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), approvals_target.join(entry.file_name()))?;
            }
        }
    }
    Ok(context)
}

fn normalize_review_json(reviewer: &str, content: &str) -> (String, bool) {
    let trimmed = content.trim();
    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && json_bool(trimmed, "approved").is_some()
        && json_bool(trimmed, "gate_unavailable").is_some()
        && json_value(trimmed, "reviewer").is_some()
        && json_value(trimmed, "decision").is_some()
        && json_value(trimmed, "recommended_next_phase").is_some()
        && json_value(trimmed, "summary").is_some()
        && review_json_has_required_arrays(trimmed)
        && review_json_findings_have_required_fields(trimmed)
    {
        (trimmed.to_string(), false)
    } else {
        (
            rejected_review_json(
                reviewer,
                "invalid review JSON",
                "review tool must return one JSON object matching review-result.schema.json",
            ),
            true,
        )
    }
}

fn review_json_has_required_arrays(content: &str) -> bool {
    ["process", "critical", "high", "warning", "info"]
        .iter()
        .all(|key| content.contains(&format!("\"{key}\"")))
}

fn review_json_findings_have_required_fields(content: &str) -> bool {
    if !content.contains("\"title\"") {
        return true;
    }
    [
        "\"evidence\"",
        "\"impact\"",
        "\"required_fix\"",
        "\"suggested_change\"",
        "\"verification\"",
    ]
    .iter()
    .all(|needle| content.contains(needle))
}

fn default_recommended_next_phase(
    reviewer: &str,
    approved: bool,
    gate_unavailable: bool,
) -> &'static str {
    if gate_unavailable {
        "blocked"
    } else if approved {
        "implementation"
    } else if reviewer == "PlanReviewer" {
        "planning"
    } else {
        "implementation"
    }
}

fn normalize_recommended_next_phase(
    value: &str,
    reviewer: &str,
    approved: bool,
    gate_unavailable: bool,
) -> String {
    match value.trim() {
        "planning" | "implementation" | "blocked" => value.trim().to_string(),
        _ => default_recommended_next_phase(reviewer, approved, gate_unavailable).to_string(),
    }
}

fn review_diagnostic_excerpt(detail: &str) -> String {
    let collapsed = detail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut excerpt = String::new();
    for ch in collapsed.chars().take(500) {
        excerpt.push(ch);
    }
    if excerpt.is_empty() {
        "review tool failed without stderr diagnostics".to_string()
    } else {
        excerpt
    }
}

fn rejected_review_json(reviewer: &str, title: &str, detail: &str) -> String {
    format!(
        "{{\n  \"reviewer\": \"{}\",\n  \"approved\": false,\n  \"gate_unavailable\": true,\n  \"decision\": \"rejected\",\n  \"recommended_next_phase\": \"blocked\",\n  \"summary\": \"{}\",\n  \"process\": [\"review tool failure was converted into a blocking finding\"],\n  \"critical\": [{{ \"title\": \"{}\", \"evidence\": \"{}\", \"impact\": \"review gate cannot make a reliable approval decision\", \"required_fix\": \"Fix the reviewer tool or return valid structured review JSON.\", \"suggested_change\": \"Inspect the reviewer script stderr/stdout, restore the configured model backend, and make stdout exactly one JSON object matching tools/schemas/review-result.schema.json.\", \"verification\": \"Rerun the same codex-auto-dev review command and confirm the detail JSON validates and gate_unavailable is false.\" }}],\n  \"high\": [],\n  \"warning\": [],\n  \"info\": []\n}}",
        json_escape(reviewer),
        json_escape(title),
        json_escape(title),
        json_escape(detail),
    )
}

fn write_review_summary(
    request: &Request,
    stage: &str,
    attempt: u32,
    results: &[ReviewResult],
) -> Result<()> {
    let summary_path = Path::new(&request.change_path)
        .join("reviews")
        .join(stage)
        .join("summary.json");
    let approved = reviews_approved(results);
    let mut reviewers = String::new();
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            reviewers.push_str(",\n");
        }
        reviewers.push_str(&format!(
            "    {{ \"reviewer\": \"{}\", \"approved\": {}, \"has_blocking_findings\": {}, \"gate_unavailable\": {}, \"recommended_next_phase\": \"{}\", \"summary\": \"{}\", \"diagnostic\": \"{}\", \"path\": \"{}\" }}",
            json_escape(&result.reviewer),
            json_bool_literal(result.approved),
            json_bool_literal(result.has_blocking_findings),
            json_bool_literal(result.gate_unavailable),
            json_escape(&result.recommended_next_phase),
            json_escape(&result.summary),
            json_escape(&result.diagnostic),
            json_escape(&result.path),
        ));
    }
    fs::write(
        summary_path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"attempt\": {},\n  \"approved\": {},\n  \"reviewers\": [\n{}\n  ],\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(stage),
            attempt,
            json_bool_literal(approved),
            reviewers,
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn next_review_attempt(details_dir: &Path) -> Result<u32> {
    let mut max_attempt = 0;
    if details_dir.exists() {
        for entry in fs::read_dir(details_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let Some((prefix, _)) = name.split_once('-') else {
                continue;
            };
            if let Ok(value) = prefix.parse::<u32>() {
                max_attempt = max_attempt.max(value);
            }
        }
    }
    Ok(max_attempt + 1)
}

fn update_change_doc_review_section(request: &Request) -> Result<()> {
    let path = Path::new(&request.change_path).join("change-doc.md");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path)?;
    let section = render_review_results_section(request);
    fs::write(
        path,
        replace_markdown_section(&content, "## Review 结果", &section),
    )?;
    Ok(())
}

fn render_review_results_section(request: &Request) -> String {
    let mut lines = vec!["## Review 结果".to_string(), String::new()];
    for stage in ["plan-review", "code-review"] {
        let summary_path = Path::new(&request.change_path)
            .join("reviews")
            .join(stage)
            .join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&summary_path).unwrap_or_default();
        let approved = json_bool(&content, "approved").unwrap_or(false);
        let attempt = json_number(&content, "attempt").unwrap_or(0);
        lines.push(format!(
            "### {}",
            if stage == "plan-review" {
                "Plan Review"
            } else {
                "Code Review"
            }
        ));
        lines.push(String::new());
        lines.push(format!(
            "- 最终状态: {}",
            if approved { "approved" } else { "rejected" }
        ));
        lines.push(format!("- 尝试次数: {attempt}"));
        lines.push(format!("- 详情: `reviews/{stage}/summary.json`"));
        for reviewer in ["PlanReviewer", "TestReviewer", "DesignReviewer"] {
            if content.contains(&format!("\"reviewer\": \"{reviewer}\"")) {
                lines.push(format!("- {reviewer}: 已记录"));
            }
        }
        lines.push(String::new());
    }
    if lines.len() == 2 {
        lines.push("尚未产生 review 结果。".to_string());
        lines.push(String::new());
    }
    lines.join("\n")
}

fn replace_markdown_section(content: &str, heading: &str, replacement: &str) -> String {
    let Some(start) = content.find(heading) else {
        return format!("{}\n\n{}\n", content.trim_end(), replacement.trim_end());
    };
    let after_heading = start + heading.len();
    let next_heading = content[after_heading..]
        .find("\n## ")
        .map(|offset| after_heading + offset);
    let end = next_heading.unwrap_or(content.len());
    format!(
        "{}{}{}",
        &content[..start],
        replacement.trim_end(),
        &content[end..]
    )
}

fn reviews_approved(results: &[ReviewResult]) -> bool {
    !results.is_empty()
        && results.iter().all(|result| {
            result.approved && !result.has_blocking_findings && !result.gate_unavailable
        })
}

fn review_gate_unavailable(results: &[ReviewResult]) -> bool {
    results.iter().any(|result| result.gate_unavailable)
}

fn review_gate_unavailable_reason(stage: &str, results: &[ReviewResult]) -> String {
    let diagnostics = results
        .iter()
        .filter(|result| result.gate_unavailable)
        .map(|result| {
            let diagnostic = fallback_empty(&result.diagnostic, "no diagnostic available");
            format!(
                "{}: {} ({diagnostic}); details: {}",
                result.reviewer, result.summary, result.path
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{stage} gate unavailable; {diagnostics}")
}

fn rejected_reviewers(results: &[ReviewResult]) -> Vec<String> {
    results
        .iter()
        .filter(|result| !result.approved || result.has_blocking_findings)
        .map(|result| result.reviewer.clone())
        .collect()
}

fn recommended_next_phase(results: &[ReviewResult], default_phase: &str) -> String {
    if results
        .iter()
        .any(|result| result.gate_unavailable || result.recommended_next_phase == "blocked")
    {
        return "blocked".to_string();
    }
    if results
        .iter()
        .any(|result| result.recommended_next_phase == "planning")
    {
        return "planning".to_string();
    }
    if results
        .iter()
        .any(|result| result.recommended_next_phase == "implementation")
    {
        return "implementation".to_string();
    }
    default_phase.to_string()
}

fn approve_gate_from_review(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    gate: &str,
    by: &str,
    source: &str,
    comment: &str,
) -> Result<()> {
    write_approval_record(request, gate, "approved", by, source, comment)?;
    request.status = format!("{}-approved", gate_status_prefix(gate));
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    let stage = if gate == "plan" {
        "planning"
    } else {
        "implementation"
    };
    write_status_json(request, stage, &request.status, comment)?;
    append_event(
        "gate_approved",
        &request.request_id,
        stage,
        &request.status,
        &format!("gate={gate}; source={source}; by={by}"),
    )?;
    update_gate_session(request, gate, "approved")
}

fn mark_review_rejected(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    phase: &str,
    stage: &str,
    reason: &str,
) -> Result<()> {
    request.status = format!("{stage}-rejected");
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, phase, &request.status, reason)?;
    append_event(
        "review_rejected",
        &request.request_id,
        phase,
        &request.status,
        reason,
    )?;
    upsert_session_for_request(request, phase, "review-rejected")
}

fn start_worktree(args: &[String]) -> Result<()> {
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
            "{} has no change packet. Run: codex-auto-dev plan --name {}-short-name --request_id {}",
            request.request_id,
            today(),
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
    println!("  change doc: {}/change-doc.md", request.change_path);
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
    ensure_gate_approved(&request, "change-doc")?;
    let commit_message = commit_message.unwrap_or_else(|| default_commit_message(&request));
    validate_commit_message(&commit_message)?;
    let delivery = deliver_finished_request(&request, &commit_message)?;
    request.status = "finished".to_string();
    request.updated_at = now_string();
    let change_path = request.change_path.clone();
    let worktree_path = request.worktree_path.clone();
    let branch = request.branch.clone();
    requests[index] = request.clone();
    save_requests(&requests)?;
    upsert_session_for_request(&request, "implementation", "finished")?;

    println!("{request_id} marked finished.");
    println!("  change doc: {change_path}/change-doc.md");
    println!("  worktree: {worktree_path}");
    println!("  branch: {branch}");
    println!("  committed: {}", delivery.commit_message);
    println!("  pushed branch: {}", delivery.branch);
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
    println!("  recovery: {}/recovery.md", request.change_path);
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
        let phase = if ensure_gate_approved(&request, "plan").is_ok() {
            AgentPhase::Implementation
        } else {
            AgentPhase::Planning
        };
        let status = match phase {
            AgentPhase::Planning => "planning",
            AgentPhase::Implementation => "in-progress",
        };
        request.status = status.to_string();
        request.updated_at = now_string();
        requests[index] = request.clone();
        save_requests(&requests)?;
        write_status_json(&request, phase.as_str(), status, "resumed from blocked")?;
        append_event(
            "request_resumed",
            &request.request_id,
            phase.as_str(),
            status,
            "resumed from blocked by user",
        )?;
        upsert_session_for_request(&request, phase.as_str(), status)?;
        Some((phase, status.to_string()))
    } else {
        None
    };
    println!("Resume package for {}", request.request_id);
    println!("  request: {}/request.md", request.change_path);
    println!("  plan: {}/plan.md", request.change_path);
    println!("  change doc: {}/change-doc.md", request.change_path);
    println!("  agent journal: {}/agent-journal.md", request.change_path);
    println!("  status: {}/status.json", request.change_path);
    println!("  recovery: {}/recovery.md", request.change_path);
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
    println!(
        "  next: codex-auto-dev tick --request_id {}",
        request.request_id
    );
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
        notes.push(
            "CodeGraph project preview required before planning: 目标仓库有内容，且 docs/codegraph/context.md 缺失或早于最新提交。请运行 codegraph-project-preview skill 生成或刷新文档。"
                .to_string(),
        );
    } else {
        notes.push(
            "CodeGraph 检查通过: docs/codegraph/context.md 看起来不早于最新提交。".to_string(),
        );
    }

    Ok(PlanPreflight { notes })
}

fn deliver_finished_request(request: &Request, commit_message: &str) -> Result<DeliveryResult> {
    if request.worktree_path.trim().is_empty() {
        return Err(format!(
            "{} has no worktree. Run codex-auto-dev start first.",
            request.request_id
        )
        .into());
    }
    if request.branch.trim().is_empty() {
        return Err(format!(
            "{} has no branch. Run codex-auto-dev start first.",
            request.request_id
        )
        .into());
    }
    let worktree = Path::new(&request.worktree_path);
    if !worktree.exists() {
        return Err(format!("worktree does not exist: {}", worktree.display()).into());
    }
    let changes = git_output(&request.worktree_path, &["status", "--porcelain"])?;
    if changes.trim().is_empty() {
        return Err("no worktree changes to commit".into());
    }

    run_command(
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(worktree),
    )?;
    run_command(
        Command::new("git")
            .args(["commit", "-m", commit_message])
            .current_dir(worktree),
    )?;

    if !remote_exists(&request.worktree_path) {
        return Err("git remote origin is required before finish can push".into());
    }
    run_command(
        Command::new("git")
            .args(["push", "-u", "origin", &request.branch])
            .current_dir(worktree)
            .envs(proxy_env()),
    )?;

    let config = load_config()?;
    let body_file = write_pr_body(request)?;
    let compare_url = github_compare_url(&config.git_url, &config.base_branch, &request.branch);
    let (pr_url, pr_status, pr_error) = run_pr_tool(
        request,
        &config.base_branch,
        &request.branch,
        commit_message,
        &body_file,
        compare_url.as_deref().unwrap_or(""),
    )?;

    Ok(DeliveryResult {
        commit_message: commit_message.to_string(),
        branch: request.branch.clone(),
        pr_url,
        pr_status,
        compare_url,
        pr_error,
    })
}

fn write_pr_body(request: &Request) -> Result<String> {
    let body_path = Path::new(".codex-auto-dev")
        .join("state")
        .join(format!("{}-pr-body.md", request.request_id));
    let change_doc_path = Path::new(&request.change_path).join("change-doc.md");
    let request_path = Path::new(&request.change_path).join("request.md");
    let change_doc = fs::read_to_string(&change_doc_path)?;
    let request_doc = fs::read_to_string(&request_path)?;
    let issue_reference = render_pr_issue_reference(request);
    let review_findings = render_pr_review_findings(request);
    let body = format!(
        "# 关联需求\n\n{issue_reference}\n\n---\n\n{review_findings}\n\n---\n\n# Request\n\n{request_doc}\n\n---\n\n# Change Doc\n\n{change_doc}\n",
    );
    fs::write(&body_path, body)?;
    Ok(absolute_path_string(&body_path))
}

fn render_pr_review_findings(request: &Request) -> String {
    let mut lines = vec![
        "# 自动评审意见".to_string(),
        String::new(),
        "本节由 `codex-auto-dev finish` 从最终 review detail JSON 生成，方便人类在 PR 页面直接查看 reviewer 的 warning/info 以及必要的上下文。".to_string(),
        String::new(),
    ];
    let mut rendered_stage = false;

    for stage in ["plan-review", "code-review"] {
        let summary_path = Path::new(&request.change_path)
            .join("reviews")
            .join(stage)
            .join("summary.json");
        if !summary_path.exists() {
            continue;
        }
        rendered_stage = true;
        let summary = fs::read_to_string(&summary_path).unwrap_or_default();
        let approved = json_bool(&summary, "approved").unwrap_or(false);
        let attempt = json_number(&summary, "attempt").unwrap_or(0);
        lines.push(format!(
            "## {}",
            if stage == "plan-review" {
                "Plan Review"
            } else {
                "Code Review"
            }
        ));
        lines.push(String::new());
        lines.push(format!(
            "- Gate: {}",
            if approved { "approved" } else { "rejected" }
        ));
        lines.push(format!("- Attempt: {attempt}"));
        lines.push(format!("- Summary: `reviews/{stage}/summary.json`"));
        lines.push(String::new());

        let mut rendered_reviewer = false;
        for (reviewer, file_stem) in review_detail_file_stems(stage) {
            let detail_path = Path::new(&request.change_path)
                .join("reviews")
                .join(stage)
                .join("details")
                .join(format!("{attempt:03}-{file_stem}.json"));
            if !detail_path.exists() {
                continue;
            }
            rendered_reviewer = true;
            let detail = fs::read_to_string(&detail_path).unwrap_or_default();
            render_pr_reviewer_detail(request, stage, reviewer, &detail_path, &detail, &mut lines);
        }
        if !rendered_reviewer {
            lines.push("本轮没有找到 reviewer detail JSON。".to_string());
            lines.push(String::new());
        }
    }

    if !rendered_stage {
        lines.push("未找到自动评审结果；如果本次是人工审批，请在 PR 评审时重点核对 change doc 和验证证据。".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

fn review_detail_file_stems(stage: &str) -> Vec<(&'static str, &'static str)> {
    if stage == "plan-review" {
        vec![("PlanReviewer", "plan-reviewer")]
    } else {
        vec![
            ("TestReviewer", "test-reviewer"),
            ("DesignReviewer", "design-reviewer"),
        ]
    }
}

fn render_pr_reviewer_detail(
    request: &Request,
    stage: &str,
    reviewer: &str,
    detail_path: &Path,
    detail: &str,
    lines: &mut Vec<String>,
) {
    let declared_reviewer = json_value(detail, "reviewer").unwrap_or_else(|| reviewer.to_string());
    let approved = json_bool(detail, "approved").unwrap_or(false);
    let decision = json_value(detail, "decision").unwrap_or_else(|| {
        if approved {
            "approved".to_string()
        } else {
            "rejected".to_string()
        }
    });
    let recommended_next_phase =
        json_value(detail, "recommended_next_phase").unwrap_or_else(|| "unknown".to_string());
    let summary = json_value(detail, "summary").unwrap_or_else(|| "no summary".to_string());
    lines.push(format!("### {declared_reviewer}"));
    lines.push(String::new());
    lines.push(format!("- Decision: {decision}"));
    lines.push(format!(
        "- Recommended next phase: {recommended_next_phase}"
    ));
    lines.push(format!("- Summary: {}", markdown_inline(&summary)));
    lines.push(format!(
        "- Detail: `{}`",
        review_detail_relative_path(request, stage, detail_path)
    ));
    lines.push(String::new());

    let mut finding_count = 0usize;
    for severity in ["critical", "high", "warning", "info"] {
        let findings = review_findings(detail, severity);
        if findings.is_empty() {
            continue;
        }
        finding_count += findings.len();
        lines.push(format!("#### {severity}"));
        lines.push(String::new());
        for (index, finding) in findings.iter().enumerate() {
            lines.push(format!(
                "{}. **{}**",
                index + 1,
                markdown_inline(&finding.title)
            ));
            lines.push(format!(
                "   - Evidence: {}",
                markdown_inline(&finding.evidence)
            ));
            lines.push(format!("   - Impact: {}", markdown_inline(&finding.impact)));
            lines.push(format!(
                "   - Required fix: {}",
                markdown_inline(&finding.required_fix)
            ));
            lines.push(format!(
                "   - Suggested change: {}",
                markdown_inline(&finding.suggested_change)
            ));
            lines.push(format!(
                "   - Verification: {}",
                markdown_inline(&finding.verification)
            ));
        }
        lines.push(String::new());
    }

    if finding_count == 0 {
        lines.push("- Findings: 无。".to_string());
        lines.push(String::new());
    }
}

fn review_detail_relative_path(request: &Request, stage: &str, detail_path: &Path) -> String {
    let fallback = detail_path.to_string_lossy().to_string();
    let Ok(relative_to_change) = detail_path.strip_prefix(&request.change_path) else {
        return fallback;
    };
    let relative = relative_to_change.to_string_lossy();
    if relative.is_empty() {
        format!("reviews/{stage}/details")
    } else {
        relative.to_string()
    }
}

fn run_pr_tool(
    request: &Request,
    base_branch: &str,
    head_branch: &str,
    title: &str,
    body_file: &str,
    compare_url: &str,
) -> Result<(Option<String>, String, String)> {
    if !Path::new(PR_TOOL).exists() {
        return Ok((
            None,
            "skipped".to_string(),
            format!("{PR_TOOL} does not exist"),
        ));
    }
    let output = Command::new("sh")
        .arg(PR_TOOL)
        .current_dir(".")
        .env("CODEX_AUTO_DEV_REQUEST_ID", &request.request_id)
        .env("CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("CODEX_AUTO_DEV_REQUEST_SOURCE", &request.source)
        .env("CODEX_AUTO_DEV_REQUEST_TITLE", &request.title)
        .env("CODEX_AUTO_DEV_REQUEST_URL", &request.url)
        .env("CODEX_AUTO_DEV_CHANGE_PATH", &request.change_path)
        .env(
            "CODEX_AUTO_DEV_CHANGE_DOC",
            format!("{}/change-doc.md", request.change_path),
        )
        .env(
            "CODEX_AUTO_DEV_REQUEST",
            format!("{}/request.md", request.change_path),
        )
        .env("CODEX_AUTO_DEV_WORKTREE", &request.worktree_path)
        .env("CODEX_AUTO_DEV_PR_TITLE", title)
        .env("CODEX_AUTO_DEV_PR_BODY_FILE", body_file)
        .env("CODEX_AUTO_DEV_PR_BASE", base_branch)
        .env("CODEX_AUTO_DEV_PR_HEAD", head_branch)
        .env("CODEX_AUTO_DEV_PR_COMPARE_URL", compare_url)
        .envs(proxy_env())
        .output();
    let Ok(output) = output else {
        return Ok((
            None,
            "failed".to_string(),
            format!("{PR_TOOL} could not be executed"),
        ));
    };
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)?;
        match parse_pr_tool_success(&stdout) {
            Ok((status, url)) => Ok((Some(url), status, String::new())),
            Err(error) => Ok((None, "failed".to_string(), error.to_string())),
        }
    } else {
        Ok((
            None,
            "failed".to_string(),
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn parse_pr_tool_success(stdout: &str) -> Result<(String, String)> {
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| format!("{PR_TOOL} succeeded without returning a PR URL"))?;
    if let Some((status, url)) = line.split_once('\t') {
        let status = status.trim();
        let url = url.trim();
        if !matches!(status, "created" | "existing") {
            return Err(format!(
                "{PR_TOOL} returned unknown PR status: {status}. Expected created<TAB>url or existing<TAB>url"
            )
            .into());
        }
        if url.is_empty() {
            return Err(format!("{PR_TOOL} returned {status} without a PR URL").into());
        }
        return Ok((status.to_string(), url.to_string()));
    }
    Ok(("created".to_string(), line.to_string()))
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

fn upgrade_workspace(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--dry-run", "--default"])?;
    let dry_run = flag_present(args, "--dry-run");
    let install_defaults = flag_present(args, "--default");
    let config = load_config()?;
    let requests = load_requests()?;

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
        println!("Refreshed framework reference examples");
        if install_defaults {
            replace_default_runtime_assets_from_examples()?;
            println!("Replaced default runtime assets from refreshed examples");
        } else {
            print_upgrade_default_asset_guidance();
        }
    }

    for request in &requests {
        if request.change_path.is_empty() {
            continue;
        }
        let approvals_dir = Path::new(&request.change_path).join("approvals");
        if !approvals_dir.exists() {
            if dry_run {
                println!("Would create {}", approvals_dir.display());
            } else {
                fs::create_dir_all(&approvals_dir)?;
                println!("Created {}", approvals_dir.display());
            }
        }

        upgrade_change_artifacts(request, dry_run)?;

        if !dry_run {
            upsert_session_for_request(request, "planning", "handoff-ready")?;
            if !request.worktree_path.is_empty() {
                upsert_session_for_request(request, "implementation", "handoff-ready")?;
            }
        }
    }

    if !dry_run {
        ensure_sessions_file()?;
    }
    println!("Upgrade complete.");
    Ok(())
}

fn list_requests() -> Result<()> {
    ensure_initialized()?;
    sync_all_requests_from_status_json()?;
    let requests = load_requests()?;
    if requests.is_empty() {
        println!("No requests yet. Run: codex-auto-dev update");
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
        for file in [
            "request.md",
            "plan.md",
            "change-doc.md",
            "agent-journal.md",
            "status.json",
        ] {
            let path = Path::new(&request.change_path).join(file);
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

fn prepare_workspace_dirs() -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
    fs::create_dir_all("dev")?;
    fs::create_dir_all(WORKTREES)?;
    fs::create_dir_all("docs/changes")?;
    fs::create_dir_all("tools")?;
    fs::create_dir_all("skills/codex-auto-dev-workflow")?;
    Ok(())
}

fn write_config(repo_name: &str, git_url: &str, base_branch: &str) -> Result<()> {
    if Path::new(CONFIG_PATH).exists() {
        return Ok(());
    }
    let config = Config {
        schema_version: FRAMEWORK_SCHEMA_VERSION,
        repo_name: repo_name.to_string(),
        git_url: git_url.to_string(),
        base_branch: base_branch.to_string(),
        parallel_limit: 1,
    };
    rewrite_config(&config)
}

fn rewrite_config(config: &Config) -> Result<()> {
    fs::write(
        CONFIG_PATH,
        format!(
            "schema_version = {}\nrepo_name = \"{}\"\ngit_url = \"{}\"\nbase_branch = \"{}\"\nparallel_limit = {}\n",
            FRAMEWORK_SCHEMA_VERSION,
            toml_escape(&config.repo_name),
            toml_escape(&config.git_url),
            toml_escape(&config.base_branch),
            config.parallel_limit
        ),
    )?;
    Ok(())
}

fn ensure_state_file() -> Result<()> {
    if !Path::new(STATE_PATH).exists() {
        save_requests(&[])?;
    }
    Ok(())
}

fn ensure_sessions_file() -> Result<()> {
    if !Path::new(SESSIONS_PATH).exists() {
        save_sessions(&[])?;
    }
    Ok(())
}

fn generate_plan_packet(request: &Request, preflight: &PlanPreflight) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    fs::create_dir_all(Path::new(&request.change_path).join("approvals"))?;
    fs::write(
        Path::new(&request.change_path).join("request.md"),
        render_request(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("plan.md"),
        render_plan_template(request, preflight),
    )?;
    fs::write(
        Path::new(&request.change_path).join("change-doc.md"),
        render_change_doc_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("agent-journal.md"),
        render_agent_journal_template(request),
    )?;
    write_status_json(request, "planning", "planning", "")?;
    Ok(())
}

fn generate_start_packet(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    write_status_json(request, "implementation", "in-progress", "")?;
    Ok(())
}

fn render_request(request: &Request) -> String {
    format!(
        "# 需求 {request_id}: {title}\n\n- Request ID: `{request_id}`\n- External ID: `{external_id}`\n- Source: `{source}`\n- URL: {url}\n\n## 需求标题\n\n{title}\n\n## 需求描述\n\n{body}\n\n## 说明\n\n标题和描述都必须作为计划阶段的需求来源。标题只能用于概览，不能替代完整需求描述。如果原始需求来自对话而不是外部平台，Codex 必须在计划阶段把用户的完整需求补写到这里，并在 `plan.md` 中给出可验证计划。\n",
        request_id = request.request_id,
        title = request.title,
        external_id = request.external_id,
        source = request.source,
        url = fallback_empty(&request.url, "n/a"),
        body = fallback_empty(
            &request.body,
            "Codex 必须从用户对话或外部需求来源补充完整需求。"
        ),
    )
}

fn render_plan_template(request: &Request, preflight: &PlanPreflight) -> String {
    format!(
        "# 计划: {title}\n\n## 规范化需求记录\n\n- Request ID: `{request_id}`\n- External ID: `{external_id}`\n- Source: `{source}`\n- URL: {url}\n\n### 需求名称\n\n{title}\n\n### 需求描述\n\n{body}\n\n## 模板说明\n\n这是计划模板。Codex 或 planning agent 必须填写真实计划；`codex-auto-dev` 只创建必要文档包，不生成实际开发计划。agent 可以重写本文件，但必须保留并更新上面的规范化需求记录。\n\n## 需求理解\n\n读取 `request.md` 的需求标题和需求描述，补齐原始需求中的缺失上下文。标题和描述都必须作为需求来源，标题不能替代完整需求描述。\n\n## 计划前检查\n\n{preflight_notes}\n\n## 目标与依赖顺序\n\n列出要完成的目标、目标之间的依赖关系、必须先完成的前置条件，以及每个目标的完成信号。\n\n## 仓库分析\n\n列出已经阅读的文件、模块、现有模式、目标项目文档和 CodeGraph 文档。说明本次改动为什么应该落在这些位置。\n\n## 目标项目内部要求\n\n列出目标项目自己的 change doc、pre-commit、文档检查、format/lint/test 命令、AI review、安全规则、敏感信息规则和禁止 panic/硬编码等要求。\n\n## 实现计划\n\n列出预计修改或新增的文件、模块、函数、结构体、命令、配置和迁移方式。说明是否包含破坏性改动，如何兼容旧数据。\n\n## 测试与验证\n\n列出单元测试、集成测试、失败路径测试、回归测试、安全检查、pre-commit、文档检查、AI review 和人工验证步骤。每条验证都要说明命令或证据。\n\n## 执行任务清单\n\n- [ ] 阅读 `request.md` 和目标项目文档。\n- [ ] 填写本计划，覆盖目标、依赖、实现位置、测试策略和风险。\n- [ ] 等待 wrapper hook 调用外层 `codex-auto-dev advance` 提交 plan gate 并运行 PlanReviewer。\n- [ ] PlanReviewer 拒绝时，读取 `reviews/plan-review/summary.json` 和最新 detail，修复计划后再次交给外层 advance/tick。\n- [ ] 计划审批通过后，外层 advance/tick 会创建独立 worktree 并派发 implementation agent。\n- [ ] implementation 只能在生成的 worktree 中实现，不直接编辑 `dev/repo`。\n- [ ] 填写 `change-doc.md` 后等待 wrapper hook 调用外层 advance 提交 change-doc gate 并运行 TestReviewer 和 DesignReviewer。\n\n## 审批门禁\n\nplan approval 通过前不得 start。change-doc approval 通过前不得 finish、commit、push、创建 PR 或 merge。\n",
        title = request.title,
        request_id = request.request_id,
        external_id = request.external_id,
        source = request.source,
        url = fallback_empty(&request.url, "n/a"),
        body = fallback_empty(
            &request.body,
            "Codex 必须从用户对话或外部需求来源补充完整需求。"
        ),
        preflight_notes = render_preflight_notes(preflight),
    )
}

fn render_change_doc_template(request: &Request) -> String {
    format!(
        "# 变更文档: {request_id}\n\n这是变更文档模板。Codex 必须在实现完成后、请求审批前填写真实内容。本文档的重点是解释需求如何被实现，而不是完整罗列所有文件变更。\n\n## 摘要\n\n用几句话说明实际完成了什么、用户可见变化是什么、是否偏离已批准计划，以及是否存在剩余风险。\n\n## 实现前后对比\n\n- 实现前: 描述原有流程、缺失能力、失败模式或用户痛点。\n- 实现后: 描述新流程、新能力、用户如何观察到变化，以及哪些行为保持兼容。\n\n## 关键设计点\n\n按关键点分别说明设计与实现方式。每个关键点应包含: 为什么这样设计、核心数据/命令/流程是什么、如何满足原始需求、边界和取舍是什么。\n\n## 变更范围摘要\n\n用总结性的方式列出主要改动区域，例如 CLI 命令、状态文件、模板、测试、文档或迁移逻辑。只列关键文件或模块，不需要完整文件清单。\n\n## 目标项目内部要求\n\n- 已阅读的目标项目文档: 填写文档路径。\n- 目标项目 change doc: 填写路径或 `Not required`，并说明原因。\n- Pre-commit: 填写命令和结果，或 `Not required`。\n- 文档检查: 填写命令和结果，或 `Not required`。\n- Format/lint/test: 填写命令和结果。\n- AI review: 填写发现、处理状态，或 `Not required`。\n- 所有目标项目内部要求是否完成: 填写 yes/no 和阻塞项。\n\n## 文档与 Checklist\n\n- 已更新的文档: 填写路径和摘要；如果没有目标项目文档需要更新，填写 `Not required` 并说明原因。\n- 所有交付文档中的 checklist 是否已全部打勾: 填写 yes/no，并列出检查过的文档路径。\n- 未完成事项是否已移出 checklist 并记录到后续流程、人工事项或阻塞项: 填写 yes/no。\n- 已批准 plan 中的历史 checklist 不要为了凑勾而篡改；如果执行结果与 plan checklist 不一致，在本 change-doc 解释。\n\n## 后续流程\n\n记录当前自动流程无法完成但仍需追踪的人工审批、外部发布、账号权限、跨团队确认或后续版本事项。不要把这些事项保留为未勾选 checklist。\n\n## 验证证据\n\n填写准确命令、输出摘要、失败修复过程和人工验证证据。日志、错误、commit hash、测试输出保持原文。\n\n## Review 结果\n\n尚未产生 review 结果。\n\n## 审批门禁\n\n填写完成后等待 wrapper hook 调用外层 `codex-auto-dev advance` 提交 change-doc gate 并运行 code-review。审批通过前不得运行 `codex-auto-dev finish --request_id {request_id}`，也不得 commit、push、创建 PR 或 merge。\n",
        request_id = request.request_id,
    )
}

fn render_agent_journal_template(request: &Request) -> String {
    format!(
        "# Agent Journal: {request_id}\n\n这个文件用于避免上下文过长后无法恢复。agent 每轮都必须追加记录: 当前阶段、读取的文件、review 发现、修改内容、运行命令、剩余风险和下一步。\n",
        request_id = request.request_id,
    )
}

fn write_executable_file(path: &str, content: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, content)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn write_default_issue_tool() -> Result<()> {
    if Path::new(ISSUE_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(ISSUE_TOOL, default_issue_tool_content())
}

fn default_issue_tool_content() -> &'static str {
    r##"#!/usr/bin/env sh
set -eu

cd dev/repo

# Output TSV lines:
# external_id<TAB>source<TAB>title<TAB>body<TAB>url
#
# Connector contract:
# - stdout MUST contain zero or more TSV records and no header.
# - Column 1 external_id MUST be stable for the same upstream request.
# - Column 2 source SHOULD be a short provider name such as github, jira, linear, or internal.
# - Column 3 title becomes the normalized requirement name.
# - Column 4 body becomes the normalized requirement description; preserve full user-visible detail.
# - Column 5 url MAY be empty when the source has no browser URL.
# - stderr is reserved for diagnostics. Exit non-zero only when update failed.
#
# Replace this script for Jira, Linear, internal workspaces, or other sources.
# The connector should emit a stable external_id so repeated updates do not
# create duplicate requests.

repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
gh api --method GET "repos/${repo}/issues" \
  -f state=open \
  --paginate \
  --jq ".[] | select(.pull_request == null) | [\"github:${repo}#\" + (.number|tostring), \"github\", .title, (.body // \"\"), .html_url] | @tsv"
"##
}

fn write_default_issue_agent_tool() -> Result<()> {
    fs::create_dir_all("tools/prompts")?;
    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        write_executable_file(ISSUE_AGENT_TOOL, default_issue_agent_tool_content())?;
    }
    if !Path::new(ISSUE_AGENT_PROMPT).exists() {
        fs::write(ISSUE_AGENT_PROMPT, default_issue_agent_prompt())?;
    }
    if !Path::new(PLAN_AGENT_PROMPT).exists() {
        fs::write(PLAN_AGENT_PROMPT, default_plan_agent_prompt())?;
    }
    if !Path::new(IMPLEMENTATION_AGENT_PROMPT).exists() {
        fs::write(
            IMPLEMENTATION_AGENT_PROMPT,
            default_implementation_agent_prompt(),
        )?;
    }
    Ok(())
}

fn codex_bin_resolver_shell() -> &'static str {
    r##"resolve_codex_bin() {
  if [ -n "${CODEX_AUTO_DEV_CODEX_BIN:-}" ]; then
    if [ -x "$CODEX_AUTO_DEV_CODEX_BIN" ]; then
      printf '%s\n' "$CODEX_AUTO_DEV_CODEX_BIN"
      return 0
    fi
    if command -v "$CODEX_AUTO_DEV_CODEX_BIN" >/dev/null 2>&1; then
      command -v "$CODEX_AUTO_DEV_CODEX_BIN"
      return 0
    fi
    echo "CODEX_AUTO_DEV_CODEX_BIN is set but is not executable and was not found on PATH: $CODEX_AUTO_DEV_CODEX_BIN" >&2
    return 1
  fi

  if command -v codex >/dev/null 2>&1; then
    command -v codex
    return 0
  fi

  if [ -n "${CODEX_AUTO_DEV_CODEX_APP:-}" ]; then
    for candidate in \
      "$CODEX_AUTO_DEV_CODEX_APP/Contents/Resources/codex" \
      "$CODEX_AUTO_DEV_CODEX_APP/Contents/MacOS/codex"
    do
      if [ -x "$candidate" ]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done
    echo "CODEX_AUTO_DEV_CODEX_APP is set but no codex binary was found inside it: $CODEX_AUTO_DEV_CODEX_APP" >&2
    return 1
  fi

  echo "codex CLI is unavailable; add codex to PATH, set CODEX_AUTO_DEV_CODEX_BIN, or set CODEX_AUTO_DEV_CODEX_APP to the Codex app bundle" >&2
  return 1
}
"##
}

fn default_issue_agent_tool_content() -> String {
    let mut content = String::from(
        r##"#!/usr/bin/env sh
set -eu
trap '' HUP

# Replace this script to use Claude Code, OpenAI API, an internal agent,
# or any other implementation backend. The script processes exactly one
# request phase: planning or implementation. The outer codex-auto-dev advance/tick
# owns submit, reviewer gates, start, finish, commit, push, PR creation,
# and phase transitions.
#
# Connector contract:
# - Inputs are provided through CODEX_AUTO_DEV_* environment variables.
# - CODEX_AUTO_DEV_AGENT_PHASE is planning or implementation.
# - The agent MUST read CODEX_AUTO_DEV_REQUEST, CODEX_AUTO_DEV_PLAN, and CODEX_AUTO_DEV_AGENT_JOURNAL.
# - planning agents MUST write a reviewable plan.md and then exit.
# - implementation agents MUST work only inside CODEX_AUTO_DEV_WORKTREE, update change-doc.md, and then exit.
# - The agent MUST NOT call codex-auto-dev submit/plan-review/code-review/start/finish.
# - The agent MUST NOT call codex-auto-dev approve/reject or edit approval JSON.
# - If a review summary has gate_unavailable=true, the agent MUST block instead of retrying or bypassing.
# - The agent MUST append recovery-oriented notes to CODEX_AUTO_DEV_AGENT_JOURNAL.
# - Success means the phase artifact is ready for the outer advance/tick review gate.
# - Failure should exit non-zero with a helpful stderr message.

"##,
    );
    content.push_str(codex_bin_resolver_shell());
    content.push_str(
        r##"

workspace="${CODEX_AUTO_DEV_WORKSPACE:-$(pwd)}"
phase="${CODEX_AUTO_DEV_AGENT_PHASE:-planning}"
shared_prompt="${CODEX_AUTO_DEV_ISSUE_AGENT_SHARED_PROMPT:-tools/prompts/issue-agent.md}"
case "$phase" in
  planning) default_prompt="tools/prompts/plan-agent.md" ;;
  implementation) default_prompt="tools/prompts/implementation-agent.md" ;;
  *)
    echo "unsupported CODEX_AUTO_DEV_AGENT_PHASE: $phase" >&2
    exit 1
    ;;
esac
prompt="${CODEX_AUTO_DEV_ISSUE_AGENT_PROMPT:-$default_prompt}"

if ! codex_bin="$(resolve_codex_bin)"; then
  echo "replace tools/issue-agent.sh with another agent backend if Codex CLI is not available" >&2
  exit 1
fi
if [ ! -f "$prompt" ]; then
  echo "agent prompt does not exist: $prompt" >&2
  exit 1
fi

{
  printf 'Workspace: %s\n' "$workspace"
  printf 'Request ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_ID:-}"
  printf 'Agent phase: %s\n' "$phase"
  printf 'External ID: %s\n' "${CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID:-}"
  printf 'Source: %s\n' "${CODEX_AUTO_DEV_REQUEST_SOURCE:-}"
  printf 'Requirement name: %s\n' "${CODEX_AUTO_DEV_REQUEST_TITLE:-}"
  printf 'Max attempts: %s\n' "${CODEX_AUTO_DEV_MAX_ATTEMPTS:-20}"
  printf 'Request document: %s\n' "${CODEX_AUTO_DEV_REQUEST:-}"
  printf 'Plan: %s\n' "${CODEX_AUTO_DEV_PLAN:-}"
  printf 'Change doc: %s\n' "${CODEX_AUTO_DEV_CHANGE_DOC:-}"
  printf 'Agent journal: %s\n' "${CODEX_AUTO_DEV_AGENT_JOURNAL:-}"
  printf 'Worktree: %s\n\n' "${CODEX_AUTO_DEV_WORKTREE:-}"
  if [ -f "$shared_prompt" ]; then
    cat "$shared_prompt"
    printf '\n\n'
  fi
  cat "$prompt"
} | nohup "$codex_bin" exec \
  --cd "$workspace" \
  --skip-git-repo-check \
  -c 'approval_policy="never"' \
  -c 'shell_environment_policy.inherit="all"' \
  --sandbox workspace-write \
  -
"##,
    );
    content
}

fn write_default_pr_tool() -> Result<()> {
    if Path::new(PR_TOOL).exists() {
        return Ok(());
    }
    write_executable_file(PR_TOOL, default_pr_tool_content())
}

fn default_pr_tool_content() -> &'static str {
    r##"#!/usr/bin/env sh
set -eu

# Replace this script for GitLab, Gerrit, Bitbucket, internal workspaces,
# or any other code review system.
#
# Input is provided through environment variables:
# CODEX_AUTO_DEV_REQUEST_ID
# CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID
# CODEX_AUTO_DEV_REQUEST_SOURCE
# CODEX_AUTO_DEV_REQUEST_TITLE
# CODEX_AUTO_DEV_REQUEST_URL
# CODEX_AUTO_DEV_CHANGE_PATH
# CODEX_AUTO_DEV_CHANGE_DOC
# CODEX_AUTO_DEV_REQUEST
# CODEX_AUTO_DEV_WORKTREE
# CODEX_AUTO_DEV_PR_TITLE
# CODEX_AUTO_DEV_PR_BODY_FILE
# CODEX_AUTO_DEV_PR_BASE
# CODEX_AUTO_DEV_PR_HEAD
# CODEX_AUTO_DEV_PR_COMPARE_URL
#
# Connector contract:
# - The worktree has already been committed and pushed before this script runs.
# - Before creating anything, determine whether this platform/repository can create PRs.
# - Before creating anything, check whether a PR for base/head already exists.
# - Print exactly one TSV line to stdout on success:
#   created<TAB>url
#   existing<TAB>url
# - Exit non-zero with a helpful stderr message when the platform cannot create a PR
#   or when an existing PR check cannot be performed safely.
# - Do not merge.

cd "${CODEX_AUTO_DEV_WORKTREE}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh is not installed; create the PR manually: ${CODEX_AUTO_DEV_PR_COMPARE_URL}" >&2
  exit 1
fi

if ! gh repo view >/dev/null 2>&1; then
  echo "gh cannot access this repository or this is not a GitHub repository; create the PR manually: ${CODEX_AUTO_DEV_PR_COMPARE_URL}" >&2
  exit 1
fi

existing_url="$(
  gh pr list \
    --state all \
    --base "${CODEX_AUTO_DEV_PR_BASE}" \
    --head "${CODEX_AUTO_DEV_PR_HEAD}" \
    --json url \
    --jq '.[0].url // ""'
)"
if [ -n "$existing_url" ]; then
  printf 'existing\t%s\n' "$existing_url"
  exit 0
fi

created_url="$(gh pr create \
  --base "${CODEX_AUTO_DEV_PR_BASE}" \
  --head "${CODEX_AUTO_DEV_PR_HEAD}" \
  --title "${CODEX_AUTO_DEV_PR_TITLE}" \
  --body-file "${CODEX_AUTO_DEV_PR_BODY_FILE}")"

if [ -z "$created_url" ]; then
  echo "gh pr create succeeded without returning a PR URL" >&2
  exit 1
fi

printf 'created\t%s\n' "$created_url"
"##
}

fn write_default_review_tools() -> Result<()> {
    fs::create_dir_all("tools/prompts")?;
    fs::create_dir_all("tools/schemas")?;
    write_default_plan_review_tool()?;
    write_default_test_review_tool()?;
    write_default_design_review_tool()?;
    write_default_review_prompt(PLAN_REVIEW_PROMPT, default_plan_review_prompt())?;
    write_default_review_prompt(TEST_REVIEW_PROMPT, default_test_review_prompt())?;
    write_default_review_prompt(DESIGN_REVIEW_PROMPT, default_design_review_prompt())?;
    write_default_review_schema()?;
    Ok(())
}

fn write_default_plan_review_tool() -> Result<()> {
    write_default_review_tool(
        PLAN_REVIEW_TOOL,
        "PlanReviewer",
        PLAN_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_test_review_tool() -> Result<()> {
    write_default_review_tool(
        TEST_REVIEW_TOOL,
        "TestReviewer",
        TEST_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_design_review_tool() -> Result<()> {
    write_default_review_tool(
        DESIGN_REVIEW_TOOL,
        "DesignReviewer",
        DESIGN_REVIEW_PROMPT,
        "workspace-write",
    )
}

fn write_default_review_tool(
    path: &str,
    reviewer: &str,
    prompt_path: &str,
    sandbox: &str,
) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }
    write_executable_file(
        path,
        default_review_tool_content(reviewer, prompt_path, sandbox),
    )
}

fn default_review_tool_content(reviewer: &str, prompt_path: &str, sandbox: &str) -> String {
    format!(
        r##"#!/usr/bin/env sh
set -eu

# Replace this script to use Claude Code, OpenAI API, an internal reviewer,
# or any other model backend. The script must print exactly one JSON object
# matching tools/schemas/review-result.schema.json to stdout.
#
# Connector contract:
# - Inputs are provided through CODEX_AUTO_DEV_* environment variables.
# - stdout MUST be exactly one JSON object matching tools/schemas/review-result.schema.json.
# - Optional gate_unavailable=true means the reviewer backend/gate cannot make a valid decision.
# - stderr is reserved for diagnostics.
# - Any invalid JSON, empty output, or tool failure becomes a blocking review result.
# - The reviewer MUST NOT modify code or documents.
# - Reviewers receive CODEX_AUTO_DEV_REVIEW_CONTEXT as an isolated copy of request/plan/change-doc/status/approvals.
# - Reviewers MUST NOT read CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS, previous review summaries, or other reviewers' details.

workspace="${{CODEX_AUTO_DEV_WORKSPACE:-$(pwd)}}"
prompt="${{CODEX_AUTO_DEV_REVIEW_PROMPT:-{prompt_path}}}"
schema="${{CODEX_AUTO_DEV_REVIEW_SCHEMA:-tools/schemas/review-result.schema.json}}"

{codex_resolver}

if ! codex_bin="$(resolve_codex_bin)"; then
  printf '{{"reviewer":"{reviewer}","approved":false,"gate_unavailable":true,"decision":"rejected","recommended_next_phase":"blocked","summary":"codex CLI is not available","process":["checked reviewer backend"],"critical":[{{"title":"missing reviewer backend","evidence":"codex command is unavailable; CODEX_AUTO_DEV_CODEX_BIN and CODEX_AUTO_DEV_CODEX_APP did not resolve an executable backend","impact":"review gate cannot evaluate the artifact without a reviewer backend","required_fix":"Install Codex CLI, add it to PATH, set CODEX_AUTO_DEV_CODEX_BIN, set CODEX_AUTO_DEV_CODEX_APP, or replace this reviewer script with another backend.","suggested_change":"Use a wrapper script path in CODEX_AUTO_DEV_CODEX_BIN or point CODEX_AUTO_DEV_CODEX_APP to the Codex app bundle; do not hardcode a machine-specific application path in this connector.","verification":"Run the same review command and confirm stdout is one valid JSON object with gate_unavailable=false."}}],"high":[],"warning":[],"info":[]}}\n'
  exit 0
fi

source_codex_home="${{CODEX_HOME:-}}"
if [ -z "$source_codex_home" ] && [ -n "${{HOME:-}}" ]; then
  source_codex_home="${{HOME}}/.codex"
fi

cleanup_review_codex_home=""
if [ -n "${{CODEX_AUTO_DEV_REVIEW_CODEX_HOME:-}}" ]; then
  review_codex_home="$CODEX_AUTO_DEV_REVIEW_CODEX_HOME"
else
  review_tmp="${{TMPDIR:-/tmp}}"
  review_codex_home="$(mktemp -d "$review_tmp/codex-auto-dev-review-home.XXXXXX")"
  cleanup_review_codex_home="$review_codex_home"
  if [ -n "$source_codex_home" ]; then
    [ -f "$source_codex_home/auth.json" ] && cp "$source_codex_home/auth.json" "$review_codex_home/auth.json"
    [ -f "$source_codex_home/config.toml" ] && cp "$source_codex_home/config.toml" "$review_codex_home/config.toml"
  fi
  chmod 700 "$review_codex_home"
  [ -f "$review_codex_home/auth.json" ] && chmod 600 "$review_codex_home/auth.json"
  [ -f "$review_codex_home/config.toml" ] && chmod 600 "$review_codex_home/config.toml"
fi

cleanup_review_home() {{
  if [ -n "$cleanup_review_codex_home" ]; then
    rm -rf "$cleanup_review_codex_home"
  fi
}}
trap cleanup_review_home EXIT

{{
  printf 'Reviewer: {reviewer}\n'
  printf 'Workspace: %s\n' "$workspace"
  printf 'Request ID: %s\n' "${{CODEX_AUTO_DEV_REQUEST_ID:-}}"
  printf 'External ID: %s\n' "${{CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID:-}}"
  printf 'Requirement name: %s\n' "${{CODEX_AUTO_DEV_REQUEST_TITLE:-}}"
  printf 'Change path: %s\n' "${{CODEX_AUTO_DEV_CHANGE_PATH:-}}"
  printf 'Review context: %s\n' "${{CODEX_AUTO_DEV_REVIEW_CONTEXT:-}}"
  printf 'Canonical change path: %s\n' "${{CODEX_AUTO_DEV_CANONICAL_CHANGE_PATH:-}}"
  printf 'Forbidden review paths: %s\n' "${{CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS:-}}"
  printf 'Target repo: %s\n' "${{CODEX_AUTO_DEV_TARGET_REPO:-}}"
  printf 'Worktree: %s\n\n' "${{CODEX_AUTO_DEV_WORKTREE:-}}"
  cat "$prompt"
}} | CODEX_HOME="$review_codex_home" "$codex_bin" exec \
  --cd "$workspace" \
  --skip-git-repo-check \
  --ephemeral \
  -c 'approval_policy="never"' \
  --sandbox {sandbox} \
  --output-schema "$schema" \
  -
"##,
        prompt_path = prompt_path,
        reviewer = reviewer,
        sandbox = sandbox,
        codex_resolver = codex_bin_resolver_shell(),
    )
}

fn write_default_review_prompt(path: &str, content: &str) -> Result<()> {
    if Path::new(path).exists() {
        return Ok(());
    }
    fs::write(path, content)?;
    Ok(())
}

fn write_default_review_schema() -> Result<()> {
    if Path::new(REVIEW_SCHEMA).exists() {
        return Ok(());
    }
    fs::write(REVIEW_SCHEMA, default_review_schema_content())?;
    Ok(())
}

fn default_review_schema_content() -> &'static str {
    r##"{
  "type": "object",
  "required": [
    "reviewer",
    "approved",
    "gate_unavailable",
    "decision",
    "recommended_next_phase",
    "summary",
    "process",
    "critical",
    "high",
    "warning",
    "info"
  ],
  "additionalProperties": false,
  "properties": {
    "reviewer": { "type": "string" },
    "approved": { "type": "boolean" },
    "gate_unavailable": { "type": "boolean" },
    "decision": { "type": "string", "enum": ["approved", "rejected"] },
    "recommended_next_phase": { "type": "string", "enum": ["planning", "implementation", "blocked"] },
    "summary": { "type": "string" },
    "process": { "type": "array", "items": { "type": "string" } },
    "critical": { "type": "array", "items": { "$ref": "#/$defs/finding" } },
    "high": { "type": "array", "items": { "$ref": "#/$defs/finding" } },
    "warning": { "type": "array", "items": { "$ref": "#/$defs/finding" } },
    "info": { "type": "array", "items": { "$ref": "#/$defs/finding" } }
  },
  "$defs": {
    "finding": {
      "type": "object",
      "required": ["title", "evidence", "impact", "required_fix", "suggested_change", "verification"],
      "additionalProperties": false,
      "properties": {
        "title": { "type": "string" },
        "evidence": { "type": "string" },
        "impact": { "type": "string" },
        "required_fix": { "type": "string" },
        "suggested_change": { "type": "string" },
        "verification": { "type": "string" }
      }
    }
  }
}
"##
}

struct DefaultManagedAsset {
    path: &'static str,
    example_path: &'static str,
    content: String,
    executable: bool,
}

struct ReferenceExample {
    path: &'static str,
    content: String,
    executable: bool,
}

fn default_reference_example_paths() -> Vec<&'static str> {
    default_managed_assets()
        .iter()
        .map(|asset| asset.example_path)
        .collect()
}

fn default_managed_assets() -> Vec<DefaultManagedAsset> {
    vec![
        DefaultManagedAsset {
            path: ISSUE_TOOL,
            example_path: ISSUE_TOOL_EXAMPLE,
            content: default_issue_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: ISSUE_AGENT_TOOL,
            example_path: ISSUE_AGENT_TOOL_EXAMPLE,
            content: default_issue_agent_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: PR_TOOL,
            example_path: PR_TOOL_EXAMPLE,
            content: default_pr_tool_content().to_string(),
            executable: true,
        },
        DefaultManagedAsset {
            path: PLAN_REVIEW_TOOL,
            example_path: PLAN_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "PlanReviewer",
                PLAN_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: TEST_REVIEW_TOOL,
            example_path: TEST_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "TestReviewer",
                TEST_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: DESIGN_REVIEW_TOOL,
            example_path: DESIGN_REVIEW_TOOL_EXAMPLE,
            content: default_review_tool_content(
                "DesignReviewer",
                DESIGN_REVIEW_PROMPT,
                "workspace-write",
            ),
            executable: true,
        },
        DefaultManagedAsset {
            path: ISSUE_AGENT_PROMPT,
            example_path: ISSUE_AGENT_PROMPT_EXAMPLE,
            content: default_issue_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: PLAN_AGENT_PROMPT,
            example_path: PLAN_AGENT_PROMPT_EXAMPLE,
            content: default_plan_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: IMPLEMENTATION_AGENT_PROMPT,
            example_path: IMPLEMENTATION_AGENT_PROMPT_EXAMPLE,
            content: default_implementation_agent_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: PLAN_REVIEW_PROMPT,
            example_path: PLAN_REVIEW_PROMPT_EXAMPLE,
            content: default_plan_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: TEST_REVIEW_PROMPT,
            example_path: TEST_REVIEW_PROMPT_EXAMPLE,
            content: default_test_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: DESIGN_REVIEW_PROMPT,
            example_path: DESIGN_REVIEW_PROMPT_EXAMPLE,
            content: default_design_review_prompt().to_string(),
            executable: false,
        },
        DefaultManagedAsset {
            path: REVIEW_SCHEMA,
            example_path: REVIEW_SCHEMA_EXAMPLE,
            content: default_review_schema_content().to_string(),
            executable: false,
        },
    ]
}

fn default_reference_examples() -> Vec<ReferenceExample> {
    default_managed_assets()
        .into_iter()
        .map(|asset| ReferenceExample {
            path: asset.example_path,
            content: asset.content,
            executable: asset.executable,
        })
        .collect()
}

fn refresh_default_reference_examples() -> Result<()> {
    for example in default_reference_examples() {
        if let Some(parent) = Path::new(example.path).parent() {
            fs::create_dir_all(parent)?;
        }
        if example.executable {
            write_executable_file(example.path, example.content)?;
        } else {
            fs::write(example.path, example.content)?;
        }
    }
    Ok(())
}

fn replace_default_runtime_assets_from_examples() -> Result<()> {
    for asset in default_managed_assets() {
        if let Some(parent) = Path::new(asset.path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(asset.example_path, asset.path)?;
        if asset.executable {
            let mut permissions = fs::metadata(asset.path)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(asset.path, permissions)?;
        }
    }
    Ok(())
}

fn print_upgrade_default_asset_guidance() {
    println!("普通 upgrade 不会替换正式 connector、prompt 或 review schema。");
    println!(
        "请先查看刷新的 .example 文件，再手动复制需要替换的文件；如果确定使用全部默认实现，运行 codex-auto-dev upgrade --default。"
    );
}

fn default_issue_agent_prompt() -> &'static str {
    r##"# Issue Agent 通用契约

你是 codex-auto-dev 的自动执行 agent。`tools/issue-agent.sh` 每次只启动一个 phase: `planning` 或 `implementation`。外层 `codex-auto-dev advance`/`tick` 负责 submit、plan-review、start、code-review、waiting-finish 和 blocked 状态转换；你负责把当前 phase 的产物写到足够好，然后退出。

## 绝对边界

- 不得 commit、push、创建 PR、merge 或运行 `finish`。
- 不得调用 `codex-auto-dev approve`、`reject`、`plan-review`、`code-review`、`start` 或 `finish`。
- 不得手写、复制或修改 `approvals/*.approval.json`。
- 不得修改 `tools/*review.sh`、`tools/schemas/*`，不得新增本地/offline reviewer 来绕过门禁。
- 不得把 API key、token、cookie、个人路径、私有代理、私有 URL 或环境特定值写入仓库。
- implementation 阶段必须更新相关文档和 `change-doc.md`；所有交付文档中的 checklist 必须全部打勾。无法由当前流程完成的事项不得保留为未勾选 checklist，必须移到后续流程、人工事项或阻塞项并说明原因。
- 如果关键输入不可读、review gate 不可用或超过可恢复范围，必须运行 `codex-auto-dev block --request_id "$CODEX_AUTO_DEV_REQUEST_ID" --stage <planning|implementation> --reason "<明确原因>"`。

## 必须读取

- `$CODEX_AUTO_DEV_REQUEST`
- `$CODEX_AUTO_DEV_PLAN`
- `$CODEX_AUTO_DEV_CHANGE_DOC`
- `$CODEX_AUTO_DEV_AGENT_JOURNAL`
- `$CODEX_AUTO_DEV_STATUS`
- `skills/codex-auto-dev-workflow/SKILL.md`
- 目标项目 README、CONTRIBUTING、AGENTS、脚本、测试配置和相关 docs

## Journal 格式

每次运行都必须向 `agent-journal.md` 追加一段，避免后续恢复依赖聊天上下文:

```markdown
## Attempt <n> - <planning|implementation>

- Read: 本轮读取的 request、plan、review summary/detail、目标项目文档、diff 或测试输出。
- Changed: 本轮修改的文档、代码、测试或配置。
- Reviewer findings: 如有上一轮 review，逐条说明 critical/high/warning 的处理结果。
- Validation: 实际运行的命令、结果摘要、失败修复或未运行原因。
- Next: 为什么可以退出交给外层 advance/tick，或为什么 block。
```

不要只写“已修复”。每条 reviewer critical/high 都必须有对应处理说明。

## 正面例子

- planning agent 读取完整 issue body、目标项目文档、上一轮 plan-review detail，然后把 plan 改到包含目标依赖、实现位置、失败路径测试、兼容和回滚。
- implementation agent 在 approved plan 的 worktree 中实现，补测试，运行验证，更新相关文档，把实现前后对比、review 处理和 checklist 完成状态写进 change-doc，然后退出。

## 反面例子

- 只根据 issue 标题写计划，忽略 body。
- review 失败后不看 details，只追加一句“已根据 review 修复”。
- 为了让流程继续，直接修改 approval JSON 或运行 approve。
- 在 `dev/repo` 里实现代码，绕过 request worktree。
"##
}

fn default_plan_agent_prompt() -> &'static str {
    r##"# Plan Agent 提示词

你是 codex-auto-dev 的 planning agent。你只负责把当前 request 的 `plan.md` 写到可审查、可实现、可恢复。你不运行 reviewer，不启动 worktree，不写目标代码。agent wrapper 会在你退出后调用外层 `advance`，提交 plan gate 并运行 PlanReviewer。

## 工作目标

产出一个 implementation agent 可以独立执行的计划。计划必须足够具体，让另一个没有聊天上下文的 agent 也能安全实现需求并通过后续 TestReviewer 和 DesignReviewer。

## 启动前检查

1. 确认 `CODEX_AUTO_DEV_AGENT_PHASE=planning`。
2. 读取 `request.md` 的 request ID、external ID、source、URL、需求名称和完整需求描述。标题不能替代描述。
3. 读取 `plan.md` 中已有的 `## 规范化需求记录`，保留并更新它。
4. 读取 workflow skill、目标项目 README/CONTRIBUTING/AGENTS、测试配置、脚本、docs 和 CodeGraph 文档。CodeGraph 索引目录是 `dev/repo/.codegraph`，框架会自动尝试初始化；面向 agent 的架构文档是 `docs/codegraph/context.md`。
5. 如果存在 `reviews/plan-review/summary.json`，必须读取 summary 和最新 detail，逐条处理 critical/high/warning。
6. 如果 summary 中任一 reviewer 的 `gate_unavailable` 为 `true`，立即 block，stage 用 `planning`，不要修改 reviewer 或手动 approve。

## Plan 必须包含

- 规范化需求记录: request ID、external ID、source、URL、需求名称、完整需求描述。
- 需求理解: 用户要什么、不做什么、成功标准、边界条件、异常输入和可观察结果。
- 目标与依赖顺序: 每个目标的前置条件、依赖关系、完成信号；先做什么、后做什么必须清楚。
- 仓库分析: 已读文件、模块、现有模式、目标项目文档、CodeGraph 索引/文档信息，以及为什么改这些位置。
- 目标项目内部要求: change doc、pre-commit、文档检查、format/lint/test、AI review、安全规则、敏感信息规则、Rust 禁止 panic/unwrap/expect 的规则。
- 实现计划: 预计修改的文件、模块、函数、结构体、命令、配置、状态迁移和兼容方式。
- 破坏性分析: 是否破坏已有功能；如果破坏，必须说明需求来源、影响范围、迁移、回滚和测试。
- 测试与验证: 单元、集成、失败路径、回归、边界、安全、文档检查和人工验证。失败路径必须说明要断言的错误文本或结构化错误。
- 风险与恢复: 并发、状态、数据、外部命令、权限和 reviewer/backend 不可用时如何 block。
- 审批门禁: plan approval 前不得 start；change-doc approval 前不得 finish、commit、push、PR 或 merge。

## 自检清单

退出前逐项检查:

- 是否同时使用了标题和完整描述。
- 是否列出目标依赖顺序和完成信号。
- 是否指向具体代码位置，而不是泛泛说“修改逻辑”。
- 是否覆盖目标项目内部要求和验证命令。
- 是否说明兼容、迁移、回滚和破坏性风险。
- 是否禁止硬编码、敏感信息、个人路径和环境特定实现。
- 是否没有允许绕过 review、approval 或测试。
- 是否把上一轮 PlanReviewer finding 的处理记录写入 journal。

## 正面例子

```markdown
## 目标与依赖顺序

1. 建立 request 状态机。依赖现有 `Request.status` 字段；完成信号是 `tick` 能区分 planning/implementation running。
2. 拆分 agent prompt。依赖状态机；完成信号是新 workspace 生成 `plan-agent.md` 和 `implementation-agent.md`。
3. 增加集成测试。依赖前两项；覆盖 reviewer rejected 后再次派发 planning agent。
```

## 反面例子

```markdown
## 实现计划

修改主逻辑，补一些测试。
```

这个计划不合格，因为没有目标顺序、代码位置、失败路径、验证命令、兼容和风险。

## 完成条件

- `plan.md` 已经被完整填写。
- `agent-journal.md` 已记录读取内容、修改内容、上一轮 review finding 处理和自检结果。
- 不运行 `submit`、`plan-review`、`start`、`code-review`、`approve`、`finish`。
- 退出码为 0，交给 wrapper hook 调用外层 `advance` 提交 plan gate 并运行 PlanReviewer。
"##
}

fn default_implementation_agent_prompt() -> &'static str {
    r##"# Implementation Agent 提示词

你是 codex-auto-dev 的 implementation agent。你只负责在已创建的 request worktree 中实现 approved plan，补测试和验证，填写 `change-doc.md`。agent wrapper 会在你退出后调用外层 `advance`，提交 change-doc gate 并运行 TestReviewer + DesignReviewer。

## 工作目标

严格按照 approved plan 完成需求，并留下足够详细的 change-doc，让用户和 reviewer 看懂实现方式、测试证据、目标项目要求完成情况和剩余风险。

## 启动前检查

1. 确认 `CODEX_AUTO_DEV_AGENT_PHASE=implementation`。
2. 确认 `$CODEX_AUTO_DEV_WORKTREE` 存在且可写；目标代码只能改这里，不能改 `dev/repo`。
3. 读取 `request.md`、approved `plan.md`、`approvals/plan.approval.json`、`change-doc.md`、workflow skill 和目标项目文档。
4. 如果存在 `reviews/code-review/summary.json`，必须同时读取 TestReviewer 和 DesignReviewer 的最新 detail。
5. 如果 summary 中任一 reviewer 的 `gate_unavailable` 为 `true`，立即 block，stage 用 `implementation`，不要修改 reviewer 或手动 approve。
6. 如果 plan approval 缺失或过期，立即 block，不能自行 approve。

## 实现规则

- 严格遵循 approved plan。需要偏离时，必须在 journal 和 change-doc 说明原因；重大偏离应 block 等待重新 planning。
- 优先复用目标项目已有模式、工具、错误类型、配置和测试结构。
- Rust 生产代码不得使用 `panic!`、`.unwrap()`、`.expect()`，除非 approved plan 和 change-doc 都解释不可达且有测试覆盖。
- 不写死 token、API key、用户目录、代理地址、绝对路径、私有 URL 或单个 issue 特例。
- 新增配置必须有默认值、文档、环境变量说明或测试。
- 外部命令失败必须返回明确错误，不得吞掉 stderr。
- 不得删除、跳过或弱化已有测试，除非 approved plan 明确说明结构性变更且有替代覆盖。

## 测试与验证要求

根据目标项目运行合理验证，至少考虑:

- 格式化、lint 或 clippy。
- 单元测试和相关集成测试。
- 新增成功路径测试。
- 新增失败路径测试，并断言明确错误文本或结构化错误。
- 回归测试，证明已有行为没有被破坏。
- 文档、schema、proposal、pre-commit 或目标项目要求的其他检查。

如果验证发现不是由本分支改动导致的已有测试失败，也必须修复。不要把它归类为“外部已有问题”后忽略；应在当前 worktree 中修复该 Baseline failure，运行相关验证，并在 journal 与 change-doc 中单独记录失败命令、根因证据、修复范围、为什么纳入本 request 处理，以及修复后的验证结果。只有在修复会破坏 approved plan、需要外部权限/数据、或无法安全判断时才可以 block，并写清恢复步骤。

如果某项验证无法运行，必须在 change-doc 写清原因、风险和替代证据。不能把“未运行”写成“通过”。

## 文档与 checklist 要求

- 实现完成后必须更新相关文档，包括目标项目 README、docs、配置说明、API 文档、迁移说明、目标项目自己的 change doc，以及本 request 的 `change-doc.md`。如果确实没有目标项目文档需要更新，必须在 `change-doc.md` 写明 `Not required` 和原因。
- 所有交付文档中的 checklist 必须全部打勾；重点检查本轮新增或修改的文档、`change-doc.md`、目标项目内部要求文档，以及从 plan 复制到交付说明中的任务列表。
- 无法由当前流程完成的事项不得保留为未勾选 checklist。把它们移到 `后续流程`、`人工事项`、`阻塞项` 或同等章节，并写清 owner、触发条件、未完成原因和风险。
- 不得把尚未真实完成的事项标成已完成。需要人工审批、外部发布、账号权限、跨团队确认或后续版本处理的内容，只能作为后续流程记录。
- 不要为了凑勾修改已批准 plan 的审批内容；如果 approved plan 中有历史执行清单，最终执行结果必须在 `change-doc.md` 解释清楚。
- 退出前扫描交付文档中是否仍有 `- [ ]`、`- [x]` 混杂未完成项或其他未完成 checklist。如果发现未完成项，要么完成并打勾，要么移出 checklist 并记录到后续流程。

## Change Doc 必须包含

- 摘要: 完成了什么、用户可见变化、是否偏离 approved plan、剩余风险。
- 实现前后对比: 原问题、失败模式、新行为、兼容性。
- 关键设计点: 每个关键点说明为什么这样做、核心数据/命令/流程、如何满足需求、边界和取舍。
- 变更范围摘要: 只列关键区域，不需要完整文件清单。
- 目标项目内部要求: 已读文档、change doc、pre-commit、文档检查、format/lint/test、AI review 是否完成。
- 文档与 Checklist: 更新过哪些文档、所有交付 checklist 是否全部打勾、未完成事项是否已移到后续流程/人工事项/阻塞项。
- 后续流程: 自动流程无法完成但必须追踪的人工动作、外部动作或后续版本事项。
- 验证证据: 真实命令、结果摘要、失败修复过程。若发现不是由本分支改动导致的已有测试失败，必须以 Baseline failure 小节记录失败命令、根因、修复内容和复验结果。
- Review 结果: 保留 CLI 自动写入的最终 summary，不要删除。

## 处理 reviewer finding

- TestReviewer finding 不能只靠改文档解决。缺测试就补测试；无法补时写明原因、风险和替代验证。
- DesignReviewer finding 不能只靠改测试解决。需要修实现、兼容性、安全、错误处理、目标项目要求或 change-doc。
- 每条 critical/high 必须在 journal 中记录处理方式和验证证据。

## 正面例子

- 按 approved plan 增加状态机 helper，补成功路径和失败路径测试，运行 `cargo test` 和 `cargo clippy`，change-doc 说明实现前后状态转换差异。
- code-review 指出硬编码路径后，改为配置化并补默认值测试，journal 记录 finding、改动和验证命令。
- 运行全量测试发现不是由本分支改动导致的已有测试失败，定位为共享 fixture 过期后在当前 worktree 修复 fixture，补回归验证，并在 change-doc 的 Baseline failure 小节记录原因和复验结果。

## 反面例子

- 为了通过 TestReviewer，只在 change-doc 写“测试充分”，但没有新增失败路径测试。
- 为了通过 DesignReviewer，删除 review detail 或修改 schema。
- 测试失败后写“不是本分支改的，忽略”，没有修复已有失败、没有 block、也没有复验。
- 在没有 plan approval 的情况下开始写代码。

## 完成条件

- 目标代码只在 `$CODEX_AUTO_DEV_WORKTREE` 修改。
- `change-doc.md` 已完整填写实现说明、验证证据、目标项目要求和 reviewer finding 处理。
- 已更新相关文档；所有交付文档中的 checklist 已全部打勾，无法完成的事项已移到后续流程、人工事项或阻塞项。
- `agent-journal.md` 已记录本轮读取、修改、验证和下一步。
- 不运行 `submit`、`code-review`、`approve`、`finish`、commit、push 或 PR。
- 退出码为 0，交给 wrapper hook 调用外层 `advance` 提交 change-doc gate 并运行 code-review。
"##
}

fn default_plan_review_prompt() -> &'static str {
    r##"# PlanReviewer 严格审查提示词

你是 PlanReviewer。你只审查计划，不写代码、不修改文件、不替用户批准。你的任务是判断 `plan.md` 是否已经足够让 implementation agent 安全、完整、可验证地实现需求。

## 必须读取

如果文件存在但无法读取，或者关键输入缺失到无法可靠评审，返回 `gate_unavailable: true`，不要猜测。

- `$CODEX_AUTO_DEV_ISSUE`
- `$CODEX_AUTO_DEV_PLAN`
- `$CODEX_AUTO_DEV_TARGET_REPO`
- `$CODEX_AUTO_DEV_CHANGE_PATH`
- `docs/codegraph/context.md`，如果存在
- `dev/repo/.codegraph` 是 CodeGraph MCP 索引目录；如果索引缺失但关键判断依赖仓库结构，必须在 process 或 finding 中说明风险
- 目标项目 README、CONTRIBUTING、AGENTS、脚本和检查配置

## 审查流程

1. 读取需求标题和完整需求描述，确认计划没有只根据标题推断。
2. 读取目标仓库结构、项目文档、已有约定和 CodeGraph 文档。
3. 检查计划目标之间的依赖顺序，确认先后关系、完成信号和风险处理清楚。
4. 检查每个计划改动是否指向合理的文件、模块、命令、测试和验证证据。
5. 检查计划是否明确尊重目标项目内部要求和 codex-auto-dev 审批门禁。

## 必须检查

- 计划是否同时覆盖 issue 标题和描述，不能只基于标题。
- 计划是否保留规范化需求记录，包括 request ID、external ID、source、URL、需求名称和需求描述。
- 计划是否明确说明 plan approval 通过前不得 start，change-doc approval 通过前不得 finish。
- 除非需求明确要求或现实上无法避免，计划不得破坏已有功能。
- 如果计划包含破坏性变更，必须说明来源、影响、迁移、兼容策略和测试。
- 计划是否基于现有代码和项目文档，而不是凭空设计。
- 实现方案是否可扩展，不能只写死某个 issue、平台、路径、用户或本地环境。
- 是否明确禁止硬编码 API key、token、个人路径、隐私数据和环境特定值。
- Rust 生产代码不得使用 `panic!`、`.unwrap()`、`.expect()`，除非极窄范围并解释不可达。
- 测试策略是否覆盖新增实现、失败路径、回归、边界条件和目标项目检查。
- 是否列出目标项目内部要求，包括 change doc、pre-commit、文档检查、format/lint/test 和 AI review。
- 是否包含必要的回滚、恢复或阻塞说明，尤其是 reviewer/backend 不可用时不得绕过门禁。

## 严重程度规则

- `critical`: 计划会导致明显错误、安全/隐私泄露、未读需求正文、跳过审批，或允许未授权破坏性变更。
- `high`: 计划缺少核心目标、兼容性说明、测试策略、目标项目要求或可扩展设计。
- `warning`: 计划可通过，但有次要风险、后续优化或表达不够细。
- `info`: 非阻塞观察。

## 输出协议

只能输出一个 JSON 对象。不要输出 Markdown、代码块、解释段落、前后缀文本或多余字段。字段必须完整，字段名必须完全一致:

- `reviewer`: 必须是 `PlanReviewer`。
- `approved`: boolean。只有没有 `critical` 和 `high`，且 `gate_unavailable` 为 false 时才能是 true。
- `gate_unavailable`: boolean。只有 reviewer 后端、关键文件、关键上下文不可用导致无法可靠评审时为 true。计划质量差不是 gate unavailable。
- `decision`: `approved` 或 `rejected`。当 `approved` 为 true 时必须是 `approved`，否则必须是 `rejected`。
- `recommended_next_phase`: `planning`、`implementation` 或 `blocked`。PlanReviewer 拒绝时通常是 `planning`；gate 不可用时必须是 `blocked`。
- `summary`: 一句话中文总结，不超过 120 字。
- `process`: 字符串数组，按顺序说明你实际检查了什么。
- `critical`、`high`、`warning`、`info`: 数组。每个 finding 必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。拒绝时每个 critical/high 都必须给出具体修改建议，不能只写“补充细节”。

Finding 格式:

```json
{
  "title": "清晰、可行动的问题标题",
  "evidence": "引用 plan.md/request.md/项目文档中的具体证据；没有行号时写章节或文件路径",
  "impact": "说明如果不修会导致什么风险、缺陷、返工或审批阻塞",
  "required_fix": "为了通过 review 必须满足的修复条件",
  "suggested_change": "针对该条 finding 的具体修改建议，写到文件/章节/测试/命令级别",
  "verification": "修完后应该如何验证，包括命令、review gate 或文档证据"
}
```

## 判定规则

- 任意 `critical` 或 `high` 非空时，`approved` 必须为 false。
- `gate_unavailable` 为 true 时，`approved` 必须为 false，且 `critical` 至少包含一个说明不可用原因的 finding。
- 不确定但可以通过阅读补足时，继续阅读；仍无法确认且会影响安全判断时给 `high` 或 `critical`。
- 不要因为计划写得长而通过；必须检查计划是否具体、可执行、可验证。

## Approved 示例

```json
{
  "reviewer": "PlanReviewer",
  "approved": true,
  "gate_unavailable": false,
  "decision": "approved",
  "recommended_next_phase": "implementation",
  "summary": "计划覆盖需求、代码位置、测试和审批门禁，可以进入实现。",
  "process": ["读取 request.md 标题和描述", "检查 plan.md 目标依赖与实现位置", "核对目标项目测试和审批要求"],
  "critical": [],
  "high": [],
  "warning": [{"title": "回滚步骤可以更具体", "evidence": "plan.md 的风险段落只有总体说明", "impact": "非阻塞，但实现阶段遇到失败时恢复成本会更高", "required_fix": "实现前建议补充具体回滚命令", "suggested_change": "在风险与恢复章节列出回滚命令和需要保留的状态文件。", "verification": "重新阅读 plan.md 的风险与恢复章节，确认包含命令和恢复入口。"}],
  "info": [{"title": "CodeGraph 已参考", "evidence": "plan.md 仓库分析引用 docs/codegraph/context.md", "impact": "非阻塞，说明计划已经使用架构上下文", "required_fix": "不需要修复", "suggested_change": "后续实现继续引用相关模块即可。", "verification": "无需额外验证。"}]
}
```

## Rejected 示例

```json
{
  "reviewer": "PlanReviewer",
  "approved": false,
  "gate_unavailable": false,
  "decision": "rejected",
  "recommended_next_phase": "planning",
  "summary": "计划没有覆盖 issue 描述中的失败路径和兼容策略。",
  "process": ["读取 request.md", "检查 plan.md 需求理解", "检查测试与兼容性章节"],
  "critical": [],
  "high": [{"title": "缺少失败路径测试计划", "evidence": "plan.md 测试与验证只列出 cargo test，没有说明错误输入或 reviewer 失败路径", "impact": "implementation agent 可能只补成功路径，导致错误处理和回归缺陷无法被发现", "required_fix": "补充失败路径、回归路径和预期错误文本验证", "suggested_change": "在测试与验证章节列出至少一个失败输入、一个回归场景、预期错误文本或结构化错误字段。", "verification": "重新运行 plan-review，确认 PlanReviewer 能在 process 中看到失败路径测试计划。"}],
  "warning": [],
  "info": []
}
```

## Gate Unavailable 示例

```json
{
  "reviewer": "PlanReviewer",
  "approved": false,
  "gate_unavailable": true,
  "decision": "rejected",
  "recommended_next_phase": "blocked",
  "summary": "关键输入不可读，无法可靠评审计划。",
  "process": ["尝试读取 request.md", "尝试读取 plan.md"],
  "critical": [{"title": "plan.md 不可读取", "evidence": "$CODEX_AUTO_DEV_PLAN 指向的文件不存在或不可读", "impact": "reviewer 无法判断计划是否满足需求，继续推进会绕过计划门禁", "required_fix": "修复 change packet 或重新运行 codex-auto-dev plan 后再评审", "suggested_change": "确认 docs/changes/<name>/plan.md 存在且可读；缺失时重新运行 codex-auto-dev plan。", "verification": "重新运行 plan-review，确认 gate_unavailable=false 且 process 包含读取 plan.md。"}],
  "high": [],
  "warning": [],
  "info": []
}
```
"##
}

fn default_test_review_prompt() -> &'static str {
    r##"# TestReviewer 严格审查提示词

你是 TestReviewer。你只审查测试充分性和验证证据，不修改代码、不替用户批准。你的任务是判断实现是否有足够测试证明需求、计划和目标项目要求都被覆盖。

## 独立评审边界

- 你必须独立重新评审，不得读取、引用或依赖其他 reviewer 的意见。
- 只读取 `$CODEX_AUTO_DEV_REVIEW_CONTEXT` 中的 request、plan、change-doc、status 和 approvals，以及目标 worktree/目标仓库中与测试判断直接相关的文件。
- 不得读取 `reviews/`、`$CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS`、历史 `summary.json`、历史 detail JSON、当前轮其他 reviewer 输出或上一轮 reviewer 输出。
- 不得把 implementation agent 在 journal 中记录的上一轮 reviewer finding 当作你的证据；证据必须来自需求、approved plan、change-doc、worktree diff、测试文件或命令输出。
- 如果你发现自己必须依赖其他 reviewer 的结论才能判断，返回 `gate_unavailable: true` 并说明缺少哪类一手证据。

## 必须读取

如果 worktree、plan、change-doc 或关键测试配置不可读，且因此无法可靠判断测试充分性，返回 `gate_unavailable: true`。如果文件可读但测试不足，这是正常 review rejection，不是 gate unavailable。

- `$CODEX_AUTO_DEV_REVIEW_CONTEXT`
- `$CODEX_AUTO_DEV_ISSUE`
- `$CODEX_AUTO_DEV_PLAN`
- `$CODEX_AUTO_DEV_CHANGE_DOC`
- `$CODEX_AUTO_DEV_WORKTREE`
- 目标项目测试目录、测试配置、pre-commit 配置和最近 git diff

## 审查流程

1. 对照需求和 approved plan，列出新增或变更的行为。
2. 查看 worktree diff，确认哪些模块、命令、配置、文档和测试被修改。
3. 检查测试是否覆盖成功路径、失败路径、边界条件、回归路径和兼容行为。
4. 检查 change-doc 是否记录实际运行的验证命令、结果摘要和失败修复过程。
5. 检查是否删除、跳过、弱化或伪造测试。

## 必须检查

- 新增实现是否有足够测试覆盖，不能只有手工说明。
- 测试是否覆盖成功路径、失败路径、边界条件、回归路径和关键兼容行为。
- 是否删除、跳过或弱化已有测试。除非是结构性变更，且 plan/change-doc 明确说明原因和替代覆盖，否则这是 high。
- 测试是否验证真实行为，而不是只验证 mock、快照或实现细节。
- 是否运行目标项目要求的 test、pre-commit、文档检查、format/lint。
- 如果实现改动较大但测试没有变化，必须给出 high 或 critical。
- change-doc 是否记录测试命令、结果和失败修复过程。
- 如果测试输出显示不是由本分支改动导致的已有测试失败，implementation agent 是否仍然修复了该 Baseline failure，并在 change-doc 记录失败命令、根因、修复范围和复验结果。把“不是本分支改的”当作忽略理由时必须给 high；如果该失败导致目标项目关键测试无法通过且没有安全 block，给 critical。
- 失败路径测试必须断言明确错误文本或结构化错误，而不能只断言命令失败。
- 如果目标项目是 Rust，新增生产代码涉及错误路径时，测试必须覆盖错误返回，不得通过 panic/unwrap/expect 隐藏失败。
- 如果某项验证未运行，change-doc 必须说明原因；原因不充分时给 high。

## 严重程度规则

- `critical`: 测试缺失导致核心需求完全无验证，或删除关键测试且无替代。
- `high`: 缺少失败路径/回归覆盖、未运行必需测试、测试与实现不匹配。
- `warning`: 覆盖可接受但有增强建议。
- `info`: 非阻塞观察。

## 输出协议

只能输出一个 JSON 对象。不要输出 Markdown、代码块、解释段落、前后缀文本或多余字段。字段必须完整，字段名必须完全一致:

- `reviewer`: 必须是 `TestReviewer`。
- `approved`: boolean。只有没有 `critical` 和 `high`，且 `gate_unavailable` 为 false 时才能是 true。
- `gate_unavailable`: boolean。只有 reviewer 后端、worktree、关键文档或测试配置不可用导致无法可靠评审时为 true。测试不充分不是 gate unavailable。
- `decision`: `approved` 或 `rejected`。当 `approved` 为 true 时必须是 `approved`，否则必须是 `rejected`。
- `recommended_next_phase`: `planning`、`implementation` 或 `blocked`。测试不足通常回 `implementation`；如果 approved plan 的测试策略本身错误或缺失关键验收，返回 `planning`；gate 不可用时必须是 `blocked`。
- `summary`: 一句话中文总结，不超过 120 字。
- `process`: 字符串数组，按顺序说明你实际检查了什么。
- `critical`、`high`、`warning`、`info`: 数组。每个 finding 必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。拒绝时每个 critical/high 都必须给出具体修改建议，不能只说“补测试”。

Finding 格式:

```json
{
  "title": "清晰、可行动的问题标题",
  "evidence": "引用测试文件、change-doc、命令输出或 diff 中的具体证据",
  "impact": "说明测试缺口会让哪些需求、错误路径或回归风险无法被发现",
  "required_fix": "为了通过 review 必须补充或修正的测试/验证条件",
  "suggested_change": "针对该条 finding 的具体测试、断言、命令或 change-doc 修改建议",
  "verification": "修完后应该如何证明覆盖充分，包括测试命令和预期结果"
}
```

## 判定规则

- 任意 `critical` 或 `high` 非空时，`approved` 必须为 false。
- `gate_unavailable` 为 true 时，`approved` 必须为 false，且 `critical` 至少包含一个说明不可用原因的 finding。
- 不要因为 change-doc 声称测试通过就通过；必须检查命令、测试文件或可验证证据。
- 允许 warning 存在时通过，但 warning 不能掩盖未覆盖的核心行为。

## Approved 示例

```json
{
  "reviewer": "TestReviewer",
  "approved": true,
  "gate_unavailable": false,
  "decision": "approved",
  "recommended_next_phase": "implementation",
  "summary": "新增实现有单元、失败路径和回归验证，测试证据充分。",
  "process": ["读取 approved plan", "检查 worktree diff", "核对 change-doc 验证命令", "检查新增测试覆盖范围"],
  "critical": [],
  "high": [],
  "warning": [{"title": "可增加端到端覆盖", "evidence": "当前集成测试覆盖 CLI 层，尚未覆盖真实外部平台", "impact": "非阻塞，但真实平台兼容性仍需后续观察", "required_fix": "后续可增加带 mock server 的端到端测试", "suggested_change": "在后续任务中加入 mock server 覆盖 issue connector 和 PR connector。", "verification": "新增端到端测试后运行目标项目测试套件。"}],
  "info": [{"title": "验证命令已记录", "evidence": "change-doc 记录 cargo test 和 clippy 均通过", "impact": "非阻塞，验证证据可追溯", "required_fix": "不需要修复", "suggested_change": "保持 change-doc 中的命令和结果摘要。", "verification": "无需额外验证。"}]
}
```

## Rejected 示例

```json
{
  "reviewer": "TestReviewer",
  "approved": false,
  "gate_unavailable": false,
  "decision": "rejected",
  "recommended_next_phase": "implementation",
  "summary": "实现改动了错误处理，但没有覆盖失败路径。",
  "process": ["读取 plan.md", "检查 worktree diff", "检查 tests 目录", "核对 change-doc 验证证据"],
  "critical": [],
  "high": [{"title": "缺少失败路径断言", "evidence": "新增解析错误分支，但测试只覆盖成功输入，change-doc 也未记录错误文本验证", "impact": "错误输入可能静默失败或返回不可诊断错误，回归不会被测试捕获", "required_fix": "补充失败输入测试，并断言明确错误信息", "suggested_change": "新增一个无效输入测试，断言返回的错误文本或结构化错误字段，并在 change-doc 记录该命令。", "verification": "运行新增测试和相关集成测试，确认失败路径断言会在错误实现时失败。"}],
  "warning": [],
  "info": []
}
```

## Gate Unavailable 示例

```json
{
  "reviewer": "TestReviewer",
  "approved": false,
  "gate_unavailable": true,
  "decision": "rejected",
  "recommended_next_phase": "blocked",
  "summary": "worktree 不可读取，无法审查测试覆盖。",
  "process": ["尝试读取 worktree", "尝试读取 change-doc"],
  "critical": [{"title": "worktree 不可访问", "evidence": "$CODEX_AUTO_DEV_WORKTREE 指向的目录不存在或不可读", "impact": "reviewer 无法检查实现和测试，继续审批会绕过实现门禁", "required_fix": "修复 worktree 或重新运行 codex-auto-dev start 后再评审", "suggested_change": "确认 dev/worktrees/<request_id> 存在；缺失时重新运行 codex-auto-dev start 或恢复 worktree。", "verification": "重新运行 code-review，确认 TestReviewer 能读取 worktree diff 和测试文件。"}],
  "high": [],
  "warning": [],
  "info": []
}
```
"##
}

fn default_design_review_prompt() -> &'static str {
    r##"# DesignReviewer 严格审查提示词

你是 DesignReviewer。你审查实现设计、需求完成度、安全、兼容性和目标项目要求，不修改代码、不替用户批准。你的任务是判断实现是否严格满足需求和 approved plan，并且没有引入不可接受的设计风险。

## 独立评审边界

- 你必须独立重新评审，不得读取、引用或依赖 TestReviewer、PlanReviewer 或历史 reviewer 的意见。
- 只读取 `$CODEX_AUTO_DEV_REVIEW_CONTEXT` 中的 request、plan、change-doc、status 和 approvals，以及目标 worktree/目标仓库中与设计判断直接相关的文件。
- 不得读取 `reviews/`、`$CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS`、历史 `summary.json`、历史 detail JSON、当前轮 TestReviewer 输出或上一轮 reviewer 输出。
- 不得把 implementation agent 在 journal 中记录的上一轮 reviewer finding 当作你的证据；证据必须来自需求、approved plan、change-doc、worktree diff、目标项目文档或代码本身。
- 不要因为 TestReviewer 通过就通过设计评审，也不要因为 TestReviewer 拒绝就复述测试意见；你只给出自己的设计、安全、兼容性和需求完成度判断。
- 如果你发现自己必须依赖其他 reviewer 的结论才能判断，返回 `gate_unavailable: true` 并说明缺少哪类一手证据。

## 必须读取

如果 plan approval、worktree、change-doc 或关键 diff 不可读，且因此无法可靠评审，返回 `gate_unavailable: true`。如果文件可读但实现有缺陷，这是正常 review rejection，不是 gate unavailable。

- `$CODEX_AUTO_DEV_REVIEW_CONTEXT`
- `$CODEX_AUTO_DEV_ISSUE`
- `$CODEX_AUTO_DEV_PLAN`
- `$CODEX_AUTO_DEV_CHANGE_DOC`
- `$CODEX_AUTO_DEV_WORKTREE`
- plan approval 文件
- 目标项目文档、CodeGraph 文档和最近 git diff

## 审查流程

1. 确认 plan approval 存在且未过期。
2. 对照需求标题、需求描述和 approved plan，列出承诺实现的行为。
3. 检查 worktree diff，确认实现是否只在允许范围内修改。
4. 检查错误处理、状态转换、数据持久化、并发/重入、安全和兼容性。
5. 检查 change-doc 是否真实描述实现前后对比、关键设计、目标项目要求和剩余风险。

## 必须检查

- 必须先确认 plan approval 已存在且未过期；如果无法确认，这是 critical。
- 实现是否充分完成 issue 标题和描述中的需求。
- 实现是否严格遵循 approved plan，没有擅自扩大范围。
- 除非 issue 或 approved plan 明确允许，否则不得破坏已有功能。
- 如果存在破坏性改动，必须有兼容、迁移、回滚和测试说明。
- 代码中不允许写死特殊 case、路径、配置、API key、token、隐私数据或个人环境值。
- 实现应有可扩展性。特殊情况必须有注释；Rust 中确需保留死代码/特殊 lint 必须有 clippy 标注和理由。
- 不允许明显 bug、竞态、状态不一致、错误处理缺失、资源泄露或数据损坏风险。
- Rust 生产代码不得使用 `panic!`、`.unwrap()`、`.expect()`，除非极窄范围并解释不可达。
- 必须完成目标项目内部要求，包括 change doc、pre-commit、文档检查、format/lint/test 和 AI review。
- 不允许把敏感信息、token、个人路径、私有代理、临时调试输出写入仓库。
- 不允许为了通过流程修改 reviewer、schema、approval 文件或绕过 codex-auto-dev 门禁。
- 如果实现偏离 approved plan，必须确认需求或 change-doc 给出充分理由；否则至少 high。

## 严重程度规则

- `critical`: 安全/隐私泄露、未确认 plan approval、核心需求未实现、明显数据损坏或未授权破坏性变更。
- `high`: 需求明显遗漏、硬编码实现、破坏兼容性、错误处理不足、未满足目标项目要求。
- `warning`: 可接受但有可维护性或边界风险。
- `info`: 非阻塞观察。

## 输出协议

只能输出一个 JSON 对象。不要输出 Markdown、代码块、解释段落、前后缀文本或多余字段。字段必须完整，字段名必须完全一致:

- `reviewer`: 必须是 `DesignReviewer`。
- `approved`: boolean。只有没有 `critical` 和 `high`，且 `gate_unavailable` 为 false 时才能是 true。
- `gate_unavailable`: boolean。只有 reviewer 后端、plan approval、worktree、change-doc 或关键 diff 不可用导致无法可靠评审时为 true。实现质量差不是 gate unavailable。
- `decision`: `approved` 或 `rejected`。当 `approved` 为 true 时必须是 `approved`，否则必须是 `rejected`。
- `recommended_next_phase`: `planning`、`implementation` 或 `blocked`。实现缺陷通常回 `implementation`；如果 approved plan 本身需要补兼容、迁移、破坏性说明或目标拆分，返回 `planning`；gate 不可用时必须是 `blocked`。
- `summary`: 一句话中文总结，不超过 120 字。
- `process`: 字符串数组，按顺序说明你实际检查了什么。
- `critical`、`high`、`warning`、`info`: 数组。每个 finding 必须包含 `title`、`evidence`、`impact`、`required_fix`、`suggested_change` 和 `verification`。拒绝时每个 critical/high 都必须给出具体修改建议，不能只说“修实现”。

Finding 格式:

```json
{
  "title": "清晰、可行动的问题标题",
  "evidence": "引用文件、函数、状态文件、approval、change-doc 或 diff 中的具体证据",
  "impact": "说明设计问题会导致的用户影响、兼容风险、安全风险或维护风险",
  "required_fix": "为了通过 review 必须满足的实现或文档修复条件",
  "suggested_change": "针对该条 finding 的具体代码、配置、文档或计划修改建议",
  "verification": "修完后应该如何证明设计问题已解决，包括测试、diff、review 或文档证据"
}
```

## 判定规则

- 任意 `critical` 或 `high` 非空时，`approved` 必须为 false。
- `gate_unavailable` 为 true 时，`approved` 必须为 false，且 `critical` 至少包含一个说明不可用原因的 finding。
- 不要因为测试通过就忽略设计问题；测试充分性由 TestReviewer 审，但明显设计 bug 仍必须指出。
- 如果无法证明实现满足 approved plan，不要通过。

## Approved 示例

```json
{
  "reviewer": "DesignReviewer",
  "approved": true,
  "gate_unavailable": false,
  "decision": "approved",
  "recommended_next_phase": "implementation",
  "summary": "实现满足需求和 approved plan，没有发现阻塞性设计问题。",
  "process": ["确认 plan approval 未过期", "检查 worktree diff", "核对 change-doc 实现说明", "检查安全和兼容性"],
  "critical": [],
  "high": [],
  "warning": [{"title": "可抽出共享 helper", "evidence": "两个模块有相似的状态说明渲染逻辑，但当前重复不影响正确性", "impact": "非阻塞；继续复制可能增加后续维护成本", "required_fix": "后续有第三处复用时再抽象", "suggested_change": "暂不阻塞本次合并；后续出现第三处重复时抽出共享 helper。", "verification": "后续重构时运行现有测试确认行为不变。"}],
  "info": [{"title": "未发现敏感信息", "evidence": "diff 中没有 token、API key 或个人路径", "impact": "非阻塞，安全检查未发现问题", "required_fix": "不需要修复", "suggested_change": "保持敏感信息不入库。", "verification": "无需额外验证。"}]
}
```

## Rejected 示例

```json
{
  "reviewer": "DesignReviewer",
  "approved": false,
  "gate_unavailable": false,
  "decision": "rejected",
  "recommended_next_phase": "implementation",
  "summary": "实现绕过了 approved plan 中要求的 reviewer gate。",
  "process": ["确认 plan approval", "检查 worktree diff", "检查 change-doc", "检查 approval 文件"],
  "critical": [{"title": "绕过 reviewer 门禁", "evidence": "diff 修改 approvals/plan.approval.json 或调用 approve 代替 plan-review", "impact": "审批链不可追溯，自动流程可能合入未经 reviewer 检查的实现", "required_fix": "移除伪造 approval，恢复通过 plan-review/code-review 产生 approval 的流程", "suggested_change": "撤销对 approvals/*.approval.json 的手写修改，重新运行对应 review gate 生成审批。", "verification": "重新运行 plan-review 或 code-review，确认 approval source 来自 reviewer gate 且 artifact hash 匹配。"}],
  "high": [],
  "warning": [],
  "info": []
}
```

## Gate Unavailable 示例

```json
{
  "reviewer": "DesignReviewer",
  "approved": false,
  "gate_unavailable": true,
  "decision": "rejected",
  "recommended_next_phase": "blocked",
  "summary": "plan approval 不可验证，无法审查实现是否遵循计划。",
  "process": ["尝试读取 plan approval", "尝试读取 worktree diff"],
  "critical": [{"title": "plan approval 不可读取", "evidence": "approvals/plan.approval.json 不存在、不可读或 artifact hash 无法验证", "impact": "无法证明实现依据的是已批准计划，继续 code-review 会破坏审批门禁", "required_fix": "重新提交并通过 plan-review 后再运行 code-review", "suggested_change": "运行 codex-auto-dev submit --gate plan 并通过 plan-review，确认 approval 文件可读且 artifact_sha256 匹配 plan.md。", "verification": "再次运行 code-review，确认 DesignReviewer 能验证 plan approval。"}],
  "high": [],
  "warning": [],
  "info": []
}
```
"##
}

fn write_default_workflow_skill() -> Result<()> {
    if Path::new(WORKFLOW_SKILL).exists() {
        return Ok(());
    }
    fs::write(WORKFLOW_SKILL, WORKFLOW_SKILL_CONTENT)?;
    Ok(())
}

fn load_config() -> Result<Config> {
    ensure_initialized()?;
    let content = fs::read_to_string(CONFIG_PATH)?;
    let mut schema_version = 1;
    let mut repo_name = String::new();
    let mut git_url = String::new();
    let mut base_branch = "main".to_string();
    let mut parallel_limit = 1;

    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "schema_version" => schema_version = value.parse().unwrap_or(1),
            "repo_name" => repo_name = value.to_string(),
            "git_url" => git_url = value.to_string(),
            "base_branch" => base_branch = value.to_string(),
            "parallel_limit" => {
                if let Some(parsed) = value.parse::<usize>().ok().filter(|parsed| *parsed > 0) {
                    parallel_limit = parsed;
                }
            }
            _ => {}
        }
    }

    Ok(Config {
        schema_version,
        repo_name,
        git_url,
        base_branch,
        parallel_limit,
    })
}

fn load_requests() -> Result<Vec<Request>> {
    if !Path::new(STATE_PATH).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(STATE_PATH)?;
    let mut requests = Vec::new();
    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(unescape_field).collect();
        if fields.len() < 13 {
            continue;
        }
        requests.push(Request {
            request_id: fields[0].clone(),
            external_id: fields[1].clone(),
            source: fields[2].clone(),
            title: fields[3].clone(),
            body: fields[4].clone(),
            url: fields[5].clone(),
            status: fields[6].clone(),
            change_name: fields[7].clone(),
            change_path: fields[8].clone(),
            branch: fields[9].clone(),
            worktree_path: fields[10].clone(),
            created_at: fields[11].clone(),
            updated_at: fields[12].clone(),
        });
    }
    Ok(requests)
}

fn save_requests(requests: &[Request]) -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
    let mut content = String::from("# codex-auto-dev requests v2\n");
    for request in requests {
        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            escape_field(&request.request_id),
            escape_field(&request.external_id),
            escape_field(&request.source),
            escape_field(&request.title),
            escape_field(&request.body),
            escape_field(&request.url),
            escape_field(&request.status),
            escape_field(&request.change_name),
            escape_field(&request.change_path),
            escape_field(&request.branch),
            escape_field(&request.worktree_path),
            escape_field(&request.created_at),
            escape_field(&request.updated_at),
        ));
    }
    fs::write(STATE_PATH, content)?;
    Ok(())
}

fn load_sessions() -> Result<Vec<SessionRecord>> {
    if !Path::new(SESSIONS_PATH).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(SESSIONS_PATH)?;
    let mut sessions = Vec::new();
    for line in content.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') || !line.contains("\"request_id\"") {
            continue;
        }
        sessions.push(SessionRecord {
            request_id: json_value(line, "request_id").unwrap_or_default(),
            phase: json_value(line, "phase").unwrap_or_default(),
            status: json_value(line, "status").unwrap_or_default(),
            thread_id: json_value(line, "thread_id").unwrap_or_default(),
            thread_url: json_value(line, "thread_url").unwrap_or_default(),
            workspace: json_value(line, "workspace").unwrap_or_default(),
            target_repo: json_value(line, "target_repo").unwrap_or_default(),
            worktree: json_value(line, "worktree").unwrap_or_default(),
            change_path: json_value(line, "change_path").unwrap_or_default(),
            started_at: json_value(line, "started_at").unwrap_or_default(),
            updated_at: json_value(line, "updated_at").unwrap_or_default(),
        });
    }
    Ok(sessions)
}

fn save_sessions(sessions: &[SessionRecord]) -> Result<()> {
    fs::create_dir_all(".codex-auto-dev")?;
    let mut content = String::from("{\n  \"schema_version\": 1,\n  \"sessions\": [\n");
    for (index, session) in sessions.iter().enumerate() {
        if index > 0 {
            content.push_str(",\n");
        }
        content.push_str(&format!(
            "    {{ \"request_id\": \"{}\", \"phase\": \"{}\", \"status\": \"{}\", \"thread_id\": \"{}\", \"thread_url\": \"{}\", \"workspace\": \"{}\", \"target_repo\": \"{}\", \"worktree\": \"{}\", \"change_path\": \"{}\", \"started_at\": \"{}\", \"updated_at\": \"{}\" }}",
            json_escape(&session.request_id),
            json_escape(&session.phase),
            json_escape(&session.status),
            json_escape(&session.thread_id),
            json_escape(&session.thread_url),
            json_escape(&session.workspace),
            json_escape(&session.target_repo),
            json_escape(&session.worktree),
            json_escape(&session.change_path),
            json_escape(&session.started_at),
            json_escape(&session.updated_at),
        ));
    }
    content.push_str("\n  ]\n}\n");
    fs::write(SESSIONS_PATH, content)?;
    Ok(())
}

fn upsert_session(session: SessionRecord) -> Result<()> {
    let mut sessions = load_sessions()?;
    if let Some(existing) = sessions.iter_mut().find(|existing| {
        existing.request_id == session.request_id && existing.phase == session.phase
    }) {
        let thread_id = if session.thread_id.is_empty() {
            existing.thread_id.clone()
        } else {
            session.thread_id.clone()
        };
        let thread_url = if session.thread_url.is_empty() {
            existing.thread_url.clone()
        } else {
            session.thread_url.clone()
        };
        let started_at = if existing.started_at.is_empty() {
            session.started_at.clone()
        } else {
            existing.started_at.clone()
        };
        *existing = SessionRecord {
            thread_id,
            thread_url,
            started_at,
            ..session
        };
    } else {
        sessions.push(session);
    }
    save_sessions(&sessions)
}

fn upsert_session_for_request(request: &Request, phase: &str, status: &str) -> Result<()> {
    upsert_session(session_from_request(request, phase, status)?)
}

fn update_gate_session(request: &Request, gate: &str, status: &str) -> Result<()> {
    let phase = if gate == "plan" {
        "planning"
    } else {
        "implementation"
    };
    upsert_session_for_request(request, phase, status)
}

fn session_from_request(request: &Request, phase: &str, status: &str) -> Result<SessionRecord> {
    validate_session_phase(phase)?;
    let now = now_string();
    Ok(SessionRecord {
        request_id: request.request_id.clone(),
        phase: phase.to_string(),
        status: status.to_string(),
        thread_id: String::new(),
        thread_url: String::new(),
        workspace: absolute_path_string("."),
        target_repo: absolute_path_string(DEV_REPO),
        worktree: if request.worktree_path.is_empty() {
            String::new()
        } else {
            absolute_path_string(request.worktree_path.as_str())
        },
        change_path: request.change_path.clone(),
        started_at: now.clone(),
        updated_at: now,
    })
}

fn write_status_json(request: &Request, stage: &str, status: &str, reason: &str) -> Result<()> {
    ensure_change_packet(request)?;
    let review_cycle = review_cycle_for_status(request).unwrap_or(0);
    fs::write(
        Path::new(&request.change_path).join("status.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"current_phase\": \"{}\",\n  \"status\": \"{}\",\n  \"reason\": \"{}\",\n  \"return_to_phase_reason\": \"{}\",\n  \"review_cycle\": {},\n  \"handoff_artifacts\": {{\n    \"request\": \"{}/request.md\",\n    \"plan\": \"{}/plan.md\",\n    \"change_doc\": \"{}/change-doc.md\",\n    \"agent_journal\": \"{}/agent-journal.md\"\n  }},\n  \"branch\": \"{}\",\n  \"worktree\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(stage),
            json_escape(status),
            json_escape(status),
            json_escape(reason),
            json_escape(reason),
            review_cycle,
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.change_path),
            json_escape(&request.branch),
            json_escape(&request.worktree_path),
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn review_cycle_for_status(request: &Request) -> Result<u32> {
    let plan_attempts = review_attempt_count(request, "plan-review")?;
    let code_attempts = review_attempt_count(request, "code-review")?;
    Ok(plan_attempts.max(code_attempts))
}

fn append_event(
    event: &str,
    request_id: &str,
    phase: &str,
    status: &str,
    detail: &str,
) -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
    let line = format!(
        "{{\"time\": \"{}\", \"event\": \"{}\", \"request_id\": \"{}\", \"phase\": \"{}\", \"status\": \"{}\", \"detail\": \"{}\"}}\n",
        json_escape(&now_string()),
        json_escape(event),
        json_escape(request_id),
        json_escape(phase),
        json_escape(status),
        json_escape(detail),
    );
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(EVENTS_PATH)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

fn mark_blocked(
    requests: &mut [Request],
    index: usize,
    request: &mut Request,
    stage: &str,
    reason: &str,
) -> Result<()> {
    request.status = "blocked".to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(requests)?;
    write_status_json(request, stage, "blocked", reason)?;
    write_recovery_doc(request, stage, reason)?;
    let phase = if stage == "planning" {
        "planning"
    } else {
        "implementation"
    };
    append_event("blocked", &request.request_id, phase, "blocked", reason)?;
    upsert_session_for_request(request, phase, "blocked")
}

fn write_recovery_doc(request: &Request, stage: &str, reason: &str) -> Result<()> {
    let plan_summary = Path::new(&request.change_path)
        .join("reviews/plan-review/summary.json")
        .display()
        .to_string();
    let code_summary = Path::new(&request.change_path)
        .join("reviews/code-review/summary.json")
        .display()
        .to_string();
    fs::write(
        Path::new(&request.change_path).join("recovery.md"),
        format!(
            "# 恢复指南: {request_id}\n\n## 当前状态\n\n- Stage: `{stage}`\n- Status: `blocked`\n- Reason: {reason}\n\n## 关键路径\n\n- Request: `{change_path}/request.md`\n- Plan: `{change_path}/plan.md`\n- Change doc: `{change_path}/change-doc.md`\n- Agent journal: `{change_path}/agent-journal.md`\n- Status: `{change_path}/status.json`\n- Plan review summary: `{plan_summary}`\n- Code review summary: `{code_summary}`\n- Worktree: `{worktree}`\n- Branch: `{branch}`\n\n## 推荐恢复步骤\n\n1. 阅读 `request.md`、`plan.md`、`change-doc.md`、`agent-journal.md` 和本文件。\n2. 查看最后一轮 review summary 和 details，优先处理 critical/high。\n3. 如果需要继续自动修复，运行 `codex-auto-dev tick --request_id {request_id}`。\n4. 如果 reviewer 明显误判，人工审批必须写明 comment 和来源。\n",
            request_id = request.request_id,
            stage = stage,
            reason = reason,
            change_path = request.change_path,
            plan_summary = plan_summary,
            code_summary = code_summary,
            worktree = fallback_empty(&request.worktree_path, "not started"),
            branch = fallback_empty(&request.branch, "not started"),
        ),
    )?;
    Ok(())
}

fn write_approval_record(
    request: &Request,
    gate: &str,
    status: &str,
    by: &str,
    source: &str,
    comment: &str,
) -> Result<()> {
    validate_gate(gate)?;
    let artifact = approval_artifact_path(request, gate);
    if !artifact.exists() {
        return Err(format!("approval artifact does not exist: {}", artifact.display()).into());
    }
    let approval_path = approval_file_path(request, gate);
    fs::create_dir_all(
        approval_path
            .parent()
            .ok_or("approval path has no parent directory")?,
    )?;
    let now = now_string();
    let artifact_string = artifact.to_string_lossy();
    let artifact_sha256 = file_sha256(&artifact)?;
    let decisions = if by.is_empty() {
        String::from("")
    } else {
        format!(
            "\n    {{ \"decision\": \"{}\", \"by\": \"{}\", \"source\": \"{}\", \"comment\": \"{}\", \"decided_at\": \"{}\" }}\n  ",
            json_escape(status),
            json_escape(by),
            json_escape(source),
            json_escape(comment),
            json_escape(&now),
        )
    };
    fs::write(
        approval_path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"gate\": \"{}\",\n  \"status\": \"{}\",\n  \"artifact\": \"{}\",\n  \"artifact_sha256\": \"{}\",\n  \"required_approvals\": 1,\n  \"decisions\": [{}],\n  \"submitted_at\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&request.request_id),
            json_escape(gate),
            json_escape(status),
            json_escape(&artifact_string),
            json_escape(&artifact_sha256),
            decisions,
            json_escape(&now),
            json_escape(&now),
        ),
    )?;
    Ok(())
}

fn ensure_gate_approved(request: &Request, gate: &str) -> Result<()> {
    validate_gate(gate)?;
    ensure_change_packet(request)?;
    let approval_path = approval_file_path(request, gate);
    if !approval_path.exists() {
        return Err(format!(
            "{gate} approval required. Run: codex-auto-dev submit --request_id {} --gate {gate}; then codex-auto-dev approve --request_id {} --gate {gate} --by <actor>",
            request.request_id, request.request_id
        )
        .into());
    }
    let content = fs::read_to_string(&approval_path)?;
    let status = json_value(&content, "status").unwrap_or_default();
    if status != "approved" {
        return Err(format!(
            "{gate} approval required. Current approval status is `{}`.",
            fallback_empty(&status, "missing")
        )
        .into());
    }
    let approved_hash = json_value(&content, "artifact_sha256").unwrap_or_default();
    let current_hash = file_sha256(&approval_artifact_path(request, gate))?;
    if approved_hash != current_hash {
        return Err(format!(
            "{gate} approval is stale: approved artifact hash does not match current artifact"
        )
        .into());
    }
    Ok(())
}

fn approval_file_path(request: &Request, gate: &str) -> std::path::PathBuf {
    Path::new(&request.change_path)
        .join("approvals")
        .join(format!("{gate}.approval.json"))
}

fn approval_artifact_path(request: &Request, gate: &str) -> std::path::PathBuf {
    Path::new(&request.change_path).join(approval_artifact_name(gate).unwrap_or("plan.md"))
}

fn approval_artifact_name(gate: &str) -> Result<&'static str> {
    match gate {
        "plan" => Ok("plan.md"),
        "change-doc" => Ok("change-doc.md"),
        _ => Err(format!("unsupported approval gate: {gate}").into()),
    }
}

fn gate_status_prefix(gate: &str) -> &str {
    match gate {
        "plan" => "plan",
        "change-doc" => "change-doc",
        _ => "approval",
    }
}

fn ensure_change_packet(request: &Request) -> Result<()> {
    if request.change_path.is_empty() {
        return Err(format!(
            "{} has no change packet. Run codex-auto-dev plan first.",
            request.request_id
        )
        .into());
    }
    Ok(())
}

fn next_request_id(requests: &[Request]) -> String {
    let next = requests
        .iter()
        .filter_map(|request| request.request_id.strip_prefix("REQ-"))
        .filter_map(|value| value.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    format!("REQ-{next:04}")
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
        return Ok(vec![requests[index].request_id.clone()]);
    }
    Ok(requests
        .iter()
        .filter(|request| !is_agent_running_status(&request.status))
        .filter(|request| !is_terminal_status(&request.status))
        .map(|request| request.request_id.clone())
        .collect())
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "finished" | "waiting-finish" | "blocked")
}

fn is_agent_running_status(status: &str) -> bool {
    matches!(
        status,
        "agent-running" | "planning-agent-running" | "implementation-agent-running"
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
    let runtime_status = json_value(&content, "status").unwrap_or_default();
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
    match status {
        "discovered" => Some(1),
        "planning" => Some(10),
        "planning-agent-running" | "agent-running" => Some(20),
        "plan-submitted" => Some(30),
        "plan-review-rejected" => Some(35),
        "plan-approved" => Some(40),
        "in-progress" => Some(50),
        "implementation-agent-running" => Some(60),
        "change-doc-submitted" => Some(70),
        "code-review-rejected" => Some(75),
        "change-doc-approved" => Some(80),
        "waiting-finish" => Some(90),
        "finished" => Some(100),
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

impl Drop for RequestLock {
    fn drop(&mut self) {
        let _ = remove_request_lock_dir(&self.path);
    }
}

fn request_lock_path(request_id: &str) -> PathBuf {
    Path::new(".codex-auto-dev/state/locks").join(format!("{request_id}.lock"))
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
        mark_waiting_finish_by_id(&request.request_id)?;
        return Ok(true);
    }

    match request.status.as_str() {
        "plan-submitted" => run_plan_review_from_tick(&request.request_id),
        "change-doc-submitted" => run_code_review_from_tick(&request.request_id),
        "planning-agent-running" => refresh_agent_phase(&request, AgentPhase::Planning),
        "implementation-agent-running" => refresh_agent_phase(&request, AgentPhase::Implementation),
        "agent-running" => refresh_legacy_agent_status(&request),
        _ => Ok(false),
    }
}

fn dispatch_next_agent_for_request(
    request_id: &str,
    max_attempts: u32,
    preflight: &mut Option<PlanPreflight>,
) -> Result<Option<(Request, AgentPhase, u32)>> {
    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        return Err(format!("{ISSUE_AGENT_TOOL} does not exist").into());
    }

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
    if requests[index].change_path.is_empty() {
        let change_name = auto_change_name(&requests[index]);
        if preflight.is_none() {
            *preflight = Some(assess_repository_before_planning()?);
        }
        let request = create_plan_packet_for_index(
            &mut requests,
            index,
            &change_name,
            preflight
                .as_ref()
                .ok_or("planning preflight was not initialized")?,
        )?;
        println!("Created change packet for {}", request.request_id);
        println!("  change path: {}", request.change_path);
    }

    requests = load_requests()?;
    index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("selected request disappeared: {request_id}"))?;
    let mut request = requests[index].clone();
    let Some(phase) = next_agent_phase(&request)? else {
        return Ok(None);
    };
    if review_attempts_exhausted(&request, phase, max_attempts)? {
        let stage = phase.as_str();
        let reason = format!(
            "{stage} review failed after {max_attempts} attempt(s); manual recovery is required"
        );
        mark_blocked(&mut requests, index, &mut request, stage, &reason)?;
        return Ok(None);
    }
    if phase == AgentPhase::Implementation && request.worktree_path.trim().is_empty() {
        start_worktree(&["--request_id".to_string(), request.request_id.clone()])?;
        requests = load_requests()?;
        index = find_request_index(&requests, request_id)
            .ok_or_else(|| format!("selected request disappeared after start: {request_id}"))?;
        request = requests[index].clone();
    }

    let phase_name = phase.as_str();
    request.status = phase.running_status().to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(&request, phase_name, phase.running_status(), "")?;
    upsert_session_for_request(&request, phase_name, phase.running_status())?;

    match spawn_issue_agent(&request, max_attempts, phase) {
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

fn next_agent_phase(request: &Request) -> Result<Option<AgentPhase>> {
    if request.change_path.is_empty()
        || is_terminal_status(&request.status)
        || is_agent_running_status(&request.status)
        || matches!(
            request.status.as_str(),
            "plan-submitted" | "change-doc-submitted"
        )
    {
        return Ok(None);
    }
    if request.status == "plan-review-rejected" {
        return Ok(Some(AgentPhase::Planning));
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
        AgentPhase::Planning => "plan-review",
        AgentPhase::Implementation => "code-review",
    };
    let attempts = review_attempt_count(request, stage)?;
    Ok(attempts >= max_attempts
        && matches!(
            request.status.as_str(),
            "plan-review-rejected" | "code-review-rejected"
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
    if exit_code != "0" {
        let reason = format!(
            "{} agent exited with code {exit_code}. See {} and {}",
            phase.as_str(),
            agent_stdout_path(&request.request_id).display(),
            agent_stderr_path(&request.request_id).display()
        );
        block_request_by_id(&request.request_id, phase.as_str(), &reason)?;
        return Ok(true);
    }

    match phase {
        AgentPhase::Planning => {
            submit_gate_from_tick(&request.request_id, "plan")?;
            run_plan_review_from_tick(&request.request_id)
        }
        AgentPhase::Implementation => {
            submit_gate_from_tick(&request.request_id, "change-doc")?;
            run_code_review_from_tick(&request.request_id)
        }
    }
}

fn refresh_legacy_agent_status(request: &Request) -> Result<bool> {
    let Some(exit_code) = read_agent_exit_code(&request.request_id)? else {
        return refresh_missing_agent_exit(request, "agent");
    };
    let reason = if exit_code == "0" {
        "legacy issue-agent exited successfully but change-doc approval is missing or stale"
            .to_string()
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
    write_approval_record(
        &request,
        gate,
        "submitted",
        "",
        "outer-tick",
        "submitted by outer tick after agent phase completed",
    )?;
    request.status = format!("{}-submitted", gate_status_prefix(gate));
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(
        &request,
        if gate == "plan" {
            "planning"
        } else {
            "implementation"
        },
        &request.status,
        "submitted by outer tick",
    )?;
    append_event(
        "gate_submitted",
        &request.request_id,
        if gate == "plan" {
            "planning"
        } else {
            "implementation"
        },
        &request.status,
        &format!("gate={gate}; source=outer-tick"),
    )?;
    update_gate_session(&request, gate, "waiting-review")
}

fn run_plan_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match plan_review(&args) {
        Ok(()) => {
            start_worktree(&args)?;
            Ok(true)
        }
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn run_code_review_from_tick(request_id: &str) -> Result<bool> {
    let args = vec!["--request_id".to_string(), request_id.to_string()];
    match code_review(&args) {
        Ok(()) => {
            mark_waiting_finish_by_id(request_id)?;
            Ok(true)
        }
        Err(error) if is_review_terminal_error(&error.to_string()) => Ok(true),
        Err(error) => Err(error),
    }
}

fn is_review_terminal_error(message: &str) -> bool {
    message.contains("rejected plan review")
        || message.contains("rejected code review")
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

fn mark_waiting_finish_by_id(request_id: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if request.status == "waiting-finish" {
        return Ok(());
    }
    ensure_gate_approved(&request, "change-doc")?;
    request.status = "waiting-finish".to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(&request, "implementation", "waiting-finish", "")?;
    upsert_session_for_request(&request, "implementation", "waiting-finish")
}

fn parse_max_attempts(value: Option<String>) -> Result<u32> {
    let Some(value) = value else {
        return Ok(20);
    };
    let parsed = value
        .parse::<u32>()
        .map_err(|_| "--max-attempts must be a positive integer")?;
    if parsed == 0 {
        return Err("--max-attempts must be greater than 0".into());
    }
    Ok(parsed)
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
    if !Path::new(CONFIG_PATH).exists() {
        return Err("not initialized. Run: codex-auto-dev new --url <git-url> or codex-auto-dev new --name <project-name>".into());
    }
    Ok(())
}

fn repo_has_commits(cwd: &str) -> bool {
    git_output(cwd, &["rev-parse", "--verify", "HEAD"]).is_ok()
}

fn codegraph_bin() -> String {
    env::var("CODEX_AUTO_DEV_CODEGRAPH_BIN").unwrap_or_else(|_| "codegraph".to_string())
}

fn codegraph_index_ready(cwd: &str) -> bool {
    Path::new(cwd).join(".codegraph").is_dir()
}

fn ensure_codegraph_initialized(cwd: &str) -> CodegraphInitOutcome {
    if !repo_has_commits(cwd) {
        return CodegraphInitOutcome::SkippedEmptyRepo;
    }
    if codegraph_index_ready(cwd) {
        return CodegraphInitOutcome::AlreadyInitialized;
    }

    let bin = codegraph_bin();
    match Command::new(&bin).args(["init", "-i", cwd]).output() {
        Ok(output) if output.status.success() => {
            if codegraph_index_ready(cwd) {
                CodegraphInitOutcome::Initialized
            } else {
                CodegraphInitOutcome::Failed(format!(
                    "{bin} init -i {cwd} succeeded but {cwd}/.codegraph was not created"
                ))
            }
        }
        Ok(output) => {
            let stderr = review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
            CodegraphInitOutcome::Failed(format!("{bin} init -i {cwd} failed: {stderr}"))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            CodegraphInitOutcome::CommandUnavailable(format!("{bin} unavailable: {error}"))
        }
        Err(error) => CodegraphInitOutcome::Failed(format!("{bin} could not run: {error}")),
    }
}

fn codegraph_preflight_note(outcome: &CodegraphInitOutcome) -> String {
    match outcome {
        CodegraphInitOutcome::SkippedEmptyRepo => "CodeGraph 跳过: 目标仓库为空。".to_string(),
        CodegraphInitOutcome::AlreadyInitialized => {
            "CodeGraph initialized: dev/repo/.codegraph 已存在。".to_string()
        }
        CodegraphInitOutcome::Initialized => {
            "CodeGraph initialized: 已运行 codegraph init -i dev/repo。".to_string()
        }
        CodegraphInitOutcome::CommandUnavailable(detail) => {
            format!("CodeGraph 初始化跳过: {detail}")
        }
        CodegraphInitOutcome::Failed(detail) => {
            format!("CodeGraph 初始化失败: {detail}")
        }
    }
}

fn print_codegraph_init_outcome(prefix: &str, outcome: &CodegraphInitOutcome) {
    println!("{prefix}{}", codegraph_preflight_note(outcome));
}

fn codegraph_event_status(outcome: &CodegraphInitOutcome) -> &'static str {
    match outcome {
        CodegraphInitOutcome::Initialized | CodegraphInitOutcome::AlreadyInitialized => "ready",
        CodegraphInitOutcome::SkippedEmptyRepo => "skipped",
        CodegraphInitOutcome::CommandUnavailable(_) | CodegraphInitOutcome::Failed(_) => "warning",
    }
}

fn codegraph_outcome_detail(outcome: &CodegraphInitOutcome) -> String {
    match outcome {
        CodegraphInitOutcome::SkippedEmptyRepo => "target repo is empty".to_string(),
        CodegraphInitOutcome::AlreadyInitialized => {
            "dev/repo/.codegraph already exists".to_string()
        }
        CodegraphInitOutcome::Initialized => "ran codegraph init -i dev/repo".to_string(),
        CodegraphInitOutcome::CommandUnavailable(detail) | CodegraphInitOutcome::Failed(detail) => {
            detail.clone()
        }
    }
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

fn codegraph_refresh_required() -> Result<bool> {
    if !repo_has_commits(DEV_REPO) {
        return Ok(false);
    }
    let codegraph_path = Path::new("docs/codegraph/context.md");
    if !codegraph_path.exists() {
        return Ok(true);
    }
    let head_timestamp = git_output(DEV_REPO, &["log", "-1", "--format=%ct"])?
        .parse::<u64>()
        .unwrap_or(0);
    let codegraph_timestamp = fs::metadata(codegraph_path)?
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(codegraph_timestamp < head_timestamp)
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

fn spawn_issue_agent(request: &Request, max_attempts: u32, phase: AgentPhase) -> Result<u32> {
    if !Path::new(ISSUE_AGENT_TOOL).exists() {
        return Err(format!("{ISSUE_AGENT_TOOL} does not exist").into());
    }
    fs::create_dir_all(agent_state_dir())?;
    let stdout = fs::File::create(agent_stdout_path(&request.request_id))?;
    let stderr = fs::File::create(agent_stderr_path(&request.request_id))?;
    let exit_path = agent_exit_path(&request.request_id);
    let hook_log_path = agent_hook_log_path(&request.request_id);
    if exit_path.exists() {
        fs::remove_file(&exit_path)?;
    }
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg("tool=$1; exit_path=$2; hook_log=$3; run_hook() { code=$1; if [ -n \"${CODEX_AUTO_DEV_BIN:-}\" ] && [ -n \"${CODEX_AUTO_DEV_REQUEST_ID:-}\" ]; then \"$CODEX_AUTO_DEV_BIN\" advance --request_id \"$CODEX_AUTO_DEV_REQUEST_ID\" --max-attempts \"${CODEX_AUTO_DEV_MAX_ATTEMPTS:-20}\" >> \"$hook_log\" 2>&1 || true; fi; }; write_exit() { code=$1; printf '%s\n' \"$code\" > \"$exit_path\"; run_hook \"$code\"; exit \"$code\"; }; trap 'write_exit 129' HUP; trap 'write_exit 130' INT; trap 'write_exit 143' TERM; sh \"$tool\"; write_exit \"$?\"")
        .arg("codex-auto-dev-agent-wrapper")
        .arg(ISSUE_AGENT_TOOL)
        .arg(&exit_path)
        .arg(&hook_log_path)
        .current_dir(".")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    command.process_group(0);
    apply_issue_agent_env(&mut command, request, max_attempts, phase)?;
    let child = command.spawn()?;
    fs::write(
        agent_pid_path(&request.request_id),
        format!("{}\n", child.id()),
    )?;
    Ok(child.id())
}

fn apply_issue_agent_env(
    command: &mut Command,
    request: &Request,
    max_attempts: u32,
    phase: AgentPhase,
) -> Result<()> {
    let current_exe = env::current_exe()?;
    command
        .env(
            "CODEX_AUTO_DEV_BIN",
            current_exe.to_string_lossy().to_string(),
        )
        .env("CODEX_AUTO_DEV_WORKSPACE", absolute_path_string("."))
        .env("CODEX_AUTO_DEV_TARGET_REPO", absolute_path_string(DEV_REPO))
        .env("CODEX_AUTO_DEV_REQUEST_ID", &request.request_id)
        .env("CODEX_AUTO_DEV_REQUEST_EXTERNAL_ID", &request.external_id)
        .env("CODEX_AUTO_DEV_REQUEST_SOURCE", &request.source)
        .env("CODEX_AUTO_DEV_REQUEST_TITLE", &request.title)
        .env("CODEX_AUTO_DEV_REQUEST_BODY", &request.body)
        .env("CODEX_AUTO_DEV_REQUEST_URL", &request.url)
        .env("CODEX_AUTO_DEV_BRANCH", &request.branch)
        .env(
            "CODEX_AUTO_DEV_WORKTREE",
            absolute_path_string(request.worktree_path.as_str()),
        )
        .env("CODEX_AUTO_DEV_MAX_ATTEMPTS", max_attempts.to_string())
        .env("CODEX_AUTO_DEV_AGENT_PHASE", phase.as_str())
        .env(
            "CODEX_AUTO_DEV_CHANGE_PATH",
            absolute_path_string(request.change_path.as_str()),
        )
        .env(
            "CODEX_AUTO_DEV_REQUEST",
            absolute_path_string(Path::new(&request.change_path).join("request.md")),
        )
        .env(
            "CODEX_AUTO_DEV_PLAN",
            absolute_path_string(Path::new(&request.change_path).join("plan.md")),
        )
        .env(
            "CODEX_AUTO_DEV_CHANGE_DOC",
            absolute_path_string(Path::new(&request.change_path).join("change-doc.md")),
        )
        .env(
            "CODEX_AUTO_DEV_AGENT_JOURNAL",
            absolute_path_string(Path::new(&request.change_path).join("agent-journal.md")),
        )
        .env(
            "CODEX_AUTO_DEV_STATUS",
            absolute_path_string(Path::new(&request.change_path).join("status.json")),
        )
        .env(
            "CODEX_AUTO_DEV_ISSUE_AGENT_SHARED_PROMPT",
            absolute_path_string(ISSUE_AGENT_PROMPT),
        )
        .env(
            "CODEX_AUTO_DEV_ISSUE_AGENT_PROMPT",
            absolute_path_string(phase.prompt_path()),
        )
        .env(
            "CODEX_AUTO_DEV_AGENT_PROMPT",
            absolute_path_string(phase.prompt_path()),
        )
        .envs(proxy_env());
    Ok(())
}

fn read_agent_exit_code(request_id: &str) -> Result<Option<String>> {
    let path = agent_exit_path(request_id);
    if !path.exists() {
        return Ok(None);
    }
    let exit_code = fs::read_to_string(path)?.trim().to_string();
    if exit_code.is_empty() {
        return Ok(None);
    }
    Ok(Some(exit_code))
}

fn read_agent_pid(request_id: &str) -> Result<Option<u32>> {
    let path = agent_pid_path(request_id);
    if !path.exists() {
        return Ok(None);
    }
    let pid = fs::read_to_string(path)?.trim().parse::<u32>().ok();
    Ok(pid)
}

fn process_is_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn agent_state_dir() -> PathBuf {
    Path::new(".codex-auto-dev/state/agents").to_path_buf()
}

fn agent_pid_path(request_id: &str) -> PathBuf {
    agent_state_dir().join(format!("{request_id}.pid"))
}

fn agent_stdout_path(request_id: &str) -> PathBuf {
    agent_state_dir().join(format!("{request_id}.stdout.log"))
}

fn agent_stderr_path(request_id: &str) -> PathBuf {
    agent_state_dir().join(format!("{request_id}.stderr.log"))
}

fn agent_hook_log_path(request_id: &str) -> PathBuf {
    agent_state_dir().join(format!("{request_id}.hook.log"))
}

fn agent_exit_path(request_id: &str) -> PathBuf {
    agent_state_dir().join(format!("{request_id}.exit"))
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
        "plan" | "change-doc" => Ok(()),
        _ => Err("gate must be `plan` or `change-doc`".into()),
    }
}

fn validate_session_phase(phase: &str) -> Result<()> {
    match phase {
        "planning" | "implementation" => Ok(()),
        _ => Err("phase must be `planning` or `implementation`".into()),
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
    let preflight = PlanPreflight {
        notes: vec!["upgrade 迁移生成的模板；正式计划前必须重新运行 plan preflight。".to_string()],
    };
    let artifacts = [
        ("request.md", render_request(request)),
        ("plan.md", render_plan_template(request, &preflight)),
        ("change-doc.md", render_change_doc_template(request)),
        ("agent-journal.md", render_agent_journal_template(request)),
    ];

    for (file, content) in artifacts {
        let path = Path::new(&request.change_path).join(file);
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
    if !status_path.exists() {
        if dry_run {
            println!("Would create {}", status_path.display());
        } else {
            write_status_json(
                request,
                "planning",
                &request.status,
                "upgrade generated status",
            )?;
            println!("Created {}", status_path.display());
        }
    }
    Ok(())
}

fn should_write_managed_artifact(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(true);
    }
    let content = fs::read_to_string(path)?;
    Ok(content.contains("This is a template. Codex")
        || content.contains("This HTML file is a visual planning template")
        || content.contains("Start a new Codex thread")
        || content.contains("# Thread Handoff")
        || content.contains("Codex Plan Prompt")
        || content.contains("Codex Start Prompt")
        || content.contains("agent 每轮")
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
    Err(format!("usage: codex-auto-dev {command}").into())
}

fn print_help() {
    println!(
        "Usage: codex-auto-dev <command>\n\nCommands:\n  new (--url <git-url> | --name <project-name>)\n  update\n  list\n  status [REQ-0001]\n  validate\n  tick [--request_id <REQ-0001>] [--max-attempts 20] [--parallel-limit 1]\n  advance --request_id <REQ-0001> [--max-attempts 20]\n  doctor\n  plan --name <YYYY-MM-DD-short-name> --request_id <REQ-0001>\n  submit --request_id <REQ-0001> --gate <plan|change-doc>\n  approve --request_id <REQ-0001> --gate <plan|change-doc> --by <actor>\n  reject --request_id <REQ-0001> --gate <plan|change-doc> --by <actor>\n  approvals --request_id <REQ-0001> [--json]\n  plan-review --request_id <REQ-0001>\n  code-review --request_id <REQ-0001>\n  start --request_id <REQ-0001>\n  finish --request_id <REQ-0001> [--message \"feat: ...\"]\n  block --request_id <REQ-0001> --stage <stage> --reason <reason>\n  resume --request_id <REQ-0001>\n  session --request_id <REQ-0001> --phase <planning|implementation> [--thread_id <id>] [--thread_url <url>] [--status <status>]\n  sessions [--json]\n  upgrade [--dry-run] [--default]"
    );
}

fn proxy_env() -> Vec<(&'static str, String)> {
    ["https_proxy", "http_proxy", "all_proxy"]
        .iter()
        .filter_map(|key| env::var(key).ok().map(|value| (*key, value)))
        .collect()
}

fn today() -> String {
    if let Ok(output) = Command::new("date").arg("+%F").output()
        && output.status.success()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    "1970-01-01".to_string()
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn fallback_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn absolute_path_string(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn file_sha256(path: &Path) -> Result<String> {
    for (program, args) in [
        ("shasum", vec!["-a", "256"]),
        ("sha256sum", Vec::<&str>::new()),
    ] {
        let output = Command::new(program).args(args).arg(path).output();
        let Ok(output) = output else {
            continue;
        };
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            if let Some(hash) = stdout.split_whitespace().next()
                && !hash.trim().is_empty()
            {
                return Ok(hash.to_string());
            }
        }
    }
    Err("unable to compute sha256: neither shasum nor sha256sum succeeded".into())
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn json_value(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let mut value = after_key[colon_index + 1..].trim_start().chars();
    if value.next()? != '"' {
        return None;
    }
    let mut escaped = false;
    let mut out = String::new();
    for ch in value {
        if escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn json_bool(content: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let value = after_key[colon_index + 1..].trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn json_number(content: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let value = after_key[colon_index + 1..].trim_start();
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u32>().ok()
}

fn review_findings(content: &str, severity: &str) -> Vec<ReviewFinding> {
    let Some(array) = json_array_content(content, severity) else {
        return Vec::new();
    };
    json_objects_in_array(&array)
        .into_iter()
        .map(|object| ReviewFinding {
            title: json_value(&object, "title").unwrap_or_else(|| "未提供标题".to_string()),
            evidence: json_value(&object, "evidence").unwrap_or_else(|| "未提供证据".to_string()),
            impact: json_value(&object, "impact").unwrap_or_else(|| "未提供影响".to_string()),
            required_fix: json_value(&object, "required_fix")
                .unwrap_or_else(|| "未提供必要修复".to_string()),
            suggested_change: json_value(&object, "suggested_change")
                .unwrap_or_else(|| "未提供修改建议".to_string()),
            verification: json_value(&object, "verification")
                .unwrap_or_else(|| "未提供验证方式".to_string()),
        })
        .collect()
}

fn review_has_blocking_findings(content: &str) -> bool {
    json_array_non_empty(content, "critical")
        || json_array_non_empty(content, "high")
        || content.contains("\"severity\":\"critical\"")
        || content.contains("\"severity\": \"critical\"")
        || content.contains("\"severity\":\"high\"")
        || content.contains("\"severity\": \"high\"")
}

fn json_array_content(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let after_colon = after_key[colon_index + 1..].trim_start();
    let rest = after_colon.strip_prefix('[')?;
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut inner = String::new();
    for ch in rest.chars() {
        if escaped {
            inner.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            inner.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            inner.push(ch);
            continue;
        }
        if !in_string {
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth -= 1;
                if depth == 0 {
                    return Some(inner);
                }
            }
        }
        inner.push(ch);
    }
    None
}

fn json_objects_in_array(array: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut started = false;
    for ch in array.chars() {
        if escaped {
            if started {
                current.push(ch);
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            if started {
                current.push(ch);
            }
            escaped = true;
            continue;
        }
        if ch == '"' {
            if started {
                current.push(ch);
            }
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if ch == '{' {
                started = true;
                depth += 1;
                current.push(ch);
                continue;
            }
            if ch == '}' && started {
                current.push(ch);
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    objects.push(current.clone());
                    current.clear();
                    started = false;
                }
                continue;
            }
        }
        if started {
            current.push(ch);
        }
    }
    objects
}

fn json_array_non_empty(content: &str, key: &str) -> bool {
    json_array_content(content, key)
        .map(|inner| !inner.trim().is_empty())
        .unwrap_or(false)
}

fn markdown_inline(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn json_bool_literal(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn ensure_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

fn indent_json_object(content: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    content
        .trim()
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}
