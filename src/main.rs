use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const CONFIG_PATH: &str = ".codex-auto-dev/config.toml";
const STATE_PATH: &str = ".codex-auto-dev/state/requests.tsv";
const DEV_REPO: &str = "dev/repo";
const WORKTREES: &str = "dev/worktrees";
const ISSUE_TOOL: &str = "tools/issue-update.sh";
const WORKFLOW_SKILL: &str = "skills/codex-auto-dev-workflow/SKILL.md";
const WORKFLOW_SKILL_CONTENT: &str = include_str!("../skills/codex-auto-dev-workflow/SKILL.md");

#[derive(Clone, Debug)]
struct Config {
    repo_name: String,
    git_url: String,
    base_branch: String,
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
        "plan" => create_plan_packet(&args),
        "start" => start_worktree(&args),
        "finish" => finish_request(&args),
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
    write_config(&repo_name, git_url, "main")?;
    ensure_state_file()?;
    write_default_issue_tool()?;
    write_default_workflow_skill()?;

    println!("Created codex-auto-dev workspace");
    println!("  mode: clone");
    println!("  workspace naming: arbitrary outer workspace name is OK for cloned repositories");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    println!("  next: codex-auto-dev update");
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
                .args(["checkout", "-B", "main"])
                .current_dir(DEV_REPO),
        )?;
    }
    write_config(repo_name, &format!("local:{repo_name}"), "main")?;
    ensure_state_file()?;
    write_default_issue_tool()?;
    write_default_workflow_skill()?;

    println!("Created codex-auto-dev workspace");
    println!("  mode: empty");
    println!("  project name: {repo_name}");
    println!("  workspace naming: use an outer workspace directory named {repo_name}-auto-dev");
    println!("  target git repository name: {repo_name}");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    println!(
        "  next: codex-auto-dev plan --name {}-initial-plan --request_id REQ-0001",
        today()
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
            updated += 1;
        } else {
            let request_id = next_request_id(&requests);
            by_external_id.insert(external_id.clone(), requests.len());
            requests.push(Request {
                request_id,
                external_id,
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

fn create_plan_packet(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--name", "--request_id", "--request-id"])?;
    let change_name = required_flag(args, "--name")?;
    let request_id = required_request_id(args)?;
    validate_change_name(&change_name)?;

    let mut requests = load_requests()?;
    let index = match find_request_index(&requests, &request_id) {
        Some(index) => index,
        None => {
            requests.push(manual_request(&request_id, &change_name));
            requests.len() - 1
        }
    };

    let mut request = requests[index].clone();
    request.change_name = change_name.clone();
    request.change_path = format!("docs/changes/{change_name}");
    request.status = "planning".to_string();
    request.updated_at = now_string();
    generate_plan_packet(&request)?;
    requests[index] = request.clone();
    save_requests(&requests)?;

    println!("Planning packet ready for {}", request.request_id);
    println!("  change path: {}", request.change_path);
    println!("  plan template: {}/plan.md", request.change_path);
    println!("  handoff: {}/thread-handoff.md", request.change_path);
    println!("  Codex must fill the templates and stop for plan approval.");
    Ok(())
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

    let branch = format!("codex/{}", request.request_id.to_lowercase());
    let worktree_path = Path::new(WORKTREES).join(&request.request_id);
    fs::create_dir_all(WORKTREES)?;
    let absolute_worktree = env::current_dir()?.join(&worktree_path);
    let absolute_worktree_string = absolute_worktree.to_string_lossy().to_string();

    let existing = git_output(DEV_REPO, &["worktree", "list", "--porcelain"])?;
    if !existing.contains(&format!("worktree {absolute_worktree_string}")) {
        fetch_if_remote_exists()?;
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
            run_command(
                Command::new("git")
                    .args([
                        "worktree",
                        "add",
                        "--orphan",
                        "-B",
                        &branch,
                        &absolute_worktree_string,
                    ])
                    .current_dir(DEV_REPO),
            )?;
        }
    }

    request.branch = branch;
    request.worktree_path = worktree_path.to_string_lossy().to_string();
    request.status = "in-progress".to_string();
    request.updated_at = now_string();
    generate_start_packet(&request)?;
    requests[index] = request.clone();
    save_requests(&requests)?;

    println!("Worktree ready for {}", request.request_id);
    println!("  worktree: {}", request.worktree_path);
    println!("  branch: {}", request.branch);
    println!(
        "  start instructions: {}/codex-start.md",
        request.change_path
    );
    println!("  Codex must implement in the worktree and stop for change-doc approval.");
    Ok(())
}

fn finish_request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    let request_id = required_request_id(args)?;
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, &request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let request = &mut requests[index];
    request.status = "finished".to_string();
    request.updated_at = now_string();
    let change_path = request.change_path.clone();
    let worktree_path = request.worktree_path.clone();
    let branch = request.branch.clone();
    save_requests(&requests)?;

    println!("{request_id} marked finished.");
    println!("  change doc: {change_path}/change-doc.md");
    println!("  worktree: {worktree_path}");
    println!("  branch: {branch}");
    println!("  No commit, push, PR, or merge was performed.");
    Ok(())
}

fn list_requests() -> Result<()> {
    ensure_initialized()?;
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
            "issue.md",
            "spec.md",
            "plan.md",
            "tasks.md",
            "plan.html",
            "change-doc.md",
            "codex-plan.md",
            "thread-handoff.md",
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
    fs::write(
        CONFIG_PATH,
        format!(
            "repo_name = \"{}\"\ngit_url = \"{}\"\nbase_branch = \"{}\"\n",
            toml_escape(repo_name),
            toml_escape(git_url),
            toml_escape(base_branch)
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

fn generate_plan_packet(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    fs::write(
        Path::new(&request.change_path).join("issue.md"),
        render_issue(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("spec.md"),
        render_spec_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("plan.md"),
        render_plan_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("tasks.md"),
        render_tasks_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("plan.html"),
        render_plan_html_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("change-doc.md"),
        render_change_doc_template(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("codex-plan.md"),
        render_codex_plan_prompt(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("thread-handoff.md"),
        render_planning_handoff(request),
    )?;
    Ok(())
}

fn generate_start_packet(request: &Request) -> Result<()> {
    fs::create_dir_all(&request.change_path)?;
    fs::write(
        Path::new(&request.change_path).join("codex-start.md"),
        render_codex_start_prompt(request),
    )?;
    fs::write(
        Path::new(&request.change_path).join("thread-handoff.md"),
        render_implementation_handoff(request),
    )?;
    Ok(())
}

fn render_issue(request: &Request) -> String {
    format!(
        "# Request {request_id}: {title}\n\n- Request ID: `{request_id}`\n- External ID: `{external_id}`\n- Source: `{source}`\n- URL: {url}\n\n## Original Request\n\n{body}\n",
        request_id = request.request_id,
        title = request.title,
        external_id = request.external_id,
        source = request.source,
        url = fallback_empty(&request.url, "n/a"),
        body = fallback_empty(
            &request.body,
            "Codex should fill the concrete request from the user conversation or issue source."
        ),
    )
}

fn render_spec_template(request: &Request) -> String {
    format!(
        "# Spec: {title}\n\nThis is a template. Codex must replace placeholders after reading the request, target repository, CodeGraph docs when present, and target project documentation.\n\n## User Need\n\nFill in the concrete user need here.\n\n## Goals And Dependencies\n\nFill in observable goals, dependency order, and acceptance signals here.\n\n## Target Project Requirements\n\nFill in project-internal requirements discovered from README, CONTRIBUTING, AGENTS, docs, scripts, pre-commit config, and AI review process.\n\n## Acceptance Criteria\n\nFill in testable acceptance criteria here.\n",
        title = request.title,
    )
}

fn render_plan_template(request: &Request) -> String {
    format!(
        "# Plan: {title}\n\nThis is a planning template. Codex must fill it; `codex-auto-dev` does not generate the real plan.\n\n## Goal Dependency Graph\n\nFill in the ordered goals and dependencies here.\n\n## Repository Analysis\n\nFill in files, modules, existing patterns, and relevant target project documentation read.\n\n## Project-Internal Requirements\n\nFill in target project change doc, pre-commit, documentation checks, format/lint/test commands, and AI review requirements.\n\n## Planned Code Changes\n\nFill in exact files/modules and intended changes here.\n\n## Testing And Verification\n\nFill in unit, integration, negative, regression, security, pre-commit, documentation check, and AI review plan here.\n\n## Approval Gate\n\nStop after filling this plan and wait for user approval before running `codex-auto-dev start --request_id {request_id}`.\n",
        title = request.title,
        request_id = request.request_id,
    )
}

fn render_tasks_template(request: &Request) -> String {
    format!(
        "# Tasks: {title}\n\nThis is a task template. Codex must replace it with concrete steps during planning.\n\n## Planning\n\n- [ ] Read `issue.md`.\n- [ ] Read target project documentation.\n- [ ] Fill `spec.md` and `plan.md` with concrete details.\n- [ ] Identify project-internal requirements.\n- [ ] Stop for user approval.\n\n## Implementation\n\n- [ ] Run `codex-auto-dev start --request_id {request_id}` only after plan approval.\n- [ ] Work only in the generated worktree.\n- [ ] Implement the approved plan.\n- [ ] Complete target project change docs when required.\n- [ ] Run required pre-commit, documentation checks, tests, and AI review.\n- [ ] Fill `change-doc.md` and stop for user approval.\n",
        title = request.title,
        request_id = request.request_id,
    )
}

fn render_plan_html_template(request: &Request) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{request_id} Plan Template</title>
  <style>
    body {{ margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #172033; background: #f7f9fc; }}
    main {{ max-width: 920px; margin: 0 auto; padding: 40px 24px; }}
    section {{ margin-bottom: 18px; padding: 24px; border: 1px solid #d9e2ef; border-radius: 8px; background: #fff; }}
    h1, h2 {{ margin-top: 0; }}
    code {{ background: #eef2f7; padding: 2px 5px; border-radius: 4px; }}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>{title}</h1>
      <p>This HTML file is a visual planning template. Codex fills the Markdown plan; this file marks where the human should review the plan.</p>
    </section>
    <section>
      <h2>Gate</h2>
      <p>Do not run <code>codex-auto-dev start --request_id {request_id}</code> until the user approves the completed plan.</p>
    </section>
  </main>
</body>
</html>
"#,
        request_id = html_escape(&request.request_id),
        title = html_escape(&request.title),
    )
}

fn render_change_doc_template(request: &Request) -> String {
    format!(
        "# Change Doc: {request_id}\n\nThis is a template. Codex must fill it after implementation and before asking for approval.\n\n## Summary\n\nFill in actual behavior and code changes here.\n\n## Files Changed\n\nFill in changed files here.\n\n## Target Project Requirements\n\n- Target project documentation read: fill in documents read.\n- Target project change doc: fill in path or `Not required`.\n- Pre-commit: fill in command and result or `Not required`.\n- Documentation checks: fill in commands and results or `Not required`.\n- Format/lint/test checks: fill in commands and results.\n- AI review: fill in findings, resolution status, or `Not required`.\n- All project-internal requirements completed: fill in yes/no with blockers.\n\n## Validation Evidence\n\nFill in exact commands and results here.\n\n## Approval Gate\n\nStop after filling this document and wait for user approval before running `codex-auto-dev finish --request_id {request_id}` or any commit/push/PR action.\n",
        request_id = request.request_id,
    )
}

fn render_codex_plan_prompt(request: &Request) -> String {
    format!(
        "# Codex Plan Prompt: {request_id}\n\nUse `skills/codex-auto-dev-workflow/SKILL.md`.\n\nYou are in planning only. `codex-auto-dev` created templates; you must fill them.\n\nRead `{change_path}/issue.md`, inspect `dev/repo`, read target project documentation, then fill `{change_path}/spec.md`, `{change_path}/plan.md`, and `{change_path}/tasks.md`.\n\nDo not write target code. Do not run `codex-auto-dev start`. Do not commit or push. Stop and give the user `{change_path}/plan.md` for approval.\n",
        request_id = request.request_id,
        change_path = request.change_path,
    )
}

fn render_planning_handoff(request: &Request) -> String {
    format!(
        "# Thread Handoff: Planning {request_id}\n\nStart a new Codex thread with this prompt:\n\n```text\nUse skills/codex-auto-dev-workflow/SKILL.md.\nWorkspace: the current codex-auto-dev workspace.\nRequest ID: {request_id}.\nPhase: planning only.\nRead {change_path}/issue.md, spec.md, plan.md, tasks.md, and docs/codegraph/context.md if present.\nInspect dev/repo and read target project documentation.\nFill the templates with a concrete implementation plan, including project-internal requirements, target project change doc, pre-commit, checks, tests, AI review, risks, and rollback.\nDo not edit target code. Do not run start, finish, commit, push, or PR commands.\nWhen done, stop and give me {change_path}/plan.md for approval.\n```\n",
        request_id = request.request_id,
        change_path = request.change_path,
    )
}

fn render_codex_start_prompt(request: &Request) -> String {
    format!(
        "# Codex Start Prompt: {request_id}\n\nUse `skills/codex-auto-dev-workflow/SKILL.md`.\n\nWorktree: `{worktree}`\nBranch: `{branch}`\nChange path: `{change_path}`\n\nThe user has approved the plan. Implement only inside the worktree. Re-read target project documentation, follow the approved plan, satisfy project-internal requirements, run required checks and AI review, then fill `{change_path}/change-doc.md` and stop for approval.\n\nDo not commit, push, create a PR, or merge.\n",
        request_id = request.request_id,
        worktree = request.worktree_path,
        branch = request.branch,
        change_path = request.change_path,
    )
}

fn render_implementation_handoff(request: &Request) -> String {
    format!(
        "# Thread Handoff: Implementation {request_id}\n\nStart a new Codex thread with this prompt after plan approval:\n\n```text\nUse skills/codex-auto-dev-workflow/SKILL.md.\nWorkspace: the current codex-auto-dev workspace.\nRequest ID: {request_id}.\nPhase: implementation.\nWork only inside {worktree}.\nRead {change_path}/issue.md, spec.md, plan.md, tasks.md, codex-start.md, and docs/codegraph/context.md if present.\nRe-read target project documentation and satisfy all project-internal requirements from the approved plan.\nImplement exactly the approved plan. Do not edit dev/repo directly. Do not commit or push.\nRun required checks and tests, including target project pre-commit, documentation checks, and AI review when required.\nUpdate {change_path}/change-doc.md with files changed, target project change doc path, completed requirements, validation evidence, AI review findings and resolutions, risks, and follow-ups.\nWhen done, stop and give me the change-doc path for approval.\n```\n",
        request_id = request.request_id,
        change_path = request.change_path,
        worktree = request.worktree_path,
    )
}

fn write_default_issue_tool() -> Result<()> {
    if Path::new(ISSUE_TOOL).exists() {
        return Ok(());
    }
    fs::write(
        ISSUE_TOOL,
        r##"#!/usr/bin/env sh
set -eu

cd dev/repo

# Output TSV lines:
# external_id<TAB>source<TAB>title<TAB>body<TAB>url
#
# Replace this script for Jira, Linear, internal workspaces, or other sources.
# The connector should emit a stable external_id so repeated updates do not
# create duplicate requests.

repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
gh api "repos/${repo}/issues" -f state=open --jq '.[] | select(.pull_request == null) | ["github:" + "'${repo}'" + "#" + (.number|tostring), "github", .title, (.body // ""), .html_url] | @tsv'
"##,
    )?;
    let mut permissions = fs::metadata(ISSUE_TOOL)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(ISSUE_TOOL, permissions)?;
    Ok(())
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
    let mut repo_name = String::new();
    let mut git_url = String::new();
    let mut base_branch = "main".to_string();

    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "repo_name" => repo_name = value.to_string(),
            "git_url" => git_url = value.to_string(),
            "base_branch" => base_branch = value.to_string(),
            _ => {}
        }
    }

    Ok(Config {
        repo_name,
        git_url,
        base_branch,
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
        "Usage: codex-auto-dev <command>\n\nCommands:\n  new (--url <git-url> | --name <project-name>)\n  update\n  plan --name <YYYY-MM-DD-short-name> --request_id <REQ-0001>\n  start --request_id <REQ-0001>\n  finish --request_id <REQ-0001>"
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

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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
