use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const CONFIG_PATH: &str = ".codex-auto-dev/config.toml";
const STATE_PATH: &str = ".codex-auto-dev/state/items.tsv";
const DEV_REPO: &str = "dev/repo";
const WORKTREES: &str = "dev/worktrees";
const ISSUE_TOOL: &str = "tools/issue-update.sh";
const WORKFLOW_SKILL: &str = "skills/codex-auto-dev-workflow/SKILL.md";

#[derive(Clone, Debug)]
struct Config {
    git_url: String,
    base_branch: String,
    required_approvals: usize,
}

#[derive(Clone, Debug)]
struct WorkItem {
    id: String,
    external_id: String,
    source: String,
    title: String,
    body: String,
    url: String,
    status: String,
    proposal_path: String,
    branch: String,
    worktree_path: String,
    approvals: Vec<String>,
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
        "init" => init(&args),
        "new" => new_project(&args),
        "update" => update(),
        "request" => request(&args),
        "list" => list(),
        "status" => status(&args),
        "codegraph" => codegraph(&args),
        "plan" => plan(&args),
        "approve" => approve(&args),
        "start" => start(&args),
        "github-create" => github_create(&args),
        "push" => push(&args),
        "tick" => tick(),
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

fn init(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return usage("init <git-url> [base-branch]");
    }

    let git_url = args[0].clone();
    let base_branch = args.get(1).cloned().unwrap_or_else(|| "main".to_string());

    fs::create_dir_all(".codex-auto-dev/state")?;
    fs::create_dir_all("dev")?;
    fs::create_dir_all(WORKTREES)?;
    fs::create_dir_all("docs/proposals")?;
    fs::create_dir_all("tools")?;
    fs::create_dir_all("skills/codex-auto-dev-workflow")?;

    if !Path::new(DEV_REPO).exists() {
        run_command(
            Command::new("git")
                .args(["clone", &git_url, DEV_REPO])
                .envs(proxy_env()),
        )?;
    }

    if !Path::new(CONFIG_PATH).exists() {
        fs::write(
            CONFIG_PATH,
            format!(
                "git_url = \"{}\"\nbase_branch = \"{}\"\nrequired_approvals = 1\n",
                toml_escape(&git_url),
                toml_escape(&base_branch)
            ),
        )?;
    }

    if !Path::new(STATE_PATH).exists() {
        save_items(&[])?;
    }

    write_default_issue_tool()?;
    write_default_workflow_skill()?;
    save_proposal_index(&load_items()?)?;
    if let Err(error) = generate_codegraph_docs(false) {
        eprintln!("codegraph warning: {error}");
    }

    println!("Initialized codex-auto-dev workspace");
    println!("  repo: {DEV_REPO}");
    println!("  issue tool: {ISSUE_TOOL}");
    println!("  workflow skill: {WORKFLOW_SKILL}");
    Ok(())
}

fn new_project(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return usage("new <project-name> [base-branch]");
    }

    let project_name = args[0].clone();
    let base_branch = args.get(1).cloned().unwrap_or_else(|| "main".to_string());
    let git_url = format!("local:{project_name}");

    fs::create_dir_all(".codex-auto-dev/state")?;
    fs::create_dir_all("dev")?;
    fs::create_dir_all(WORKTREES)?;
    fs::create_dir_all("docs/proposals")?;
    fs::create_dir_all("tools")?;
    fs::create_dir_all("skills/codex-auto-dev-workflow")?;

    if !Path::new(DEV_REPO).exists() {
        fs::create_dir_all(DEV_REPO)?;
        run_command(Command::new("git").arg("init").current_dir(DEV_REPO))?;
        run_command(
            Command::new("git")
                .args(["checkout", "-B", &base_branch])
                .current_dir(DEV_REPO),
        )?;
        fs::write(
            Path::new(DEV_REPO).join("README.md"),
            format!("# {project_name}\n\nCreated by codex-auto-dev.\n"),
        )?;
        run_command(
            Command::new("git")
                .args(["config", "user.name", "codex-auto-dev"])
                .current_dir(DEV_REPO),
        )?;
        run_command(
            Command::new("git")
                .args(["config", "user.email", "codex-auto-dev@example.local"])
                .current_dir(DEV_REPO),
        )?;
        run_command(
            Command::new("git")
                .args(["add", "README.md"])
                .current_dir(DEV_REPO),
        )?;
        run_command(
            Command::new("git")
                .args(["commit", "-m", "Initial project scaffold"])
                .current_dir(DEV_REPO),
        )?;
    }

    if !Path::new(CONFIG_PATH).exists() {
        fs::write(
            CONFIG_PATH,
            format!(
                "git_url = \"{}\"\nbase_branch = \"{}\"\nrequired_approvals = 1\n",
                toml_escape(&git_url),
                toml_escape(&base_branch)
            ),
        )?;
    }

    if !Path::new(STATE_PATH).exists() {
        save_items(&[])?;
    }

    write_default_issue_tool()?;
    write_default_workflow_skill()?;
    save_proposal_index(&load_items()?)?;
    if let Err(error) = generate_codegraph_docs(false) {
        eprintln!("codegraph warning: {error}");
    }

    println!("Created new codex-auto-dev project workspace");
    println!("  project: {project_name}");
    println!("  repo: {DEV_REPO}");
    println!("  next: codex-auto-dev request \"Your first requirement\" \"Details...\"");
    Ok(())
}

fn request(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    if args.is_empty() {
        return usage("request <title> [body]");
    }
    let mut items = load_items()?;
    let id = next_id(&items);
    let title = args[0].clone();
    let body = args.get(1..).unwrap_or(&[]).join(" ");
    let now = now_string();
    items.push(WorkItem {
        id: id.clone(),
        external_id: format!("manual:{id}"),
        source: "manual".to_string(),
        title,
        body,
        url: String::new(),
        status: "discovered".to_string(),
        proposal_path: String::new(),
        branch: String::new(),
        worktree_path: String::new(),
        approvals: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    });
    save_items(&items)?;
    println!("Created request {id}");
    Ok(())
}

fn update() -> Result<()> {
    ensure_initialized()?;
    let mut items = load_items()?;
    let output = Command::new("sh").arg(ISSUE_TOOL).output()?;
    if !output.status.success() {
        return Err(format!(
            "{ISSUE_TOOL} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let mut by_external_id = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.external_id.clone(), index))
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
            items[index].source = fields[1].clone();
            items[index].title = fields[2].clone();
            items[index].body = fields[3].clone();
            items[index].url = fields[4].clone();
            items[index].updated_at = now_string();
            updated += 1;
        } else {
            let id = next_id(&items);
            by_external_id.insert(external_id.clone(), items.len());
            items.push(WorkItem {
                id,
                external_id,
                source: fields[1].clone(),
                title: fields[2].clone(),
                body: fields[3].clone(),
                url: fields[4].clone(),
                status: "discovered".to_string(),
                proposal_path: String::new(),
                branch: String::new(),
                worktree_path: String::new(),
                approvals: Vec::new(),
                created_at: now_string(),
                updated_at: now_string(),
            });
            created += 1;
        }
    }

    save_items(&items)?;
    println!("Update complete: {created} new, {updated} refreshed");
    Ok(())
}

fn list() -> Result<()> {
    ensure_initialized()?;
    let items = load_items()?;
    if items.is_empty() {
        println!("No work items yet. Run: codex-auto-dev update");
        return Ok(());
    }

    for item in items {
        println!("{:<8} {:<16} {}", item.id, item.status, item.title);
    }
    Ok(())
}

fn status(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    let items = load_items()?;
    if args.is_empty() {
        let config = load_config()?;
        println!("repo: {}", config.git_url);
        println!("base_branch: {}", config.base_branch);
        let mut counts = BTreeMap::<String, usize>::new();
        for item in items {
            *counts.entry(item.status).or_default() += 1;
        }
        if counts.is_empty() {
            println!("No work items yet.");
        } else {
            for (status, count) in counts {
                println!("{status}: {count}");
            }
        }
        return Ok(());
    }

    let item = find_item(&items, &args[0])?;
    println!("id: {}", item.id);
    println!("external_id: {}", item.external_id);
    println!("source: {}", item.source);
    println!("status: {}", item.status);
    println!("title: {}", item.title);
    println!("url: {}", item.url);
    println!("proposal_path: {}", item.proposal_path);
    println!("branch: {}", item.branch);
    println!("worktree_path: {}", item.worktree_path);
    println!("approvals: {}", item.approvals.join(","));
    Ok(())
}

fn plan(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    if args.is_empty() {
        return usage("plan <id>");
    }

    let mut items = load_items()?;
    let index = find_item_index(&items, &args[0])?;
    let mut item = items[index].clone();
    if item.proposal_path.is_empty() {
        item.proposal_path = format!("docs/proposals/{}/{}", today(), item.id);
    }
    generate_plan_artifacts(&item)?;
    item.status = "plan_ready".to_string();
    item.updated_at = now_string();
    items[index] = item.clone();
    save_items(&items)?;
    save_proposal_index(&items)?;
    println!("Plan ready for {}: {}/plan.md", item.id, item.proposal_path);
    Ok(())
}

fn approve(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    if args.is_empty() {
        return usage("approve <id> [voter]");
    }

    let config = load_config()?;
    let mut items = load_items()?;
    let index = find_item_index(&items, &args[0])?;
    let voter = args.get(1).cloned().unwrap_or_else(|| "local".to_string());
    let item = &mut items[index];

    if item.proposal_path.is_empty() || !Path::new(&item.proposal_path).join("plan.md").exists() {
        return Err(format!(
            "{} has no plan yet. Run: codex-auto-dev plan {}",
            item.id, item.id
        )
        .into());
    }

    if !item.approvals.contains(&voter) {
        item.approvals.push(voter);
    }

    if item.approvals.len() >= config.required_approvals {
        item.status = "approved".to_string();
    }
    item.updated_at = now_string();
    let id = item.id.clone();
    let status = item.status.clone();
    let approvals = item.approvals.len();
    save_items(&items)?;
    save_proposal_index(&items)?;
    println!(
        "{id}: {approvals}/{} approvals, status={status}",
        config.required_approvals
    );
    Ok(())
}

fn start(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    if args.is_empty() {
        return usage("start <id>");
    }

    let config = load_config()?;
    let mut items = load_items()?;
    let index = find_item_index(&items, &args[0])?;
    let mut item = items[index].clone();
    if item.status != "approved"
        && item.status != "in_progress"
        && item.status != "change_doc_ready"
    {
        return Err(format!("{} must be approved before start.", item.id).into());
    }

    if item.proposal_path.is_empty() {
        item.proposal_path = format!("docs/proposals/{}/{}", today(), item.id);
    }

    if should_refresh_codegraph(&item)? {
        if let Err(error) = generate_codegraph_docs(true) {
            eprintln!("codegraph warning: {error}");
        }
    }

    let branch = format!("codex/{}", item.id.to_lowercase());
    let worktree_path = Path::new(WORKTREES).join(&item.id);
    fs::create_dir_all(WORKTREES)?;
    let worktree_string = worktree_path.to_string_lossy().to_string();
    let existing = git_output(DEV_REPO, &["worktree", "list", "--porcelain"])?;
    if !existing.contains(&worktree_string) {
        run_command(
            Command::new("git")
                .args(["fetch", "--all", "--prune"])
                .current_dir(DEV_REPO)
                .envs(proxy_env()),
        )?;
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
                    &worktree_string,
                    &base_ref,
                ])
                .current_dir(DEV_REPO),
        )?;
    }

    item.branch = branch;
    item.worktree_path = worktree_string;
    item.status = "in_progress".to_string();
    item.updated_at = now_string();
    generate_start_instructions(&item)?;
    generate_change_doc(&item)?;
    items[index] = item.clone();
    save_items(&items)?;
    save_proposal_index(&items)?;

    println!("Started {} in {}", item.id, item.worktree_path);
    println!("Codex should now use {WORKFLOW_SKILL} and work only inside this worktree.");
    Ok(())
}

fn codegraph(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    let refresh = args.iter().any(|arg| arg == "--refresh" || arg == "-r");
    generate_codegraph_docs(refresh)?;
    println!("CodeGraph documents updated under docs/codegraph");
    Ok(())
}

fn github_create(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    let repo_name = args.first().cloned().unwrap_or_else(|| repo_dir_name());
    let visibility = if args.iter().any(|arg| arg == "--public") {
        "--public"
    } else {
        "--private"
    };
    let output = Command::new("gh")
        .args([
            "repo", "create", &repo_name, visibility, "--source", DEV_REPO, "--remote", "origin",
        ])
        .envs(proxy_env())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "gh repo create failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    println!("{}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}

fn push(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    let message = if args.is_empty() {
        "Update project".to_string()
    } else {
        args.join(" ")
    };
    run_command(Command::new("git").args(["add", "."]).current_dir(DEV_REPO))?;
    let status = git_output(DEV_REPO, &["status", "--short"])?;
    if status.trim().is_empty() {
        println!("No changes to push.");
        return Ok(());
    }
    run_command(
        Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(DEV_REPO),
    )?;
    run_command(
        Command::new("git")
            .args(["push", "-u", "origin", "HEAD"])
            .current_dir(DEV_REPO)
            .envs(proxy_env()),
    )?;
    println!("Pushed dev repository changes.");
    Ok(())
}

fn tick() -> Result<()> {
    ensure_initialized()?;
    update()?;
    let mut items = load_items()?;
    let discovered: Vec<String> = items
        .iter()
        .filter(|item| item.status == "discovered")
        .map(|item| item.id.clone())
        .collect();
    for id in &discovered {
        plan(&[id.clone()])?;
    }

    items = load_items()?;
    let approved: Vec<String> = items
        .iter()
        .filter(|item| item.status == "approved")
        .map(|item| item.id.clone())
        .collect();

    println!("Tick summary:");
    println!("  planned: {}", discovered.len());
    println!("  approved waiting for start: {}", approved.len());
    for id in approved {
        println!("  next: codex-auto-dev start {id}");
    }
    Ok(())
}

fn validate() -> Result<()> {
    ensure_initialized()?;
    let items = load_items()?;
    for item in items.iter().filter(|item| !item.proposal_path.is_empty()) {
        for file in [
            "spec.md",
            "plan.md",
            "tasks.md",
            "plan.html",
            "change-doc.md",
        ] {
            let path = Path::new(&item.proposal_path).join(file);
            if !path.exists() {
                return Err(
                    format!("{} missing required artifact: {}", item.id, path.display()).into(),
                );
            }
        }
    }
    save_proposal_index(&items)?;
    println!("validated {} work item(s)", items.len());
    Ok(())
}

fn generate_plan_artifacts(item: &WorkItem) -> Result<()> {
    fs::create_dir_all(&item.proposal_path)?;
    fs::write(
        Path::new(&item.proposal_path).join("issue.md"),
        render_issue(item),
    )?;
    fs::write(
        Path::new(&item.proposal_path).join("spec.md"),
        render_spec(item),
    )?;
    fs::write(
        Path::new(&item.proposal_path).join("plan.md"),
        render_plan(item),
    )?;
    fs::write(
        Path::new(&item.proposal_path).join("tasks.md"),
        render_tasks(item),
    )?;
    fs::write(
        Path::new(&item.proposal_path).join("plan.html"),
        render_plan_html(item),
    )?;
    fs::write(
        Path::new(&item.proposal_path).join("change-doc.md"),
        render_pending_change_doc(item),
    )?;
    Ok(())
}

fn generate_codegraph_docs(refresh: bool) -> Result<()> {
    ensure_command("codegraph")?;
    fs::create_dir_all("docs/codegraph")?;
    let codegraph_dir = Path::new(DEV_REPO).join(".codegraph");
    if !codegraph_dir.exists() {
        run_command(
            Command::new("codegraph")
                .args(["init", "-i"])
                .current_dir(DEV_REPO),
        )?;
    } else if refresh {
        run_command(Command::new("codegraph").arg("sync").current_dir(DEV_REPO))?;
    }

    let status = command_output(
        Command::new("codegraph")
            .arg("status")
            .current_dir(DEV_REPO),
    )?;
    let context = command_output(
        Command::new("codegraph")
            .args([
                "context",
                "repository architecture and implementation overview",
            ])
            .current_dir(DEV_REPO),
    )
    .unwrap_or_else(|error| format!("codegraph context failed: {error}"));

    fs::write("docs/codegraph/status.txt", &status)?;
    fs::write(
        "docs/codegraph/context.md",
        render_codegraph_context(&status, &context),
    )?;
    fs::write(
        "docs/codegraph/index.html",
        render_codegraph_html(&status, &context),
    )?;
    Ok(())
}

fn should_refresh_codegraph(item: &WorkItem) -> Result<bool> {
    if !Path::new("docs/codegraph/status.txt").exists() {
        return Ok(true);
    }
    let text = format!("{} {}", item.title.to_lowercase(), item.body.to_lowercase());
    let broad_terms = [
        "architecture",
        "refactor",
        "rewrite",
        "large",
        "migration",
        "cross-cutting",
        "全局",
        "重构",
        "架构",
        "大范围",
        "迁移",
    ];
    Ok(broad_terms.iter().any(|term| text.contains(term)))
}

fn render_codegraph_context(status: &str, context: &str) -> String {
    format!(
        "# CodeGraph Context\n\n## Status\n\n```text\n{}\n```\n\n## Repository Context\n\n```text\n{}\n```\n",
        fallback_empty(status, "No status output."),
        fallback_empty(context, "No context output."),
    )
}

fn render_codegraph_html(status: &str, context: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CodeGraph Context</title>
  <style>
    body {{ margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #172033; background: #f7f9fc; }}
    main {{ max-width: 1040px; margin: 0 auto; padding: 40px 24px; }}
    section {{ margin-bottom: 18px; padding: 24px; border: 1px solid #d9e2ef; border-radius: 8px; background: #fff; }}
    pre {{ white-space: pre-wrap; background: #eef2f7; border-radius: 6px; padding: 16px; overflow: auto; }}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>CodeGraph Context</h1>
      <p>Reusable code understanding generated from the target repository.</p>
    </section>
    <section>
      <h2>Status</h2>
      <pre>{}</pre>
    </section>
    <section>
      <h2>Context</h2>
      <pre>{}</pre>
    </section>
  </main>
</body>
</html>
"#,
        html_escape(fallback_empty(status, "No status output.")),
        html_escape(fallback_empty(context, "No context output.")),
    )
}

fn generate_start_instructions(item: &WorkItem) -> Result<()> {
    fs::create_dir_all(&item.proposal_path)?;
    fs::write(
        Path::new(&item.proposal_path).join("codex-start.md"),
        format!(
            "# Codex Start Instructions: {id}\n\n- Worktree: `{worktree}`\n- Branch: `{branch}`\n- Proposal: `{proposal}`\n\nUse `{skill}`. Read `spec.md`, `plan.md`, and `tasks.md` before editing code. Write code only inside the worktree. Update `change-doc.md` when implementation work changes.\n",
            id = item.id,
            worktree = item.worktree_path,
            branch = item.branch,
            proposal = item.proposal_path,
            skill = WORKFLOW_SKILL,
        ),
    )?;
    Ok(())
}

fn generate_change_doc(item: &WorkItem) -> Result<()> {
    let status = git_output(&item.worktree_path, &["status", "--short"]).unwrap_or_default();
    let changed_files =
        git_output(&item.worktree_path, &["diff", "--name-only"]).unwrap_or_default();
    let diff_stat = git_output(&item.worktree_path, &["diff", "--stat"]).unwrap_or_default();
    fs::write(
        Path::new(&item.proposal_path).join("change-doc.md"),
        format!(
            "# Change Doc: {id}\n\n## Summary\n\n{title}\n\n## Implementation Status\n\nThis work item has been started in an isolated worktree. Codex should update this document as implementation progresses.\n\n## Worktree\n\n- Branch: `{branch}`\n- Path: `{worktree}`\n\n## Current Git Status\n\n```text\n{status}\n```\n\n## Changed Files\n\n```text\n{changed_files}\n```\n\n## Diff Stat\n\n```text\n{diff_stat}\n```\n\n## Validation\n\n- [ ] Run project-specific checks.\n- [ ] Review generated diff.\n- [ ] Confirm proposal acceptance criteria.\n",
            id = item.id,
            title = item.title,
            branch = item.branch,
            worktree = item.worktree_path,
            status = fallback_empty(&status, "No working tree changes detected."),
            changed_files = fallback_empty(&changed_files, "No changed files detected."),
            diff_stat = fallback_empty(&diff_stat, "No diff stat available."),
        ),
    )?;
    Ok(())
}

fn render_issue(item: &WorkItem) -> String {
    format!(
        "# {title}\n\n- ID: {id}\n- External ID: {external_id}\n- Source: {source}\n- URL: {url}\n\n## Body\n\n{body}\n",
        title = item.title,
        id = item.id,
        external_id = item.external_id,
        source = item.source,
        url = fallback_empty(&item.url, "n/a"),
        body = fallback_empty(&item.body, "_No body provided._"),
    )
}

fn render_spec(item: &WorkItem) -> String {
    format!(
        "# Spec: {title}\n\n## User Need\n\n{body}\n\n## Scope\n\n- Analyze the linked work item.\n- Produce an implementation plan before code changes.\n- Keep implementation isolated to this work item's worktree.\n\n## Non-Goals\n\n- Do not modify `dev/repo` directly.\n- Do not create or merge PRs automatically.\n\n## Acceptance Criteria\n\n- [ ] Plan is approved before implementation.\n- [ ] Implementation happens in an isolated worktree.\n- [ ] Change doc records final changes and validation.\n\n## Open Questions\n\n- None recorded yet.\n",
        title = item.title,
        body = fallback_empty(&item.body, "Clarify the requested behavior."),
    )
}

fn render_plan(item: &WorkItem) -> String {
    format!(
        "# Plan: {title}\n\n## Summary\n\nPrepare an isolated implementation for `{id}` after approval.\n\n## Technical Approach\n\n1. Inspect `dev/repo` to understand project structure.\n2. Identify files and tests relevant to the work item.\n3. After approval, create `dev/worktrees/{id}`.\n4. Implement changes only in the isolated worktree.\n5. Update `change-doc.md` with diff and validation details.\n\n## Validation\n\n- [ ] Run relevant project checks from the worktree.\n- [ ] Review `git diff`.\n\n## Risks\n\n- Requirements may need clarification.\n- The default issue tool may need replacing for non-GitHub platforms.\n",
        title = item.title,
        id = item.id,
    )
}

fn render_tasks(item: &WorkItem) -> String {
    format!(
        "# Tasks: {title}\n\n- [ ] Review `issue.md`.\n- [ ] Review `spec.md`.\n- [ ] Review `plan.md`.\n- [ ] Wait for required approval votes.\n- [ ] Run `codex-auto-dev start {id}`.\n- [ ] Implement in the isolated worktree.\n- [ ] Update `change-doc.md`.\n",
        title = item.title,
        id = item.id,
    )
}

fn render_plan_html(item: &WorkItem) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{id} Plan</title>
  <style>
    body {{ margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #172033; background: #f7f9fc; }}
    main {{ max-width: 920px; margin: 0 auto; padding: 40px 24px; }}
    section {{ margin-bottom: 18px; padding: 24px; border: 1px solid #d9e2ef; border-radius: 8px; background: #fff; }}
    h1, h2 {{ margin-top: 0; }}
    pre {{ background: #eef2f7; border-radius: 6px; padding: 16px; overflow: auto; }}
    .badge {{ display: inline-block; padding: 4px 10px; border: 1px solid #9cc2ff; border-radius: 999px; color: #1558b0; background: #edf5ff; }}
  </style>
</head>
<body>
  <main>
    <section>
      <span class="badge">{status}</span>
      <h1>{title}</h1>
      <p>{body}</p>
    </section>
    <section>
      <h2>Flow</h2>
      <pre>Issue -> Plan -> Approval -> Worktree -> Codex implementation -> Change doc</pre>
    </section>
  </main>
</body>
</html>
"#,
        id = html_escape(&item.id),
        status = html_escape(&item.status),
        title = html_escape(&item.title),
        body = html_escape(fallback_empty(&item.body, "No body provided.")),
    )
}

fn render_pending_change_doc(item: &WorkItem) -> String {
    format!(
        "# Change Doc: {id}\n\n## Summary\n\n{title}\n\n## Status\n\nImplementation has not started yet. This document will be updated after approval and worktree execution.\n\n## Validation\n\n- [ ] Not started.\n",
        id = item.id,
        title = item.title,
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
    fs::write(
        WORKFLOW_SKILL,
        r#"# codex-auto-dev-workflow

Use this skill when maintaining a repository managed by `codex-auto-dev`.

## Loop

1. Run `codex-auto-dev status`.
2. Run `codex-auto-dev update` or `codex-auto-dev tick`.
3. For `plan_ready` items, stop and ask for approval.
4. For `approved` items, run `codex-auto-dev start <id>`.
5. Work only inside `dev/worktrees/<id>`.
6. Read `docs/proposals/.../<id>/spec.md`, `plan.md`, and `tasks.md`.
7. Implement the change, run relevant checks, and update `change-doc.md`.

## Boundaries

- Do not edit `dev/repo` directly.
- Do not mix multiple issue implementations in one worktree.
- Do not overwrite user changes.
- Do not create or merge PRs unless explicitly requested.
- If blocked, update `change-doc.md` and report the blocker.
"#,
    )?;
    Ok(())
}

fn load_config() -> Result<Config> {
    ensure_initialized()?;
    let content = fs::read_to_string(CONFIG_PATH)?;
    let mut git_url = String::new();
    let mut base_branch = "main".to_string();
    let mut required_approvals = 1;

    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        match key {
            "git_url" => git_url = value.to_string(),
            "base_branch" => base_branch = value.to_string(),
            "required_approvals" => {
                required_approvals = value.parse().unwrap_or(1);
            }
            _ => {}
        }
    }

    Ok(Config {
        git_url,
        base_branch,
        required_approvals,
    })
}

fn load_items() -> Result<Vec<WorkItem>> {
    if !Path::new(STATE_PATH).exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(STATE_PATH)?;
    let mut items = Vec::new();
    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(unescape_field).collect();
        if fields.len() < 13 {
            continue;
        }
        items.push(WorkItem {
            id: fields[0].clone(),
            external_id: fields[1].clone(),
            source: fields[2].clone(),
            title: fields[3].clone(),
            body: fields[4].clone(),
            url: fields[5].clone(),
            status: fields[6].clone(),
            proposal_path: fields[7].clone(),
            branch: fields[8].clone(),
            worktree_path: fields[9].clone(),
            approvals: fields[10]
                .split('|')
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect(),
            created_at: fields[11].clone(),
            updated_at: fields[12].clone(),
        });
    }
    Ok(items)
}

fn save_items(items: &[WorkItem]) -> Result<()> {
    fs::create_dir_all(".codex-auto-dev/state")?;
    let mut content = String::from("# codex-auto-dev items v1\n");
    for item in items {
        content.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            escape_field(&item.id),
            escape_field(&item.external_id),
            escape_field(&item.source),
            escape_field(&item.title),
            escape_field(&item.body),
            escape_field(&item.url),
            escape_field(&item.status),
            escape_field(&item.proposal_path),
            escape_field(&item.branch),
            escape_field(&item.worktree_path),
            escape_field(&item.approvals.join("|")),
            escape_field(&item.created_at),
            escape_field(&item.updated_at),
        ));
    }
    fs::write(STATE_PATH, content)?;
    Ok(())
}

fn save_proposal_index(items: &[WorkItem]) -> Result<()> {
    let proposals: Vec<&WorkItem> = items
        .iter()
        .filter(|item| !item.proposal_path.is_empty())
        .collect();
    let mut content = format!(
        "{{\n  \"schema_version\": 1,\n  \"updated_at\": \"{}\",\n  \"proposals\": [\n",
        today()
    );
    for (index, item) in proposals.iter().enumerate() {
        let comma = if index + 1 == proposals.len() {
            ""
        } else {
            ","
        };
        content.push_str(&format!(
            "    {{\n      \"id\": \"{}\",\n      \"date\": \"{}\",\n      \"title\": \"{}\",\n      \"status\": \"{}\",\n      \"path\": \"{}\",\n      \"artifacts\": {{\n        \"spec_md\": \"{}/spec.md\",\n        \"plan_md\": \"{}/plan.md\",\n        \"tasks_md\": \"{}/tasks.md\",\n        \"plan_html\": \"{}/plan.html\",\n        \"change_doc_md\": \"{}/change-doc.md\"\n      }}\n    }}{}\n",
            json_escape(&item.id),
            json_escape(&proposal_date(item)),
            json_escape(&item.title),
            json_escape(&item.status),
            json_escape(&item.proposal_path),
            json_escape(&item.proposal_path),
            json_escape(&item.proposal_path),
            json_escape(&item.proposal_path),
            json_escape(&item.proposal_path),
            json_escape(&item.proposal_path),
            comma,
        ));
    }
    content.push_str("  ]\n}\n");
    fs::write("proposal.json", content)?;
    Ok(())
}

fn next_id(items: &[WorkItem]) -> String {
    let next = items
        .iter()
        .filter_map(|item| item.id.strip_prefix("CAD-"))
        .filter_map(|value| value.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    format!("CAD-{next:04}")
}

fn find_item<'a>(items: &'a [WorkItem], id: &str) -> Result<&'a WorkItem> {
    items
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| format!("unknown work item: {id}").into())
}

fn find_item_index(items: &[WorkItem], id: &str) -> Result<usize> {
    items
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| format!("unknown work item: {id}").into())
}

fn ensure_initialized() -> Result<()> {
    if !Path::new(CONFIG_PATH).exists() {
        return Err("not initialized. Run: codex-auto-dev init <git-url>".into());
    }
    Ok(())
}

fn ensure_command(name: &str) -> Result<()> {
    let output = Command::new(name).arg("--version").output();
    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "{name} is installed but failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into()),
        Err(error) => Err(format!("{name} is not available: {error}").into()),
    }
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

fn command_output(command: &mut Command) -> Result<String> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string()
            .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn usage(command: &str) -> Result<()> {
    Err(format!("usage: codex-auto-dev {command}").into())
}

fn print_help() {
    println!(
        "Usage: codex-auto-dev <command>\n\nCommands:\n  init <git-url> [base-branch]\n  new <project-name> [base-branch]\n  update\n  request <title> [body]\n  tick\n  list\n  status [id]\n  codegraph [--refresh]\n  plan <id>\n  approve <id> [voter]\n  start <id>\n  github-create [repo-name] [--public]\n  push [commit-message]\n  validate"
    );
}

fn proxy_env() -> Vec<(&'static str, String)> {
    ["https_proxy", "http_proxy", "all_proxy"]
        .iter()
        .filter_map(|key| env::var(key).ok().map(|value| (*key, value)))
        .collect()
}

fn today() -> String {
    if let Ok(output) = Command::new("date").arg("+%F").output() {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
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

fn proposal_date(item: &WorkItem) -> String {
    item.proposal_path
        .split('/')
        .nth(2)
        .unwrap_or("unknown")
        .to_string()
}

fn repo_dir_name() -> String {
    env::current_dir()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "codex-auto-dev-project".to_string())
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

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
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
