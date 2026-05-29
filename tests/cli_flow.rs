use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const SOURCE_SKILL: &str = include_str!("../skills/codex-auto-dev-workflow/SKILL.md");

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_codex-auto-dev"))
}

fn temp_workspace(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "codex-auto-dev-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp workspace should be created");
    path
}

fn run(workspace: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("codex-auto-dev command should run")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_success(workspace: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("git command should run")
        .status
        .success()
}

fn assert_git_success(workspace: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_installer_can_install_skill_into_codex_home() {
    let codex_home = temp_workspace("codex-home");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("sh")
        .args([
            "scripts/install.sh",
            "--skill-only",
            "--dest",
            codex_home.to_str().expect("temp path should be utf-8"),
        ])
        .current_dir(&repo_root)
        .output()
        .expect("install script should run");
    assert_success(&output);

    let installed_skill = fs::read_to_string(
        codex_home
            .join("skills")
            .join("codex-auto-dev-workflow")
            .join("SKILL.md"),
    )
    .expect("installed skill should be readable");
    assert_eq!(installed_skill, SOURCE_SKILL);
}

#[test]
fn bootstrap_documents_remote_one_command_install() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("sh")
        .args(["scripts/bootstrap.sh", "--help"])
        .current_dir(&repo_root)
        .output()
        .expect("bootstrap script should run");
    assert_success(&output);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("raw.githubusercontent.com/ZhmYe/codex-auto-dev-workflow"));
    assert!(stdout.contains("scripts/install.sh --force"));
}

#[test]
fn skill_requires_install_or_verify_cli_before_workspace_commands() {
    assert!(SOURCE_SKILL.contains("## Required First Step: Install Or Verify CLI"));
    assert!(SOURCE_SKILL.contains("Before any workspace command"));
    assert!(SOURCE_SKILL.contains("codex-auto-dev --help"));
    assert!(SOURCE_SKILL.contains("Do not run workspace commands until"));
    assert!(SOURCE_SKILL.contains("bootstrap.sh | sh"));
}

#[test]
fn new_name_creates_framework_and_empty_target_repo_only() {
    let workspace = temp_workspace("new-name");

    let output = run(&workspace, &["new", "--name", "exact-test-project"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("project name: exact-test-project"));
    assert!(stdout.contains("exact-test-project-auto-dev"));
    assert!(stdout.contains("target git repository name: exact-test-project"));

    assert!(workspace.join("dev/repo/.git").is_dir());
    assert!(workspace.join("tools/issue-update.sh").is_file());
    assert!(
        workspace
            .join("skills/codex-auto-dev-workflow/SKILL.md")
            .is_file()
    );
    let generated_skill = fs::read_to_string(
        workspace
            .join("skills")
            .join("codex-auto-dev-workflow")
            .join("SKILL.md"),
    )
    .expect("generated skill should be readable");
    assert_eq!(generated_skill, SOURCE_SKILL);
    assert!(generated_skill.starts_with("---\nname: codex-auto-dev-workflow"));
    assert!(workspace.join("docs/changes").is_dir());
    assert!(
        !workspace.join("proposal.json").exists(),
        "runtime workspace must not create framework proposal index"
    );
    assert!(
        !git_success(
            &workspace.join("dev/repo"),
            &["rev-parse", "--verify", "HEAD"]
        ),
        "new --name must not create a target repository commit"
    );
    assert!(
        fs::read_dir(workspace.join("docs/changes"))
            .expect("changes dir should exist")
            .next()
            .is_none(),
        "new must not create a change packet or fake plan"
    );
}

#[test]
fn new_url_clones_existing_target_repo() {
    let source = temp_workspace("source-repo");
    assert_git_success(&source, &["init"]);
    assert_git_success(&source, &["checkout", "-B", "main"]);
    assert_git_success(&source, &["config", "user.name", "Test User"]);
    assert_git_success(&source, &["config", "user.email", "test@example.local"]);
    fs::write(source.join("README.md"), "# Source\n").expect("source file should be writable");
    assert_git_success(&source, &["add", "README.md"]);
    assert_git_success(&source, &["commit", "-m", "Initial source"]);

    let workspace = temp_workspace("new-url");
    let source_url = source.to_str().expect("source path should be utf-8");
    let output = run(&workspace, &["new", "--url", source_url]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("workspace naming: arbitrary outer workspace name is OK"));

    assert!(workspace.join("dev/repo/README.md").is_file());
    assert!(git_success(
        &workspace.join("dev/repo"),
        &["rev-parse", "--verify", "HEAD"]
    ));
    assert!(workspace.join("tools/issue-update.sh").is_file());
    assert!(
        workspace
            .join("skills/codex-auto-dev-workflow/SKILL.md")
            .is_file()
    );
}

#[test]
fn update_deduplicates_by_external_id_and_assigns_request_ids() {
    let workspace = temp_workspace("update");
    assert_success(&run(&workspace, &["new", "--name", "update-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst request\\tBody\\thttps://example.test/1\\n'\n",
    )
    .expect("issue tool should be replaceable");

    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(&workspace, &["update"]));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert_eq!(state.matches("REQ-0001").count(), 1);
    assert!(!state.contains("REQ-0002"));
}

#[test]
fn plan_creates_only_templates_for_codex_to_fill() {
    let workspace = temp_workspace("plan");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "plan-test"]));

    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let change_path = workspace.join("docs/changes").join(&change_name);
    for artifact in [
        "issue.md",
        "spec.md",
        "plan.md",
        "tasks.md",
        "plan.html",
        "change-doc.md",
        "codex-plan.md",
        "thread-handoff.md",
    ] {
        assert!(
            change_path.join(artifact).is_file(),
            "missing artifact: {artifact}"
        );
    }

    let plan = fs::read_to_string(change_path.join("plan.md")).expect("plan should be readable");
    let handoff = fs::read_to_string(change_path.join("thread-handoff.md"))
        .expect("handoff should be readable");
    let change_doc = fs::read_to_string(change_path.join("change-doc.md"))
        .expect("change doc should be readable");

    assert!(plan.contains("Codex must fill it"));
    assert!(plan.contains("Project-Internal Requirements"));
    assert!(plan.contains("pre-commit"));
    assert!(plan.contains("AI review"));
    assert!(handoff.contains("Phase: planning only."));
    assert!(handoff.contains("Do not edit target code."));
    assert!(change_doc.contains("This is a template."));
    assert!(change_doc.contains("Target Project Requirements"));
}

#[test]
fn start_creates_worktree_and_implementation_handoff_without_implementing() {
    let workspace = temp_workspace("start");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "start-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));

    let worktree = workspace.join("dev/worktrees/REQ-0001");
    assert!(worktree.is_dir(), "worktree should exist");
    assert!(
        git_success(&worktree, &["status", "--short"]),
        "orphan worktree should be a valid git worktree"
    );

    let change_path = workspace.join("docs/changes").join(change_name);
    let handoff = fs::read_to_string(change_path.join("thread-handoff.md"))
        .expect("thread handoff should be readable");
    let start = fs::read_to_string(change_path.join("codex-start.md"))
        .expect("start prompt should be readable");
    assert!(handoff.contains("Phase: implementation."));
    assert!(handoff.contains("Do not commit or push."));
    assert!(handoff.contains("change-doc path"));
    assert!(handoff.contains("project-internal requirements"));
    assert!(handoff.contains("pre-commit"));
    assert!(handoff.contains("AI review"));
    assert!(start.contains("Do not commit, push, create a PR, or merge."));
}

#[test]
fn finish_marks_status_without_commit_or_push() {
    let workspace = temp_workspace("finish");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "finish-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    let output = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No commit, push, PR, or merge was performed."));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("finished"));
}

fn current_date() -> String {
    let output = Command::new("date")
        .arg("+%F")
        .output()
        .expect("date command should run");
    assert!(output.status.success(), "date command should succeed");
    String::from_utf8(output.stdout)
        .expect("date output should be utf-8")
        .trim()
        .to_string()
}
