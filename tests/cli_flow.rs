use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

fn run_with_env(workspace: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command.args(args).current_dir(workspace);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("codex-auto-dev command should run")
}

fn run_cad_alias(workspace: &Path, args: &[&str]) -> Output {
    let alias = std::env::var_os("CARGO_BIN_EXE_cad").expect("cad alias should be built");
    Command::new(alias)
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("cad command should run")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &Output, expected_stderr: &str) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_stderr),
        "stderr did not contain expected text\nexpected:\n{}\nstdout:\n{}\nstderr:\n{}",
        expected_stderr,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn install_fake_codegraph(fake_bin: &Path) -> String {
    fs::create_dir_all(fake_bin).expect("fake bin should be writable");
    let script = fake_bin.join("codegraph");
    fs::write(
        &script,
        "#!/usr/bin/env sh\nset -eu\nprintf '%s\\n' \"$*\" >> \"$CODEX_TEST_CODEGRAPH_LOG\"\nif [ \"${1:-}\" = \"init\" ]; then\n  target=\"${3:-.}\"\n  mkdir -p \"$target/.codegraph\"\nfi\n",
    )
    .expect("fake codegraph should be writable");
    let mut permissions = fs::metadata(&script)
        .expect("fake codegraph metadata readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).expect("fake codegraph executable");
    let original_path = std::env::var("PATH").unwrap_or_default();
    format!("{}:{original_path}", fake_bin.display())
}

fn assert_workspace_files_equal(workspace: &Path, left: &str, right: &str) {
    let left_content =
        fs::read_to_string(workspace.join(left)).unwrap_or_else(|_| panic!("{left} readable"));
    let right_content =
        fs::read_to_string(workspace.join(right)).unwrap_or_else(|_| panic!("{right} readable"));
    assert_eq!(left_content, right_content, "{left} should match {right}");
}

fn force_request_state(
    workspace: &Path,
    request_id: &str,
    status: &str,
    branch: &str,
    worktree: &str,
) {
    let state_path = workspace.join(".codex-auto-dev/state/requests.tsv");
    let content = fs::read_to_string(&state_path).expect("requests state should be readable");
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.starts_with(request_id) {
            let mut fields = line.split('\t').map(str::to_string).collect::<Vec<_>>();
            assert!(fields.len() >= 11, "request state line should have fields");
            fields[6] = status.to_string();
            fields[9] = branch.to_string();
            fields[10] = worktree.to_string();
            lines.push(fields.join("\t"));
        } else {
            lines.push(line.to_string());
        }
    }
    fs::write(&state_path, format!("{}\n", lines.join("\n")))
        .expect("requests state should be writable");
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

fn git_output(workspace: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout)
        .expect("git stdout should be utf-8")
        .trim()
        .to_string()
}

fn wait_for_file(path: &Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for file: {}", path.display());
}

fn wait_for_file_contains(path: &Path, expected: &str) {
    for _ in 0..100 {
        if path.exists() {
            let content = fs::read_to_string(path).expect("waited file should be readable");
            if content.contains(expected) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "timed out waiting for file content: {} should contain {}",
        path.display(),
        expected
    );
}

fn create_bare_origin_with_master(label: &str) -> PathBuf {
    let origin_parent = temp_workspace(label);
    let origin = origin_parent.join("origin.git");
    fs::create_dir_all(&origin).expect("origin dir should be created");
    assert_git_success(&origin, &["init", "--bare"]);

    let seed = temp_workspace(&format!("{label}-seed"));
    assert_git_success(&seed, &["init"]);
    assert_git_success(&seed, &["checkout", "-B", "master"]);
    assert_git_success(&seed, &["config", "user.name", "Test User"]);
    assert_git_success(&seed, &["config", "user.email", "test@example.local"]);
    fs::write(seed.join("README.md"), "# Source\n").expect("source file should be writable");
    assert_git_success(&seed, &["add", "README.md"]);
    assert_git_success(&seed, &["commit", "-m", "Initial source"]);
    assert_git_success(
        &seed,
        &["remote", "add", "origin", origin.to_str().expect("utf-8")],
    );
    assert_git_success(&seed, &["push", "-u", "origin", "master"]);
    assert_git_success(&origin, &["symbolic-ref", "HEAD", "refs/heads/master"]);
    origin
}

#[test]
fn help_lists_state_and_validation_commands() {
    let workspace = temp_workspace("help-commands");
    let output = run(&workspace, &["--help"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("  list"));
    assert!(stdout.contains("  dashboard"));
    assert!(stdout.contains("  status [REQ-0001]"));
    assert!(stdout.contains("  validate"));
    assert!(stdout.contains("  pr-status --request_id <REQ-0001>"));
    assert!(stdout.contains("  pr-refresh --request_id <REQ-0001>"));
    assert!(stdout.contains("  integration-review --request_id <REQ-0001>"));
}

#[test]
fn cad_alias_prints_the_same_cli_help() {
    let workspace = temp_workspace("cad-alias");
    let output = run_cad_alias(&workspace, &["--help"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: codex-auto-dev <command>"));
    assert!(stdout.contains("dashboard"));
}

#[test]
fn new_refuses_to_initialize_the_framework_source_checkout() {
    let workspace = temp_workspace("self-guard");
    fs::create_dir_all(workspace.join("src")).expect("src dir writable");
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"codex-auto-dev-workflow\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("Cargo.toml writable");
    fs::write(workspace.join("src/main.rs"), "fn main() {}\n").expect("main.rs writable");
    fs::create_dir_all(workspace.join("templates")).expect("templates dir writable");
    fs::create_dir_all(workspace.join("skills/codex-auto-dev-workflow"))
        .expect("skill dir writable");

    let output = run(&workspace, &["new", "--name", "should-not-create-dev"]);

    assert_failure_contains(
        &output,
        "refusing to initialize codex-auto-dev source checkout",
    );
    assert!(!workspace.join("dev").exists());
    assert!(!workspace.join(".codex-auto-dev").exists());
    assert!(!workspace.join("tools").exists());
}

#[test]
fn templates_are_external_assets_not_embedded_in_main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "assets/dashboard/index.html",
        "templates/prompts/issue-agent.md",
        "templates/prompts/plan-agent.md",
        "templates/prompts/implementation-agent.md",
        "templates/prompts/rebase-agent.md",
        "templates/prompts/plan-reviewer.md",
        "templates/prompts/test-reviewer.md",
        "templates/prompts/design-reviewer.md",
        "templates/prompts/integration-reviewer.md",
        "templates/runtime/plan.md",
        "templates/runtime/change-doc.md",
        "templates/schemas/review-result.schema.json",
        "templates/scripts/issue-update.sh",
        "templates/scripts/issue-agent.sh",
        "templates/scripts/rebase-agent.sh",
        "templates/scripts/review-tool.sh",
        "templates/scripts/pr-create.sh",
        "templates/scripts/pr-status.sh",
        "src/assets.rs",
        "src/dashboard.rs",
        "src/delivery.rs",
        "src/defaults.rs",
        "src/doctor.rs",
        "src/registry.rs",
        "src/review_gate.rs",
        "src/state.rs",
        "src/utils.rs",
    ] {
        assert!(root.join(path).is_file(), "missing template asset: {path}");
    }

    let main_source =
        fs::read_to_string(root.join("src/main.rs")).expect("main.rs should be readable");
    assert!(
        main_source.lines().count() < 4500,
        "main.rs should stay small enough to be navigable"
    );
    assert!(!main_source.contains("<!doctype html>"));
    assert!(!main_source.contains("# PlanReviewer 严格审查提示词"));
    assert!(!main_source.contains("# TestReviewer 严格审查提示词"));
    assert!(!main_source.contains("# DesignReviewer 严格审查提示词"));
    assert!(!main_source.contains("# Issue Agent 共享 agent 契约"));
    assert!(!main_source.contains("fn default_review_tool_content"));
    assert!(!main_source.contains("fn deliver_finished_request"));
    assert!(!main_source.contains("fn doctor_command_check"));
    assert!(!main_source.contains("fn load_requests"));
    assert!(!main_source.contains("fn run_single_reviewer"));
    assert!(!main_source.contains("fn json_objects_in_array"));
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
    assert!(SOURCE_SKILL.contains("## 必做第一步: 安装或验证 CLI"));
    assert!(SOURCE_SKILL.contains("Before any workspace command"));
    assert!(SOURCE_SKILL.contains("codex-auto-dev --help"));
    assert!(SOURCE_SKILL.contains("Do not run workspace commands until"));
    assert!(SOURCE_SKILL.contains("bootstrap.sh | sh"));
    assert!(SOURCE_SKILL.contains("--parallel-limit"));
    assert!(SOURCE_SKILL.contains("parallel_limit = 1"));
    assert!(SOURCE_SKILL.contains("CODEX_AUTO_DEV_REVIEW_CONTEXT"));
    assert!(SOURCE_SKILL.contains("不得读取其他 reviewer 输出"));
    assert!(SOURCE_SKILL.contains("审批是显式文件化门禁"));
    assert!(SOURCE_SKILL.contains("交付文档中的 checklist 必须全部打勾"));
    assert!(SOURCE_SKILL.contains("无法由当前流程完成的事项不得保留为未勾选 checklist"));
    assert!(SOURCE_SKILL.contains("codex-auto-dev upgrade --dry-run"));
    assert!(SOURCE_SKILL.contains("codex-auto-dev upgrade --default"));
    assert!(SOURCE_SKILL.contains("workspaces.json"));
    assert!(SOURCE_SKILL.contains("codex-auto-dev dashboard"));
    assert!(SOURCE_SKILL.contains("cad dashboard"));
    assert!(SOURCE_SKILL.contains("Request -> Plan -> Plan Review"));
    assert!(SOURCE_SKILL.contains("不要依赖会被覆盖的 `summary.json`"));
    assert!(SOURCE_SKILL.contains("纵向列表"));
    assert!(SOURCE_SKILL.contains("marked"));
    assert!(SOURCE_SKILL.contains("jsoneditor"));
    assert!(SOURCE_SKILL.contains("PlanReviewer 提交前自检"));
    assert!(SOURCE_SKILL.contains("Code Review 提交前自检"));
    assert!(SOURCE_SKILL.contains("src/registry.rs"));
    assert!(SOURCE_SKILL.contains("src/state.rs"));
    assert!(SOURCE_SKILL.contains("src/review_gate.rs"));
    assert!(SOURCE_SKILL.contains("src/defaults.rs"));
    assert!(SOURCE_SKILL.contains("templates/prompts/*.md"));
    assert!(SOURCE_SKILL.contains("RebaseAgent"));
    assert!(SOURCE_SKILL.contains("IntegrationReviewer"));
    assert!(SOURCE_SKILL.contains("不能为了自己分支的修改删除 base/master 新代码"));
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
    let config = fs::read_to_string(workspace.join(".codex-auto-dev/config.toml"))
        .expect("config should be readable");
    assert!(config.contains("parallel_limit = 1"));

    assert!(workspace.join("dev/repo/.git").is_dir());
    assert!(workspace.join("tools/issue-update.sh").is_file());
    assert!(workspace.join("tools/pr-create.sh").is_file());
    assert!(workspace.join("tools/plan-review.sh").is_file());
    assert!(workspace.join("tools/test-review.sh").is_file());
    assert!(workspace.join("tools/design-review.sh").is_file());
    assert!(workspace.join("tools/integration-review.sh").is_file());
    assert!(workspace.join("tools/issue-agent.sh").is_file());
    assert!(workspace.join("tools/rebase-agent.sh").is_file());
    assert!(workspace.join("tools/pr-status.sh").is_file());
    assert!(workspace.join("tools/prompts/plan-reviewer.md").is_file());
    assert!(workspace.join("tools/prompts/test-reviewer.md").is_file());
    assert!(workspace.join("tools/prompts/design-reviewer.md").is_file());
    assert!(
        workspace
            .join("tools/prompts/integration-reviewer.md")
            .is_file()
    );
    assert!(workspace.join("tools/prompts/issue-agent.md").is_file());
    assert!(workspace.join("tools/prompts/plan-agent.md").is_file());
    assert!(
        workspace
            .join("tools/prompts/implementation-agent.md")
            .is_file()
    );
    assert!(workspace.join("tools/prompts/rebase-agent.md").is_file());
    assert!(
        workspace
            .join("tools/schemas/review-result.schema.json")
            .is_file()
    );
    for (target, example) in [
        ("tools/issue-update.sh", "tools/issue-update.example.sh"),
        ("tools/issue-agent.sh", "tools/issue-agent.example.sh"),
        ("tools/rebase-agent.sh", "tools/rebase-agent.example.sh"),
        ("tools/pr-create.sh", "tools/pr-create.example.sh"),
        ("tools/pr-status.sh", "tools/pr-status.example.sh"),
        ("tools/plan-review.sh", "tools/plan-review.example.sh"),
        ("tools/test-review.sh", "tools/test-review.example.sh"),
        ("tools/design-review.sh", "tools/design-review.example.sh"),
        (
            "tools/integration-review.sh",
            "tools/integration-review.example.sh",
        ),
        (
            "tools/prompts/issue-agent.md",
            "tools/prompts/issue-agent.example.md",
        ),
        (
            "tools/prompts/plan-agent.md",
            "tools/prompts/plan-agent.example.md",
        ),
        (
            "tools/prompts/implementation-agent.md",
            "tools/prompts/implementation-agent.example.md",
        ),
        (
            "tools/prompts/rebase-agent.md",
            "tools/prompts/rebase-agent.example.md",
        ),
        (
            "tools/prompts/plan-reviewer.md",
            "tools/prompts/plan-reviewer.example.md",
        ),
        (
            "tools/prompts/test-reviewer.md",
            "tools/prompts/test-reviewer.example.md",
        ),
        (
            "tools/prompts/design-reviewer.md",
            "tools/prompts/design-reviewer.example.md",
        ),
        (
            "tools/prompts/integration-reviewer.md",
            "tools/prompts/integration-reviewer.example.md",
        ),
        (
            "tools/schemas/review-result.schema.json",
            "tools/schemas/review-result.example.schema.json",
        ),
    ] {
        assert_workspace_files_equal(&workspace, target, example);
    }
    let issue_tool =
        fs::read_to_string(workspace.join("tools/issue-update.sh")).expect("issue tool readable");
    assert!(issue_tool.contains("--method GET"));
    assert!(issue_tool.contains("--paginate"));
    assert!(issue_tool.contains("(.body //"));
    assert!(issue_tool.contains("Connector contract"));
    assert!(issue_tool.contains("external_id<TAB>source<TAB>title<TAB>body<TAB>url"));
    assert!(issue_tool.contains("normalized requirement name"));
    let issue_agent =
        fs::read_to_string(workspace.join("tools/issue-agent.sh")).expect("agent tool readable");
    assert!(issue_agent.contains("Connector contract"));
    assert!(issue_agent.contains("CODEX_AUTO_DEV_REQUEST_SOURCE"));
    assert!(issue_agent.contains("CODEX_AUTO_DEV_AGENT_PHASE"));
    assert!(issue_agent.contains("trap '' HUP"));
    assert!(issue_agent.contains("nohup \"$codex_bin\" exec"));
    assert!(issue_agent.contains("CODEX_AUTO_DEV_CODEX_BIN"));
    assert!(issue_agent.contains("CODEX_AUTO_DEV_CODEX_APP"));
    assert!(issue_agent.contains("resolve_codex_bin"));
    assert!(!issue_agent.contains("/Applications/Codex.app"));
    assert!(issue_agent.contains("MUST NOT call codex-auto-dev approve/reject"));
    assert!(issue_agent.contains("gate_unavailable=true"));
    assert!(issue_agent.contains("shell_environment_policy.inherit"));
    let issue_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/issue-agent.md"))
        .expect("issue agent prompt readable");
    assert!(issue_agent_prompt.contains("共享 agent 契约"));
    assert!(issue_agent_prompt.contains("## 绝对边界"));
    assert!(issue_agent_prompt.contains("## Journal 格式"));
    assert!(issue_agent_prompt.contains("每条 reviewer critical/high 都必须有对应处理说明"));
    assert!(issue_agent_prompt.contains("## Reviewer 提交前自检"));
    assert!(issue_agent_prompt.contains("PlanReviewer 提交前自检"));
    assert!(issue_agent_prompt.contains("TestReviewer"));
    assert!(issue_agent_prompt.contains("DesignReviewer"));
    let plan_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/plan-agent.md"))
        .expect("plan agent prompt readable");
    assert!(plan_agent_prompt.contains("## 启动前检查"));
    assert!(plan_agent_prompt.contains("## Plan 必须包含"));
    assert!(plan_agent_prompt.contains("## PlanReviewer 提交前自检清单"));
    assert!(plan_agent_prompt.contains("逐项核对 PlanReviewer"));
    assert!(plan_agent_prompt.contains("不得退出交给 PlanReviewer"));
    assert!(plan_agent_prompt.contains("不运行 `submit`、`plan-review`"));
    let implementation_agent_prompt =
        fs::read_to_string(workspace.join("tools/prompts/implementation-agent.md"))
            .expect("implementation agent prompt readable");
    assert!(implementation_agent_prompt.contains("## 实现规则"));
    assert!(implementation_agent_prompt.contains("## 测试与验证要求"));
    assert!(implementation_agent_prompt.contains("不是由本分支改动导致的已有测试失败"));
    assert!(implementation_agent_prompt.contains("Baseline failure"));
    assert!(implementation_agent_prompt.contains("## 文档与 checklist 要求"));
    assert!(implementation_agent_prompt.contains("所有交付文档中的 checklist 必须全部打勾"));
    assert!(
        implementation_agent_prompt.contains("无法由当前流程完成的事项不得保留为未勾选 checklist")
    );
    assert!(implementation_agent_prompt.contains("## Code Review 提交前自检"));
    assert!(implementation_agent_prompt.contains("逐项核对 TestReviewer"));
    assert!(implementation_agent_prompt.contains("逐项核对 DesignReviewer"));
    assert!(implementation_agent_prompt.contains("不得退出交给 code-review"));
    assert!(implementation_agent_prompt.contains("## Change Doc 必须包含"));
    assert!(implementation_agent_prompt.contains("不运行 `submit`、`code-review`"));
    assert!(implementation_agent_prompt.contains("不处理 PR rebase 冲突"));
    let rebase_agent =
        fs::read_to_string(workspace.join("tools/rebase-agent.sh")).expect("rebase tool readable");
    assert!(rebase_agent.contains("Connector contract"));
    assert!(rebase_agent.contains("CODEX_AUTO_DEV_AGENT_PHASE"));
    assert!(rebase_agent.contains("rebase"));
    assert!(rebase_agent.contains("resolve_codex_bin"));
    let rebase_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/rebase-agent.md"))
        .expect("rebase agent prompt readable");
    assert!(rebase_agent_prompt.contains("RebaseAgent"));
    assert!(rebase_agent_prompt.contains("保留 base/master"));
    assert!(rebase_agent_prompt.contains("不能为了自己分支的修改删除 base/master 新代码"));
    assert!(rebase_agent_prompt.contains("不得扩大需求范围"));
    let pr_tool =
        fs::read_to_string(workspace.join("tools/pr-create.sh")).expect("pr tool readable");
    assert!(pr_tool.contains("Connector contract"));
    assert!(pr_tool.contains("created<TAB>url"));
    assert!(pr_tool.contains("existing<TAB>url"));
    assert!(pr_tool.contains("gh pr list"));
    assert!(pr_tool.contains("gh pr create"));
    let pr_status_tool =
        fs::read_to_string(workspace.join("tools/pr-status.sh")).expect("pr status tool readable");
    assert!(pr_status_tool.contains("Connector contract"));
    assert!(pr_status_tool.contains("status<TAB>url<TAB>detail"));
    let review_tool =
        fs::read_to_string(workspace.join("tools/plan-review.sh")).expect("review tool readable");
    assert!(review_tool.contains("Connector contract"));
    assert!(review_tool.contains("exactly one JSON object"));
    assert!(review_tool.contains("gate_unavailable=true"));
    assert!(review_tool.contains("CODEX_AUTO_DEV_REVIEW_CONTEXT"));
    assert!(review_tool.contains("CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS"));
    assert!(review_tool.contains("CODEX_AUTO_DEV_REVIEW_CODEX_HOME"));
    assert!(review_tool.contains("CODEX_AUTO_DEV_CODEX_BIN"));
    assert!(review_tool.contains("CODEX_AUTO_DEV_CODEX_APP"));
    assert!(review_tool.contains("resolve_codex_bin"));
    assert!(review_tool.contains("CODEX_HOME=\"$review_codex_home\" \"$codex_bin\" exec"));
    assert!(!review_tool.contains("/Applications/Codex.app"));
    assert!(review_tool.contains("--ephemeral"));
    assert!(review_tool.contains("--sandbox workspace-write"));
    let test_review_prompt = fs::read_to_string(workspace.join("tools/prompts/test-reviewer.md"))
        .expect("test reviewer prompt readable");
    assert!(test_review_prompt.contains("## 独立评审边界"));
    assert!(test_review_prompt.contains("不得读取 `reviews/`"));
    assert!(test_review_prompt.contains("不得读取、引用或依赖其他 reviewer 的意见"));
    assert!(test_review_prompt.contains("不是由本分支改动导致的已有测试失败"));
    assert!(test_review_prompt.contains("Baseline failure"));
    let design_review_prompt =
        fs::read_to_string(workspace.join("tools/prompts/design-reviewer.md"))
            .expect("design reviewer prompt readable");
    assert!(design_review_prompt.contains("## 独立评审边界"));
    assert!(design_review_prompt.contains("不得读取 `reviews/`"));
    assert!(design_review_prompt.contains("不得读取、引用或依赖 TestReviewer"));
    let integration_review_prompt =
        fs::read_to_string(workspace.join("tools/prompts/integration-reviewer.md"))
            .expect("integration reviewer prompt readable");
    assert!(integration_review_prompt.contains("IntegrationReviewer"));
    assert!(integration_review_prompt.contains("没有 `<<<<<<<`"));
    assert!(integration_review_prompt.contains("保留 base/master"));
    assert!(integration_review_prompt.contains("不能为了自己分支的修改删除 base/master 新代码"));
    assert!(integration_review_prompt.contains("change-doc"));
    let review_schema =
        fs::read_to_string(workspace.join("tools/schemas/review-result.schema.json"))
            .expect("review schema readable");
    assert!(review_schema.contains("\"gate_unavailable\""));
    assert!(review_schema.contains("\"recommended_next_phase\""));
    assert!(review_schema.contains("\"additionalProperties\": false"));
    assert!(review_schema.contains("\"title\""));
    assert!(review_schema.contains("\"evidence\""));
    assert!(review_schema.contains("\"impact\""));
    assert!(review_schema.contains("\"required_fix\""));
    assert!(review_schema.contains("\"suggested_change\""));
    assert!(review_schema.contains("\"verification\""));
    assert!(!review_schema.contains("\"$schema\""));
    let plan_prompt = fs::read_to_string(workspace.join("tools/prompts/plan-reviewer.md"))
        .expect("plan reviewer prompt readable");
    assert!(plan_prompt.contains("## 输出协议"));
    assert!(plan_prompt.contains("## Approved 示例"));
    assert!(plan_prompt.contains("## Gate Unavailable 示例"));
    let test_prompt = fs::read_to_string(workspace.join("tools/prompts/test-reviewer.md"))
        .expect("test reviewer prompt readable");
    assert!(test_prompt.contains("失败路径测试必须断言明确错误文本"));
    assert!(test_prompt.contains("\"reviewer\": \"TestReviewer\""));
    let design_prompt = fs::read_to_string(workspace.join("tools/prompts/design-reviewer.md"))
        .expect("design reviewer prompt readable");
    assert!(design_prompt.contains("不允许为了通过流程修改 reviewer"));
    assert!(design_prompt.contains("\"reviewer\": \"DesignReviewer\""));
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
fn workspace_registry_tracks_new_upgrade_and_current_list_refresh() {
    let registry_home = temp_workspace("registry-home");
    let workspace = temp_workspace("registry-one");
    let registry_home_str = registry_home
        .to_str()
        .expect("registry home path should be utf-8");
    assert_success(&run_with_env(
        &workspace,
        &["new", "--name", "registry-project"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));

    let registry_path = registry_home.join("workspaces.json");
    let registry = fs::read_to_string(&registry_path).expect("workspace registry should exist");
    assert!(registry.contains("\"schema_version\""));
    assert!(registry.contains("\"repo_name\": \"registry-project\""));
    assert!(registry.contains(&workspace.to_string_lossy().to_string()));
    assert!(registry.contains("\"request_count\": 0"));

    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tRegistry request\\tRegistry body\\thttps://example.test/registry\\n'\n",
    )
    .expect("issue connector should be writable");
    assert_success(&run_with_env(
        &workspace,
        &["update"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));
    let list = run_with_env(
        &workspace,
        &["list"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    );
    assert_success(&list);
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list_stdout.contains("REQ-0001"));
    assert!(list_stdout.contains("Registry request"));

    let refreshed_registry =
        fs::read_to_string(&registry_path).expect("workspace registry should be refreshed");
    assert!(refreshed_registry.contains("\"request_count\": 1"));
    assert!(refreshed_registry.contains("\"discovered\": 1"));

    assert_success(&run_with_env(
        &workspace,
        &["upgrade"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));
    let upgraded_registry =
        fs::read_to_string(&registry_path).expect("workspace registry should survive upgrade");
    assert!(upgraded_registry.contains("\"repo_name\": \"registry-project\""));
    assert!(upgraded_registry.contains("\"last_status\": \"ready\""));
}

#[test]
fn dashboard_json_lists_all_registered_workspaces_with_stage_files_and_review_attempts() {
    let registry_home = temp_workspace("dashboard-registry-home");
    let workspace_one = temp_workspace("dashboard-one");
    let workspace_two = temp_workspace("dashboard-two");
    let registry_home_str = registry_home
        .to_str()
        .expect("registry home path should be utf-8");

    assert_success(&run_with_env(
        &workspace_one,
        &["new", "--name", "dashboard-one"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));
    assert_success(&run_with_env(
        &workspace_two,
        &["new", "--name", "dashboard-two"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));

    fs::write(
        workspace_one.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tDashboard request\\tDashboard body\\thttps://example.test/dashboard\\n'\n",
    )
    .expect("issue connector should be writable");
    assert_success(&run_with_env(
        &workspace_one,
        &["update"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));
    let change_name = format!("{}-dashboard-request", current_date());
    assert_success(&run_with_env(
        &workspace_one,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    ));

    let change_path = workspace_one.join("docs/changes").join(&change_name);
    let plan_review_details = change_path.join("reviews/plan-review/details");
    let code_review_details = change_path.join("reviews/code-review/details");
    let integration_review_details = change_path.join("reviews/integration-review/details");
    let pr_conflict_attempts = change_path.join("pr-conflicts/attempts");
    fs::create_dir_all(&plan_review_details).expect("plan review details dir writable");
    fs::create_dir_all(&code_review_details).expect("code review details dir writable");
    fs::create_dir_all(&integration_review_details)
        .expect("integration review details dir writable");
    fs::create_dir_all(&pr_conflict_attempts).expect("pr conflict attempts dir writable");
    fs::write(
        plan_review_details.join("001-plan-reviewer.json"),
        "{\"reviewer\":\"PlanReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"round one asks for more detail\",\"process\":[\"read request\",\"read plan\"],\"critical\":[],\"high\":[{\"title\":\"缺少测试计划\",\"evidence\":\"plan.md 没有失败路径测试\",\"impact\":\"实现可能没有回归保护\",\"required_fix\":\"补充测试计划\",\"suggested_change\":\"在 plan.md 的测试章节列出失败路径和回归路径。\",\"verification\":\"重新运行 plan-review。\"}],\"warning\":[],\"info\":[]}\n",
    )
    .expect("plan review detail writable");
    fs::write(
        plan_review_details.join("002-plan-reviewer.json"),
        "{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"round two approved\",\"process\":[\"read updated plan\"],\"critical\":[],\"high\":[],\"warning\":[{\"title\":\"保留人工关注点\",\"evidence\":\"plan.md 仍有一个后续优化\",\"impact\":\"不阻塞实现，但人类评审应关注\",\"required_fix\":\"无需阻塞修复\",\"suggested_change\":\"在 change-doc.md 记录该后续优化是否完成。\",\"verification\":\"code-review 时检查 change-doc。\"}],\"info\":[]}\n",
    )
    .expect("plan review second detail writable");
    fs::write(
        code_review_details.join("001-test-reviewer.json"),
        "{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"read tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}\n",
    )
    .expect("test review detail writable");
    fs::write(
        code_review_details.join("001-design-reviewer.json"),
        "{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"read design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}\n",
    )
    .expect("design review detail writable");
    fs::write(
        integration_review_details.join("001-integration-reviewer.json"),
        "{\"reviewer\":\"IntegrationReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"integration ok\",\"process\":[\"checked rebase\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[{\"title\":\"保留 base/master\",\"evidence\":\"change-doc.md 记录 PR 集成刷新记录\",\"impact\":\"人类 reviewer 可以确认 rebase 支线\",\"required_fix\":\"无需修复\",\"suggested_change\":\"继续在 PR 中查看集成记录。\",\"verification\":\"dashboard 显示 integration-review detail。\"}]}\n",
    )
    .expect("integration review detail writable");
    fs::write(
        change_path.join("change-doc.md"),
        "# Change\n\n## PR 集成刷新记录\n\n保留 base/master。\n",
    )
    .expect("change doc with integration record writable");
    fs::write(
        pr_conflict_attempts.join("001-rebase-conflict.md"),
        "# PR 冲突记录 Attempt 001\n\n## 冲突诊断\n\nRebase stopped with conflicts in README.md。\n",
    )
    .expect("pr conflict attempt writable");
    force_request_state(
        &workspace_one,
        "REQ-0001",
        "wait-update-pr",
        "codex/req-0001",
        "dev/worktrees/REQ-0001",
    );

    let dashboard_root = temp_workspace("dashboard-root");
    let output = run_with_env(
        &dashboard_root,
        &["dashboard", "--json"],
        &[("CODEX_AUTO_DEV_HOME", registry_home_str)],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"projects\""));
    assert!(stdout.contains("\"repo_name\": \"dashboard-one\""));
    assert!(stdout.contains("\"repo_name\": \"dashboard-two\""));
    assert!(stdout.contains("\"stage_id\": \"request\""));
    assert!(stdout.contains("\"stage_id\": \"plan\""));
    assert!(stdout.contains("\"stage_id\": \"plan-review\""));
    assert!(stdout.contains("\"stage_id\": \"implementation\""));
    assert!(stdout.contains("\"stage_id\": \"code-review\""));
    assert!(stdout.contains("\"stage_id\": \"finish-pr\""));
    assert!(stdout.contains(
        "\"stage_id\": \"finish-pr\", \"label\": \"Finish / PR\", \"title\": \"交付与 PR\", \"state\": \"active\""
    ));
    assert!(stdout.contains("\"stage_id\": \"pr-refresh\""));
    assert!(stdout.contains("\"title\": \"PR 冲突与刷新记录\""));
    assert!(stdout.contains("pr-conflicts/attempts"));
    assert!(stdout.contains("PR Refresh 冲突记录"));
    assert!(stdout.contains("Rebase stopped with conflicts in README.md"));
    assert!(stdout.contains("\"stage_id\": \"integration-review\""));
    assert!(stdout.contains("\"artifact_path\": \"docs/changes"));
    assert!(stdout.contains("plan.md"));
    assert!(stdout.contains("change-doc.md"));
    assert!(stdout.contains("\"review_attempts\""));
    assert!(stdout.contains("\"attempt\": 1"));
    assert!(stdout.contains("\"attempt\": 2"));
    assert!(stdout.contains("001-plan-reviewer.json"));
    assert!(stdout.contains("002-plan-reviewer.json"));
    assert!(stdout.contains("TestReviewer"));
    assert!(stdout.contains("DesignReviewer"));
    assert!(stdout.contains("IntegrationReviewer"));
    assert!(stdout.contains("integration ok"));
    assert!(stdout.contains("保留 base/master"));
    assert!(stdout.contains("round two approved"));
    assert!(stdout.contains("保留人工关注点"));
    assert!(stdout.contains("\"artifact_kind\": \"review-details\""));
}

#[test]
fn new_url_clones_existing_target_repo() {
    let source = temp_workspace("source-repo");
    assert_git_success(&source, &["init"]);
    assert_git_success(&source, &["checkout", "-B", "master"]);
    assert_git_success(&source, &["config", "user.name", "Test User"]);
    assert_git_success(&source, &["config", "user.email", "test@example.local"]);
    fs::write(source.join("README.md"), "# Source\n").expect("source file should be writable");
    assert_git_success(&source, &["add", "README.md"]);
    assert_git_success(&source, &["commit", "-m", "Initial source"]);

    let workspace = temp_workspace("new-url");
    let fake_bin = workspace.join("fake-bin");
    let codegraph_log = workspace.join("codegraph.log");
    let fake_path = install_fake_codegraph(&fake_bin);
    let codegraph_log_str = codegraph_log
        .to_str()
        .expect("codegraph log path should be utf-8");
    let source_url = source.to_str().expect("source path should be utf-8");
    let output = run_with_env(
        &workspace,
        &["new", "--url", source_url],
        &[
            ("PATH", &fake_path),
            ("CODEX_TEST_CODEGRAPH_LOG", codegraph_log_str),
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("workspace naming: arbitrary outer workspace name is OK"));
    assert!(stdout.contains("CodeGraph initialized"));
    assert!(stdout.contains("CodeGraph project preview required before planning"));

    assert!(workspace.join("dev/repo/README.md").is_file());
    assert!(workspace.join("dev/repo/.codegraph").is_dir());
    let codegraph_log_content =
        fs::read_to_string(codegraph_log).expect("fake codegraph log readable");
    assert!(codegraph_log_content.contains("init -i dev/repo"));
    assert!(git_success(
        &workspace.join("dev/repo"),
        &["rev-parse", "--verify", "HEAD"]
    ));
    assert!(workspace.join("tools/issue-update.sh").is_file());
    assert!(workspace.join("tools/pr-create.sh").is_file());
    assert!(
        workspace
            .join("skills/codex-auto-dev-workflow/SKILL.md")
            .is_file()
    );
}

#[test]
fn plan_preflight_initializes_codegraph_for_non_empty_repo() {
    let workspace = temp_workspace("plan-codegraph");
    let fake_bin = workspace.join("fake-bin");
    let codegraph_log = workspace.join("codegraph.log");
    let fake_path = install_fake_codegraph(&fake_bin);
    let codegraph_log_str = codegraph_log
        .to_str()
        .expect("codegraph log path should be utf-8");

    assert_success(&run_with_env(
        &workspace,
        &["new", "--name", "plan-codegraph-test"],
        &[
            ("PATH", &fake_path),
            ("CODEX_TEST_CODEGRAPH_LOG", codegraph_log_str),
        ],
    ));
    let target = workspace.join("dev/repo");
    assert_git_success(&target, &["config", "user.name", "Test User"]);
    assert_git_success(&target, &["config", "user.email", "test@example.local"]);
    fs::write(target.join("README.md"), "# Target\n").expect("target file writable");
    assert_git_success(&target, &["add", "README.md"]);
    assert_git_success(&target, &["commit", "-m", "Initial target"]);

    let change_name = format!("{}-codegraph-preflight", current_date());
    let output = run_with_env(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
        &[
            ("PATH", &fake_path),
            ("CODEX_TEST_CODEGRAPH_LOG", codegraph_log_str),
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("preflight: CodeGraph initialized"));
    assert!(workspace.join("dev/repo/.codegraph").is_dir());
    let codegraph_log_content =
        fs::read_to_string(codegraph_log).expect("fake codegraph log readable");
    assert!(codegraph_log_content.contains("init -i dev/repo"));
}

#[test]
fn new_url_empty_repository_skips_codegraph_until_user_request() {
    let origin = temp_workspace("empty-origin");
    assert_git_success(&origin, &["init", "--bare"]);

    let workspace = temp_workspace("new-url-empty");
    let output = run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("repository is empty"));
    assert!(stdout.contains("skip CodeGraph"));
    assert!(workspace.join("dev/repo/.git").is_dir());
}

#[test]
fn update_deduplicates_by_external_id_and_assigns_request_ids() {
    let workspace = temp_workspace("update");
    assert_success(&run(&workspace, &["new", "--name", "update-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst request\\tDetailed body line one\\\\nline two\\thttps://example.test/1\\n'\n",
    )
    .expect("issue tool should be replaceable");

    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(&workspace, &["update"]));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert_eq!(state.matches("REQ-0001").count(), 1);
    assert!(!state.contains("REQ-0002"));

    let change_name = format!("{}-issue-body", current_date());
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    let change_path = workspace.join("docs/changes").join(change_name);
    let request =
        fs::read_to_string(change_path.join("request.md")).expect("request should be readable");
    let plan = fs::read_to_string(change_path.join("plan.md")).expect("plan should be readable");
    assert!(request.contains("## 需求标题"));
    assert!(request.contains("First request"));
    assert!(request.contains("## 需求描述"));
    assert!(request.contains("Detailed body line one\nline two"));
    assert!(plan.contains("标题和描述都必须作为需求来源"));
}

#[test]
fn plan_refuses_when_remote_has_changes_that_need_pull() {
    let origin = create_bare_origin_with_master("behind-origin");
    let workspace = temp_workspace("plan-behind");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);

    let other = temp_workspace("other-clone");
    assert_git_success(
        &other,
        &[
            "clone",
            origin.to_str().expect("origin should be utf-8"),
            ".",
        ],
    );
    assert_git_success(&other, &["config", "user.name", "Other User"]);
    assert_git_success(&other, &["config", "user.email", "other@example.local"]);
    fs::write(other.join("REMOTE.md"), "# Remote update\n").expect("remote file writable");
    assert_git_success(&other, &["add", "REMOTE.md"]);
    assert_git_success(&other, &["commit", "-m", "Remote update"]);
    assert_git_success(&other, &["push", "origin", "master"]);

    let change_name = format!("{}-needs-pull", current_date());
    let output = run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    );
    assert_failure_contains(&output, "git pull required before planning");
    assert!(
        !workspace.join("docs/changes").join(change_name).exists(),
        "plan packet must not be created before repository sync"
    );
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
        "request.md",
        "plan.md",
        "change-doc.md",
        "agent-journal.md",
        "status.json",
    ] {
        assert!(
            change_path.join(artifact).is_file(),
            "missing artifact: {artifact}"
        );
    }
    for removed_artifact in [
        "issue.md",
        "spec.md",
        "tasks.md",
        "plan.html",
        "codex-plan.md",
        "codex-start.md",
    ] {
        assert!(
            !change_path.join(removed_artifact).exists(),
            "runtime workspace should not create {removed_artifact}"
        );
    }

    let plan = fs::read_to_string(change_path.join("plan.md")).expect("plan should be readable");
    let change_doc = fs::read_to_string(change_path.join("change-doc.md"))
        .expect("change doc should be readable");
    let journal = fs::read_to_string(change_path.join("agent-journal.md"))
        .expect("agent journal should be readable");
    let status =
        fs::read_to_string(change_path.join("status.json")).expect("status should be readable");

    assert!(plan.contains("这是计划模板"));
    assert!(plan.contains("## 规范化需求记录"));
    assert!(plan.contains("- Request ID: `REQ-0001`"));
    assert!(plan.contains("- External ID: `manual:REQ-0001`"));
    assert!(plan.contains("- Source: `manual`"));
    assert!(plan.contains("### 需求名称"));
    assert!(plan.contains("### 需求描述"));
    assert!(plan.contains("agent 可以重写本文件，但必须保留并更新上面的规范化需求记录"));
    assert!(plan.contains("目标项目内部要求"));
    assert!(plan.contains("执行任务清单"));
    assert!(plan.contains("pre-commit"));
    assert!(plan.contains("AI review"));
    assert!(change_doc.contains("这是变更文档模板"));
    assert!(change_doc.contains("实现前后对比"));
    assert!(change_doc.contains("关键设计点"));
    assert!(change_doc.contains("变更范围摘要"));
    assert!(change_doc.contains("不需要完整文件清单"));
    assert!(change_doc.contains("目标项目内部要求"));
    assert!(change_doc.contains("文档与 Checklist"));
    assert!(change_doc.contains("所有交付文档中的 checklist 是否已全部打勾"));
    assert!(change_doc.contains("后续流程"));
    assert!(change_doc.contains("Review 结果"));
    assert!(journal.contains("agent 每轮"));
    assert!(status.contains("\"stage\": \"planning\""));
    assert!(change_path.join("approvals").is_dir());
}

#[test]
fn start_requires_plan_approval_before_creating_worktree() {
    let workspace = temp_workspace("start");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "start-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let rejected_start = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start, "plan approval required");

    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));

    let worktree = workspace.join("dev/worktrees/REQ-0001");
    assert!(worktree.is_dir(), "worktree should exist");
    assert!(
        git_success(&worktree, &["status", "--short"]),
        "orphan worktree should be a valid git worktree"
    );

    let change_path = workspace.join("docs/changes").join(change_name);
    let status =
        fs::read_to_string(change_path.join("status.json")).expect("status should be readable");
    assert!(status.contains("\"stage\": \"implementation\""));
    let change_doc = fs::read_to_string(change_path.join("change-doc.md"))
        .expect("change doc should be readable");
    assert!(change_doc.contains("实现前后对比"));
    assert!(change_doc.contains("关键设计点"));
    assert!(change_doc.contains("不需要完整文件清单"));
    assert!(change_doc.contains("Pre-commit"));
    assert!(change_doc.contains("AI review"));

    let sessions = fs::read_to_string(workspace.join(".codex-auto-dev/sessions.json"))
        .expect("sessions registry should be readable");
    assert!(sessions.contains("\"phase\": \"planning\""));
    assert!(sessions.contains("\"phase\": \"implementation\""));
    assert!(sessions.contains("\"status\": \"handoff-ready\""));
}

#[test]
fn start_auto_pulls_target_repo_before_creating_worktree() {
    let origin = create_bare_origin_with_master("start-pull-origin");
    let workspace = temp_workspace("start-pull");
    let change_name = format!("{}-auto-pull-before-start", current_date());
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));

    let other = temp_workspace("start-pull-other");
    assert_git_success(
        &other,
        &[
            "clone",
            origin.to_str().expect("origin should be utf-8"),
            ".",
        ],
    );
    assert_git_success(&other, &["config", "user.name", "Other User"]);
    assert_git_success(&other, &["config", "user.email", "other@example.local"]);
    fs::write(other.join("REMOTE.md"), "# Remote update before start\n")
        .expect("remote file writable");
    assert_git_success(&other, &["add", "REMOTE.md"]);
    assert_git_success(&other, &["commit", "-m", "Remote update before start"]);
    assert_git_success(&other, &["push", "origin", "master"]);

    let output = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("git pull: updated dev/repo before worktree creation"));
    assert!(
        workspace.join("dev/repo/REMOTE.md").is_file(),
        "target repo should be fast-forwarded before worktree creation"
    );
    assert!(
        workspace.join("dev/worktrees/REQ-0001/REMOTE.md").is_file(),
        "worktree should be based on the pulled target repo"
    );
}

#[test]
fn start_blocks_when_target_repo_pull_fails_before_worktree_creation() {
    let origin = create_bare_origin_with_master("start-pull-fail-origin");
    let workspace = temp_workspace("start-pull-fail");
    let change_name = format!("{}-pull-fail-before-start", current_date());
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    let target_repo = workspace.join("dev/repo");
    assert_git_success(&target_repo, &["checkout", "master"]);
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));

    assert_git_success(&target_repo, &["config", "user.name", "Local User"]);
    assert_git_success(
        &target_repo,
        &["config", "user.email", "local@example.local"],
    );
    fs::write(target_repo.join("LOCAL.md"), "# Local-only commit\n").expect("local file writable");
    assert_git_success(&target_repo, &["add", "LOCAL.md"]);
    assert_git_success(&target_repo, &["commit", "-m", "Local-only commit"]);

    let other = temp_workspace("start-pull-fail-other");
    assert_git_success(
        &other,
        &[
            "clone",
            origin.to_str().expect("origin should be utf-8"),
            ".",
        ],
    );
    assert_git_success(&other, &["config", "user.name", "Other User"]);
    assert_git_success(&other, &["config", "user.email", "other@example.local"]);
    fs::write(other.join("REMOTE.md"), "# Remote divergent commit\n")
        .expect("remote file writable");
    assert_git_success(&other, &["add", "REMOTE.md"]);
    assert_git_success(&other, &["commit", "-m", "Remote divergent commit"]);
    assert_git_success(&other, &["push", "origin", "master"]);

    let output = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&output, "git pull failed before worktree creation");
    assert!(
        !workspace.join("dev/worktrees/REQ-0001").exists(),
        "worktree must not be created after failed pull"
    );
    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("requests state readable");
    assert!(state.contains("\tblocked\t"));
    let status = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("status.json"),
    )
    .expect("status readable");
    assert!(status.contains("\"status\": \"blocked\""));
    assert!(status.contains("git pull failed before worktree creation"));
}

#[test]
fn finish_requires_change_doc_approval_then_commits_and_pushes_request_branch() {
    let workspace = temp_workspace("finish");
    let change_name = format!("{}-first-feature", current_date());
    let origin = create_bare_origin_with_master("finish-origin");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'github:owner/repo#42\\tgithub\\tFirst feature\\tBody\\thttps://github.com/owner/repo/issues/42\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-create.sh"),
        "#!/usr/bin/env sh\nset -eu\ncp \"$CODEX_AUTO_DEV_PR_BODY_FILE\" .codex-auto-dev/state/captured-pr-body.md\nprintf 'https://example.test/pr/1\\n'\n",
    )
    .expect("pr connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'open\\thttps://example.test/pr/1\\tstill under review\\n'\n",
    )
    .expect("pr status connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    let worktree = workspace.join("dev/worktrees/REQ-0001");
    assert_git_success(&worktree, &["config", "user.name", "Test User"]);
    assert_git_success(&worktree, &["config", "user.email", "test@example.local"]);
    fs::write(worktree.join("feature.txt"), "implemented\n").expect("feature should be writable");

    let rejected_finish = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_finish, "change-doc approval required");

    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "change-doc"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "change-doc",
            "--by",
            "tester",
            "--comment",
            "变更文档已检查",
        ],
    ));
    let review_dir = workspace
        .join("docs/changes")
        .join(&change_name)
        .join("reviews/code-review");
    let review_details_dir = review_dir.join("details");
    fs::create_dir_all(&review_details_dir).expect("review details dir should be writable");
    fs::write(
        review_details_dir.join("001-test-reviewer.json"),
        "{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests are adequate with a follow-up warning\",\"process\":[\"read diff\",\"read tests\"],\"critical\":[],\"high\":[],\"warning\":[{\"title\":\"补充跨平台 fixture\",\"evidence\":\"tests/cli_flow.rs 只覆盖默认 shell 路径\",\"impact\":\"非 GitHub connector 的路径差异可能晚些才暴露\",\"required_fix\":\"保持当前 PR 可合并，但后续补跨平台 fixture\",\"suggested_change\":\"在人类评审时确认是否需要为内部平台 connector 增加 fixture。\",\"verification\":\"新增 fixture 后运行 cargo test --test cli_flow。\"}],\"info\":[]}\n",
    )
    .expect("test review detail should be writable");
    fs::write(
        review_details_dir.join("001-design-reviewer.json"),
        "{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design is acceptable with review notes\",\"process\":[\"read approved plan\",\"read implementation\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[{\"title\":\"保留平台中立 PR contract\",\"evidence\":\"tools/pr-create.sh 通过环境变量接收 base/head/title/body\",\"impact\":\"后续 GitLab 或内部系统可以复用同一个 finish 流程\",\"required_fix\":\"无需阻塞修复\",\"suggested_change\":\"人类评审 PR 时重点确认 connector contract 是否足够表达内部平台。\",\"verification\":\"检查 PR 描述中的 connector contract 和默认脚本说明。\"}]}\n",
    )
    .expect("design review detail should be writable");
    fs::write(
        review_dir.join("summary.json"),
        "{\"schema_version\":1,\"request_id\":\"REQ-0001\",\"stage\":\"code-review\",\"attempt\":1,\"approved\":true,\"reviewers\":[{\"reviewer\":\"TestReviewer\",\"approved\":true,\"has_blocking_findings\":false,\"gate_unavailable\":false,\"recommended_next_phase\":\"implementation\",\"summary\":\"tests are adequate with a follow-up warning\",\"diagnostic\":\"\",\"path\":\"docs/changes/reviews/code-review/details/001-test-reviewer.json\"},{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"has_blocking_findings\":false,\"gate_unavailable\":false,\"recommended_next_phase\":\"implementation\",\"summary\":\"design is acceptable with review notes\",\"diagnostic\":\"\",\"path\":\"docs/changes/reviews/code-review/details/001-design-reviewer.json\"}],\"updated_at\":\"2026-05-31T00:00:00Z\"}\n",
    )
    .expect("review summary should be writable");
    let output = run(
        &workspace,
        &[
            "finish",
            "--request_id",
            "REQ-0001",
            "--message",
            "feat: deliver first feature",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("committed: feat: deliver first feature"));
    assert!(stdout.contains("pushed branch: codex/req-0001"));
    assert!(stdout.contains("PR created: https://example.test/pr/1"));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("wait-finish"));
    assert!(!state.contains("\tfinished\t"));
    let pushed_commit = git_output(&origin, &["rev-parse", "refs/heads/codex/req-0001"]);
    assert!(!pushed_commit.is_empty());
    let commit_message = git_output(&worktree, &["log", "-1", "--pretty=%s"]);
    assert_eq!(commit_message, "feat: deliver first feature");
    let pr_body = fs::read_to_string(workspace.join(".codex-auto-dev/state/captured-pr-body.md"))
        .expect("captured pr body should be readable");
    assert!(pr_body.contains("Closes owner/repo#42"));
    assert!(pr_body.contains("https://github.com/owner/repo/issues/42"));
    assert!(pr_body.contains("# Change Doc"));
    assert!(pr_body.contains("# Request"));
    assert!(pr_body.contains("# 自动评审意见"));
    assert!(pr_body.contains("TestReviewer"));
    assert!(pr_body.contains("warning"));
    assert!(pr_body.contains("补充跨平台 fixture"));
    assert!(pr_body.contains("非 GitHub connector 的路径差异可能晚些才暴露"));
    assert!(pr_body.contains("人类评审时确认是否需要为内部平台 connector 增加 fixture"));
    assert!(pr_body.contains("DesignReviewer"));
    assert!(pr_body.contains("info"));
    assert!(pr_body.contains("保留平台中立 PR contract"));
    assert!(pr_body.contains("后续 GitLab 或内部系统可以复用同一个 finish 流程"));

    let approval = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(change_name)
            .join("approvals/change-doc.approval.json"),
    )
    .expect("change-doc approval should be readable");
    assert!(approval.contains("\"status\": \"approved\""));
    assert!(approval.contains("\"artifact_sha256\""));
    assert!(approval.contains("变更文档已检查"));

    let pending_check = run(&workspace, &["pr-status", "--request_id", "REQ-0001"]);
    assert_success(&pending_check);
    let pending_stdout = String::from_utf8_lossy(&pending_check.stdout);
    assert!(pending_stdout.contains("PR status for REQ-0001: open"));
    assert!(pending_stdout.contains("request remains wait-finish"));

    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'merged\\thttps://example.test/pr/1\\tmerged into base\\n'\n",
    )
    .expect("pr status connector should be writable");
    let merged_check = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_success(&merged_check);
    let merged_stdout = String::from_utf8_lossy(&merged_check.stdout);
    assert!(merged_stdout.contains("PR status for REQ-0001: merged"));
    assert!(merged_stdout.contains("request marked finished"));
    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("\tfinished\t"));
}

#[test]
fn finish_reports_existing_pr_from_pr_connector() {
    let workspace = temp_workspace("finish-existing-pr");
    let change_name = format!("{}-existing-pr-feature", current_date());
    let origin = create_bare_origin_with_master("finish-existing-pr-origin");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'github:owner/repo#43\\tgithub\\tExisting PR feature\\tBody\\thttps://github.com/owner/repo/issues/43\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-create.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'existing\\thttps://example.test/pr/existing\\n'\n",
    )
    .expect("pr connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'open\\thttps://example.test/pr/existing\\tstill open\\n'\n",
    )
    .expect("pr status connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    let worktree = workspace.join("dev/worktrees/REQ-0001");
    assert_git_success(&worktree, &["config", "user.name", "Test User"]);
    assert_git_success(&worktree, &["config", "user.email", "test@example.local"]);
    fs::write(
        worktree.join("feature.txt"),
        "implemented existing pr flow\n",
    )
    .expect("feature should be writable");
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "change-doc"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "change-doc",
            "--by",
            "tester",
        ],
    ));

    let output = run(
        &workspace,
        &[
            "finish",
            "--request_id",
            "REQ-0001",
            "--message",
            "feat: deliver existing pr feature",
        ],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("marked wait-finish"));
    assert!(stdout.contains("PR already exists: https://example.test/pr/existing"));
    assert!(!stdout.contains("PR created: https://example.test/pr/existing"));

    let refreshed = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_success(&refreshed);
    let refreshed_stdout = String::from_utf8_lossy(&refreshed.stdout);
    assert!(refreshed_stdout.contains("PR status for REQ-0001: open"));
    assert!(refreshed_stdout.contains("request remains wait-finish"));

    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'merged\\thttps://example.test/pr/existing\\tmerged after review\\n'\n",
    )
    .expect("pr status connector should be writable");
    let finished = run(&workspace, &["pr-status", "--request_id", "REQ-0001"]);
    assert_success(&finished);
    let finished_stdout = String::from_utf8_lossy(&finished.stdout);
    assert!(finished_stdout.contains("PR status for REQ-0001: merged"));
    assert!(finished_stdout.contains("request marked finished"));
}

#[test]
fn pr_refresh_conflict_uses_rebase_agent_and_integration_review() {
    let workspace = temp_workspace("pr-refresh-conflict");
    let change_name = format!("{}-rebase-conflict-feature", current_date());
    let origin = create_bare_origin_with_master("pr-refresh-conflict-origin");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'github:owner/repo#44\\tgithub\\tRebase conflict feature\\tBody\\thttps://github.com/owner/repo/issues/44\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-create.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'existing\\thttps://example.test/pr/rebase\\n'\n",
    )
    .expect("pr connector should be writable");
    fs::write(
        workspace.join("tools/rebase-agent.sh"),
        "#!/usr/bin/env sh\nset -eu\ncd \"$CODEX_AUTO_DEV_WORKTREE\"\nprintf '# Source\\nmaster line\\nfeature line\\n' > README.md\ngit add README.md\nGIT_EDITOR=true git rebase --continue\nprintf '\\n## 测试 RebaseAgent 记录\\n\\n- 保留 base/master: master line\\n- 保留 request 分支: feature line\\n' >> \"$CODEX_AUTO_DEV_CHANGE_DOC\"\n",
    )
    .expect("rebase agent connector should be writable");
    fs::write(
        workspace.join("tools/integration-review.sh"),
        "#!/usr/bin/env sh\nset -eu\nif grep -R '<<<<<<<' \"$CODEX_AUTO_DEV_WORKTREE\" >/dev/null 2>&1; then\n  printf '{\"reviewer\":\"IntegrationReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"implementation\",\"summary\":\"conflict markers remain\",\"process\":[\"checked worktree\"],\"critical\":[{\"title\":\"conflict markers remain\",\"evidence\":\"worktree still contains conflict markers\",\"impact\":\"branch cannot be safely reviewed or merged\",\"required_fix\":\"remove conflict markers and complete rebase\",\"suggested_change\":\"resolve conflicted files and rerun integration review\",\"verification\":\"grep for conflict markers returns no matches\"}],\"high\":[],\"warning\":[],\"info\":[]}'\n  exit 0\nfi\nif ! grep -q 'master line' \"$CODEX_AUTO_DEV_WORKTREE/README.md\" || ! grep -q 'feature line' \"$CODEX_AUTO_DEV_WORKTREE/README.md\"; then\n  printf '{\"reviewer\":\"IntegrationReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"implementation\",\"summary\":\"base or branch change was dropped\",\"process\":[\"checked README\"],\"critical\":[],\"high\":[{\"title\":\"base or branch change dropped\",\"evidence\":\"README.md does not preserve both master line and feature line\",\"impact\":\"rebase lost behavior from either base/master or request branch\",\"required_fix\":\"preserve both sides of the conflict\",\"suggested_change\":\"rewrite README.md to keep master line and feature line, then rerun tests\",\"verification\":\"grep confirms both lines exist\"}],\"warning\":[],\"info\":[]}'\n  exit 0\nfi\nprintf '{\"reviewer\":\"IntegrationReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"integration preserved base and branch changes\",\"process\":[\"checked conflict markers\",\"checked base/master preservation\",\"checked request branch preservation\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[{\"title\":\"base/master preserved\",\"evidence\":\"README.md contains master line and feature line\",\"impact\":\"human reviewer can see both sides were retained\",\"required_fix\":\"none\",\"suggested_change\":\"keep this integration note in change-doc\",\"verification\":\"grep confirms both lines exist\"}]}'\n",
    )
    .expect("integration reviewer connector should be writable");

    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    let worktree = workspace.join("dev/worktrees/REQ-0001");
    assert_git_success(&worktree, &["config", "user.name", "Test User"]);
    assert_git_success(&worktree, &["config", "user.email", "test@example.local"]);
    fs::write(worktree.join("README.md"), "# Source\nfeature line\n")
        .expect("feature README should be writable");
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "change-doc"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "change-doc",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(
        &workspace,
        &[
            "finish",
            "--request_id",
            "REQ-0001",
            "--message",
            "feat: deliver rebase conflict feature",
        ],
    ));

    let repo = workspace.join("dev/repo");
    assert_git_success(&repo, &["config", "user.name", "Test User"]);
    assert_git_success(&repo, &["config", "user.email", "test@example.local"]);
    assert_git_success(&repo, &["checkout", "master"]);
    fs::write(repo.join("README.md"), "# Source\nmaster line\n")
        .expect("master README should be writable");
    assert_git_success(&repo, &["add", "README.md"]);
    assert_git_success(&repo, &["commit", "-m", "Update master line"]);
    assert_git_success(&repo, &["push", "origin", "master"]);

    let refresh = run(&workspace, &["pr-refresh", "--request_id", "REQ-0001"]);
    assert_success(&refresh);
    let refresh_stdout = String::from_utf8_lossy(&refresh.stdout);
    assert!(refresh_stdout.contains("rebase conflict"));
    assert!(refresh_stdout.contains("rebase-agent"));

    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/requests.tsv"),
        "wait-update-pr",
    );
    let readme = fs::read_to_string(worktree.join("README.md")).expect("README readable");
    assert!(readme.contains("master line"));
    assert!(readme.contains("feature line"));
    let change_doc = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("change-doc.md"),
    )
    .expect("change doc readable");
    assert!(change_doc.contains("PR 集成刷新记录"));
    assert!(change_doc.contains("PR 冲突记录 (Attempt 001)"));
    assert!(change_doc.contains("保留 base/master"));
    let conflict_record = workspace
        .join("docs/changes")
        .join(&change_name)
        .join("pr-conflicts/attempts/001-rebase-conflict.md");
    assert!(conflict_record.is_file());
    let conflict_record_content =
        fs::read_to_string(conflict_record).expect("conflict record readable");
    assert!(conflict_record_content.contains("PR 冲突记录 Attempt 001"));
    assert!(conflict_record_content.contains("Rebase stopped with conflicts"));
    assert!(conflict_record_content.contains("不得为了消除冲突直接删除 master 上的新代码"));
    let integration_detail = workspace
        .join("docs/changes")
        .join(&change_name)
        .join("reviews/integration-review/details/001-integration-reviewer.json");
    assert!(integration_detail.is_file());
}

#[test]
fn review_gates_require_structured_passes_before_start_and_change_doc_approval() {
    let workspace = temp_workspace("review-gates");
    let change_name = format!("{}-reviewed-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "review-gates-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"plan is too vague\",\"process\":[\"read issue\",\"read plan\"],\"critical\":[],\"high\":[{\"title\":\"missing compatibility plan\",\"evidence\":\"plan lacks compatibility section\",\"impact\":\"implementation may break existing behavior without migration or fallback\",\"required_fix\":\"add compatibility plan\",\"suggested_change\":\"Add a compatibility section describing preserved behavior, migration steps, and fallback handling before implementation starts.\",\"verification\":\"Rerun plan-review and confirm the plan names compatibility tests or migration checks.\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    let rejected_plan_review = run(&workspace, &["plan-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_plan_review, "PlanReviewer rejected");
    let plan_review_path = workspace
        .join("docs/changes")
        .join(&change_name)
        .join("reviews/plan-review/details/001-plan-reviewer.json");
    let rejected_review =
        fs::read_to_string(&plan_review_path).expect("plan review should be written");
    assert!(rejected_review.contains("\"high\""));
    let rejected_start_after_review = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start_after_review, "plan approval required");

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan is implementation-grade\",\"process\":[\"read issue\",\"read plan\",\"read repo\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[{\"title\":\"minor follow-up\",\"evidence\":\"plan could add one follow-up note\",\"impact\":\"non-blocking documentation clarity improvement\",\"required_fix\":\"optional follow-up only\",\"suggested_change\":\"Optionally add a short follow-up note after implementation scope.\",\"verification\":\"No gate rerun required unless the plan text is changed materially.\"}]}'\n",
    )
    .expect("plan review connector should be writable");
    assert_success(&run(
        &workspace,
        &["plan-review", "--request_id", "REQ-0001"],
    ));
    let plan_approval = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("approvals/plan.approval.json"),
    )
    .expect("plan approval should be readable");
    assert!(plan_approval.contains("\"status\": \"approved\""));
    assert!(plan_approval.contains("PlanReviewer"));

    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    let worktree = workspace.join("dev/worktrees/REQ-0001");
    fs::write(worktree.join("feature.txt"), "implemented\n").expect("feature should be writable");

    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests are sufficient\",\"process\":[\"checked diff\",\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"implementation\",\"summary\":\"implementation hardcodes behavior\",\"process\":[\"checked approved plan\",\"checked diff\"],\"critical\":[],\"high\":[{\"title\":\"hardcoded behavior\",\"evidence\":\"feature.txt hardcodes behavior\",\"impact\":\"future requests cannot reuse or configure the behavior safely\",\"required_fix\":\"replace hardcoded behavior with extensible implementation\",\"suggested_change\":\"Move the hardcoded value into configuration or derive it from request/repository state following the approved plan.\",\"verification\":\"Add or update a test that exercises a non-default value and rerun code-review.\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    let rejected_code_review = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_code_review, "DesignReviewer rejected");
    let rejected_finish_after_review = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_failure_contains(
        &rejected_finish_after_review,
        "change-doc approval required",
    );

    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"implementation matches approved plan\",\"process\":[\"checked approved plan\",\"checked diff\",\"checked secrets\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    assert_success(&run(
        &workspace,
        &["code-review", "--request_id", "REQ-0001"],
    ));
    let change_doc_approval = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("approvals/change-doc.approval.json"),
    )
    .expect("change-doc approval should be readable");
    assert!(change_doc_approval.contains("\"status\": \"approved\""));
    assert!(change_doc_approval.contains("code-review"));
    assert!(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("reviews/code-review/details/001-test-reviewer.json")
            .is_file()
    );
    assert!(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("reviews/code-review/details/001-design-reviewer.json")
            .is_file()
    );
    let change_doc = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("change-doc.md"),
    )
    .expect("change-doc should be readable");
    assert!(change_doc.contains("## Review 结果"));
    assert!(change_doc.contains("PlanReviewer"));
    assert!(change_doc.contains("TestReviewer"));
    assert!(change_doc.contains("DesignReviewer"));
}

#[test]
fn code_reviewers_get_isolated_context_without_other_or_historical_review_outputs() {
    let workspace = temp_workspace("review-isolation");
    let change_name = format!("{}-review-isolation", current_date());
    let origin = create_bare_origin_with_master("review-isolation-origin");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));

    let change_path = workspace.join("docs/changes").join(&change_name);
    let historical_details = change_path.join("reviews/code-review/details");
    fs::create_dir_all(&historical_details).expect("historical review dir writable");
    fs::write(
        historical_details.join("001-test-reviewer.json"),
        "{\"reviewer\":\"TestReviewer\",\"approved\":false}\n",
    )
    .expect("historical detail writable");
    fs::write(
        change_path.join("reviews/code-review/summary.json"),
        "{\"approved\":false,\"reviewers\":[{\"reviewer\":\"TestReviewer\"}]}\n",
    )
    .expect("historical summary writable");

    fs::write(
        workspace.join("tools/test-review.sh"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${CODEX_AUTO_DEV_CHANGE_PATH:-}" != "${CODEX_AUTO_DEV_REVIEW_CONTEXT:-}" ]; then
  echo "review context was not isolated" >&2
  exit 2
fi
if [ -e "$CODEX_AUTO_DEV_CHANGE_PATH/reviews" ]; then
  echo "historical review output leaked into TestReviewer context" >&2
  exit 3
fi
case "${CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS:-}" in
  *reviews/code-review*) ;;
  *) echo "forbidden review paths were not declared" >&2; exit 4 ;;
esac
printf '{"reviewer":"TestReviewer","approved":true,"gate_unavailable":false,"decision":"approved","recommended_next_phase":"implementation","summary":"isolated test review ok","process":["checked isolated review context"],"critical":[],"high":[],"warning":[],"info":[]}\n'
"#,
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${CODEX_AUTO_DEV_CHANGE_PATH:-}" != "${CODEX_AUTO_DEV_REVIEW_CONTEXT:-}" ]; then
  echo "review context was not isolated" >&2
  exit 2
fi
if [ -e "$CODEX_AUTO_DEV_CHANGE_PATH/reviews" ]; then
  echo "TestReviewer or historical output leaked into DesignReviewer context" >&2
  exit 3
fi
case "${CODEX_AUTO_DEV_REVIEW_FORBIDDEN_PATHS:-}" in
  *reviews/code-review*) ;;
  *) echo "forbidden review paths were not declared" >&2; exit 4 ;;
esac
printf '{"reviewer":"DesignReviewer","approved":true,"gate_unavailable":false,"decision":"approved","recommended_next_phase":"implementation","summary":"isolated design review ok","process":["checked isolated review context"],"critical":[],"high":[],"warning":[],"info":[]}\n'
"#,
    )
    .expect("design review connector should be writable");

    let review = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_success(&review);
    assert!(
        change_path
            .join("reviews/code-review/details/002-test-reviewer.json")
            .is_file(),
        "new attempt should still be written to the canonical review details"
    );
    assert!(
        change_path
            .join("reviews/code-review/details/002-design-reviewer.json")
            .is_file(),
        "design reviewer detail should still be persisted after isolated review"
    );
}

#[test]
fn advance_syncs_stale_request_index_from_status_json_before_reviewing() {
    let workspace = temp_workspace("stale-index");
    let change_name = format!("{}-stale-index", current_date());
    assert_success(&run(&workspace, &["new", "--name", "stale-index-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf 'plan-review\\n' >> .codex-auto-dev/state/review.log\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf 'test-review\\n' >> .codex-auto-dev/state/review.log\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf 'design-review\\n' >> .codex-auto-dev/state/review.log\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nprintf 'implementation dispatched\\n' >> .codex-auto-dev/state/agent-dispatch.log\n",
    )
    .expect("issue agent should be writable");

    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &["plan-review", "--request_id", "REQ-0001"],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "change-doc"],
    ));
    fs::write(
        workspace.join("docs/changes").join(&change_name).join("status.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"REQ-0001\",\n  \"stage\": \"implementation\",\n  \"current_phase\": \"change-doc-submitted\",\n  \"status\": \"change-doc-submitted\",\n  \"reason\": \"submitted by outer tick\",\n  \"return_to_phase_reason\": \"submitted by outer tick\",\n  \"review_cycle\": 1,\n  \"handoff_artifacts\": {{}},\n  \"branch\": \"codex/req-0001\",\n  \"worktree\": \"dev/worktrees/REQ-0001\",\n  \"updated_at\": \"{}\"\n}}\n",
            current_unix_timestamp()
        ),
    )
    .expect("status json should be writable");

    let review_log_before = fs::read_to_string(workspace.join(".codex-auto-dev/state/review.log"))
        .expect("review log readable");
    assert_eq!(review_log_before.matches("plan-review").count(), 1);
    force_request_state(&workspace, "REQ-0001", "plan-submitted", "", "");

    let advance = run(&workspace, &["advance", "--request_id", "REQ-0001"]);
    assert_success(&advance);
    let stdout = String::from_utf8_lossy(&advance.stdout);
    assert!(stdout.contains("Advance complete for REQ-0001"));

    let review_log = fs::read_to_string(workspace.join(".codex-auto-dev/state/review.log"))
        .expect("review log readable");
    assert_eq!(
        review_log.matches("plan-review").count(),
        1,
        "advance must not rerun plan-review when status.json is already change-doc-submitted"
    );
    assert!(review_log.contains("test-review"));
    assert!(review_log.contains("design-review"));
    assert!(
        !workspace
            .join(".codex-auto-dev/state/agent-dispatch.log")
            .exists(),
        "advance must not dispatch a duplicate implementation agent"
    );
    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("requests state readable");
    assert!(state.contains("wait-update-pr"));
    assert!(state.contains("dev/worktrees/REQ-0001"));
}

#[test]
fn list_and_status_sync_stale_request_index_from_status_json() {
    let workspace = temp_workspace("list-status-sync");
    let change_name = format!("{}-list-status-sync", current_date());
    let origin = create_bare_origin_with_master("list-status-sync-origin");
    assert_success(&run(
        &workspace,
        &[
            "new",
            "--url",
            origin.to_str().expect("origin should be utf-8"),
        ],
    ));
    assert_git_success(&workspace.join("dev/repo"), &["checkout", "master"]);
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));

    let change_path = workspace.join("docs/changes").join(&change_name);
    fs::write(
        change_path.join("status.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"REQ-0001\",\n  \"stage\": \"implementation\",\n  \"current_phase\": \"wait-update-pr\",\n  \"status\": \"wait-update-pr\",\n  \"reason\": \"code-review approved\",\n  \"return_to_phase_reason\": \"code-review approved\",\n  \"review_cycle\": 1,\n  \"handoff_artifacts\": {{}},\n  \"branch\": \"codex/req-0001\",\n  \"worktree\": \"dev/worktrees/REQ-0001\",\n  \"updated_at\": \"{}\"\n}}\n",
            current_unix_timestamp()
        ),
    )
    .expect("runtime status should be writable");
    force_request_state(
        &workspace,
        "REQ-0001",
        "implementation-agent-running",
        "",
        "",
    );

    let status = run(&workspace, &["status", "REQ-0001"]);
    assert_success(&status);
    let status_stdout = String::from_utf8_lossy(&status.stdout);
    assert!(status_stdout.contains("status: wait-update-pr"));
    assert!(status_stdout.contains("branch: codex/req-0001"));

    force_request_state(
        &workspace,
        "REQ-0001",
        "implementation-agent-running",
        "",
        "",
    );
    let list = run(&workspace, &["list"]);
    assert_success(&list);
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list_stdout.contains("REQ-0001"));
    assert!(list_stdout.contains("wait-update-pr"));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("requests state readable");
    assert!(state.contains("wait-update-pr"));
    assert!(state.contains("codex/req-0001"));
    assert!(state.contains("dev/worktrees/REQ-0001"));
}

#[test]
fn review_gate_backend_failure_blocks_request_with_diagnostics() {
    let workspace = temp_workspace("review-backend-failure");
    let change_name = format!("{}-review-backend-failure", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "review-backend-failure-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\necho 'backend offline: reviewer model unavailable' >&2\nexit 42\n",
    )
    .expect("plan review connector should be writable");

    let failed_review = run(&workspace, &["plan-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&failed_review, "PlanReviewer review gate unavailable");
    assert_failure_contains(&failed_review, "backend offline");

    let change_path = workspace.join("docs/changes").join(&change_name);
    let detail = fs::read_to_string(
        change_path
            .join("reviews/plan-review/details")
            .join("001-plan-reviewer.json"),
    )
    .expect("review detail should be written");
    assert!(detail.contains("\"summary\": \"review tool failed\""));
    assert!(detail.contains("backend offline"));

    let summary = fs::read_to_string(change_path.join("reviews/plan-review/summary.json"))
        .expect("review summary should be written");
    assert!(summary.contains("\"gate_unavailable\": true"));
    assert!(summary.contains("backend offline"));

    let status = fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(status.contains("\"status\": \"blocked\""));
    assert!(status.contains("plan-review gate unavailable"));

    let requests = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("requests state should be readable");
    assert!(requests.contains("blocked"));
}

#[test]
fn review_gate_rejects_legacy_json_missing_required_schema_fields() {
    let workspace = temp_workspace("review-legacy-json");
    let change_name = format!("{}-legacy-review-json", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "legacy-review-json-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"old shape\",\"process\":[\"checked\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");

    let failed_review = run(&workspace, &["plan-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&failed_review, "PlanReviewer review gate unavailable");

    let change_path = workspace.join("docs/changes").join(&change_name);
    let detail = fs::read_to_string(
        change_path
            .join("reviews/plan-review/details")
            .join("001-plan-reviewer.json"),
    )
    .expect("detail should be readable");
    assert!(detail.contains("\"gate_unavailable\": true"));
    assert!(detail.contains("invalid review JSON"));
    assert!(detail.contains("\"required_fix\""));
    assert!(detail.contains("\"suggested_change\""));
    assert!(detail.contains("\"verification\""));
}

#[test]
fn review_gate_rejects_findings_without_detailed_modification_advice() {
    let workspace = temp_workspace("review-vague-finding");
    let change_name = format!("{}-vague-review-finding", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "vague-review-finding-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"plan lacks details\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[{\"title\":\"missing tests\",\"evidence\":\"plan.md does not list failure path tests\",\"required_fix\":\"add tests\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");

    let failed_review = run(&workspace, &["plan-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&failed_review, "PlanReviewer review gate unavailable");

    let change_path = workspace.join("docs/changes").join(&change_name);
    let detail = fs::read_to_string(
        change_path
            .join("reviews/plan-review/details")
            .join("001-plan-reviewer.json"),
    )
    .expect("detail should be readable");
    assert!(detail.contains("invalid review JSON"));
    assert!(detail.contains("\"impact\""));
    assert!(detail.contains("\"suggested_change\""));
    assert!(detail.contains("\"verification\""));
}

#[test]
fn approval_becomes_stale_when_artifact_changes_after_approval() {
    let workspace = temp_workspace("stale-approval");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "stale-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));

    let plan_path = workspace
        .join("docs/changes")
        .join(change_name)
        .join("plan.md");
    fs::write(&plan_path, "# Plan\n\n审批后被修改。\n").expect("plan should be writable");

    let rejected_start = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start, "approval is stale");
}

#[test]
fn session_command_registers_visible_thread_links() {
    let workspace = temp_workspace("session");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "session-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    assert_success(&run(
        &workspace,
        &[
            "session",
            "--request_id",
            "REQ-0001",
            "--phase",
            "planning",
            "--thread_id",
            "thread-123",
            "--thread_url",
            "https://codex.example/thread-123",
            "--status",
            "running",
        ],
    ));

    let output = run(&workspace, &["sessions", "--json"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"thread_id\": \"thread-123\""));
    assert!(stdout.contains("\"thread_url\": \"https://codex.example/thread-123\""));
    assert!(stdout.contains("\"status\": \"running\""));
}

#[test]
fn upgrade_refreshes_examples_without_overwriting_user_connectors() {
    let workspace = temp_workspace("upgrade");
    let change_name = format!("{}-first-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "upgrade-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    fs::write(
        workspace.join(".codex-auto-dev/config.toml"),
        "repo_name = \"upgrade-test\"\ngit_url = \"local:upgrade-test\"\nbase_branch = \"main\"\n",
    )
    .expect("old config should be writable");
    fs::remove_file(workspace.join(".codex-auto-dev/sessions.json"))
        .expect("old workspace should not have sessions file");
    fs::remove_file(workspace.join("tools/pr-create.sh"))
        .expect("old workspace should not have pr connector");
    fs::remove_file(workspace.join("tools/plan-review.sh"))
        .expect("old workspace should not have plan review connector");
    fs::remove_file(workspace.join("tools/test-review.sh"))
        .expect("old workspace should not have test review connector");
    fs::remove_file(workspace.join("tools/design-review.sh"))
        .expect("old workspace should not have design review connector");
    fs::remove_file(workspace.join("tools/issue-agent.sh"))
        .expect("old workspace should not have issue agent connector");
    fs::remove_dir_all(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("approvals"),
    )
    .expect("old workspace should not have approvals dir");
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'custom connector\\n'\n",
    )
    .expect("custom issue tool should be writable");
    let journal_path = workspace
        .join("docs/changes")
        .join(&change_name)
        .join("agent-journal.md");
    fs::write(
        &journal_path,
        "# Agent Journal: REQ-0001\n\n这个文件用于避免上下文过长后无法恢复。agent 每轮都必须追加记录。\n\n## Attempt 1 - planning\n\n- Read: 原始需求和目标项目文档。\n- Changed: 填写 plan.md。\n",
    )
    .expect("agent journal should be writable");

    let dry_run = run(&workspace, &["upgrade", "--dry-run"]);
    assert_success(&dry_run);
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Would create .codex-auto-dev/sessions.json"));
    assert!(dry_run_stdout.contains("Would refresh tools/issue-update.example.sh"));
    assert!(dry_run_stdout.contains("Would refresh tools/plan-review.example.sh"));
    assert!(dry_run_stdout.contains("Would refresh tools/prompts/plan-reviewer.example.md"));
    assert!(
        dry_run_stdout.contains("Would refresh tools/schemas/review-result.example.schema.json")
    );
    assert!(dry_run_stdout.contains("不会替换正式 connector、prompt 或 review schema"));
    assert!(dry_run_stdout.contains("codex-auto-dev upgrade --default"));

    assert_success(&run(&workspace, &["upgrade"]));

    let config = fs::read_to_string(workspace.join(".codex-auto-dev/config.toml"))
        .expect("config should be readable");
    assert!(config.contains("schema_version = 3"));
    assert!(config.contains("parallel_limit = 1"));
    assert!(workspace.join(".codex-auto-dev/sessions.json").is_file());
    assert!(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("approvals")
            .is_dir()
    );
    let issue_tool =
        fs::read_to_string(workspace.join("tools/issue-update.sh")).expect("tool readable");
    assert!(issue_tool.contains("custom connector"));
    let issue_tool_example = fs::read_to_string(workspace.join("tools/issue-update.example.sh"))
        .expect("issue tool example readable");
    assert!(issue_tool_example.contains("Connector contract"));
    assert!(issue_tool_example.contains("gh api --method GET"));
    assert!(!workspace.join("tools/pr-create.sh").exists());
    assert!(workspace.join("tools/pr-create.example.sh").is_file());
    assert!(!workspace.join("tools/plan-review.sh").exists());
    assert!(workspace.join("tools/plan-review.example.sh").is_file());
    assert!(!workspace.join("tools/test-review.sh").exists());
    assert!(workspace.join("tools/test-review.example.sh").is_file());
    assert!(!workspace.join("tools/design-review.sh").exists());
    assert!(workspace.join("tools/design-review.example.sh").is_file());
    assert!(!workspace.join("tools/issue-agent.sh").exists());
    assert!(workspace.join("tools/issue-agent.example.sh").is_file());
    let journal = fs::read_to_string(journal_path).expect("agent journal should be readable");
    assert!(journal.contains("Attempt 1 - planning"));
    assert!(journal.contains("填写 plan.md"));
    assert!(workspace.join("tools/prompts/issue-agent.md").is_file());
    assert!(
        workspace
            .join("tools/prompts/issue-agent.example.md")
            .is_file()
    );
    assert!(workspace.join("tools/prompts/plan-agent.md").is_file());
    assert!(
        workspace
            .join("tools/prompts/plan-agent.example.md")
            .is_file()
    );
    assert!(
        workspace
            .join("tools/prompts/implementation-agent.md")
            .is_file()
    );
    assert!(
        workspace
            .join("tools/prompts/implementation-agent.example.md")
            .is_file()
    );
    assert!(
        workspace
            .join("tools/schemas/review-result.schema.json")
            .is_file()
    );
    let plan_reviewer_example =
        fs::read_to_string(workspace.join("tools/prompts/plan-reviewer.example.md"))
            .expect("plan reviewer prompt example readable");
    assert!(plan_reviewer_example.contains("recommended_next_phase"));
    let review_schema_example =
        fs::read_to_string(workspace.join("tools/schemas/review-result.example.schema.json"))
            .expect("review schema example readable");
    assert!(review_schema_example.contains("\"recommended_next_phase\""));
}

#[test]
fn upgrade_default_replaces_managed_assets_from_refreshed_examples() {
    let workspace = temp_workspace("upgrade-default");
    assert_success(&run(&workspace, &["new", "--name", "upgrade-default-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'custom connector\\n'\n",
    )
    .expect("custom issue tool should be writable");
    fs::write(
        workspace.join("tools/prompts/plan-reviewer.md"),
        "# custom reviewer prompt\n",
    )
    .expect("custom prompt should be writable");
    fs::write(
        workspace.join("tools/schemas/review-result.schema.json"),
        "{\"type\":\"object\"}\n",
    )
    .expect("custom schema should be writable");

    let dry_run = run(&workspace, &["upgrade", "--dry-run", "--default"]);
    assert_success(&dry_run);
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Would refresh tools/issue-update.example.sh"));
    assert!(
        dry_run_stdout
            .contains("Would replace tools/issue-update.sh from tools/issue-update.example.sh")
    );
    assert!(dry_run_stdout.contains(
        "Would replace tools/prompts/plan-reviewer.md from tools/prompts/plan-reviewer.example.md"
    ));
    assert!(dry_run_stdout.contains(
        "Would replace tools/schemas/review-result.schema.json from tools/schemas/review-result.example.schema.json"
    ));

    let output = run(&workspace, &["upgrade", "--default"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Replaced default runtime assets from refreshed examples"));

    assert_workspace_files_equal(
        &workspace,
        "tools/issue-update.sh",
        "tools/issue-update.example.sh",
    );
    assert_workspace_files_equal(
        &workspace,
        "tools/prompts/plan-reviewer.md",
        "tools/prompts/plan-reviewer.example.md",
    );
    assert_workspace_files_equal(
        &workspace,
        "tools/schemas/review-result.schema.json",
        "tools/schemas/review-result.example.schema.json",
    );
    let issue_tool =
        fs::read_to_string(workspace.join("tools/issue-update.sh")).expect("tool readable");
    assert!(!issue_tool.contains("custom connector"));
    assert!(issue_tool.contains("Connector contract"));
}

#[test]
fn tick_default_parallel_limit_counts_running_issue_agents() {
    let workspace = temp_workspace("tick-parallel-default");
    assert_success(&run(
        &workspace,
        &["new", "--name", "tick-parallel-default-test"],
    ));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond tick request\\tSecond body\\thttps://example.test/2\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"keep planning\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[{\"title\":\"hold request\",\"evidence\":\"test keeps planning request from advancing to implementation\",\"impact\":\"without this the hook could dispatch another phase during the test\",\"required_fix\":\"leave request in planning for this test\",\"suggested_change\":\"No production change; this connector is test-only.\",\"verification\":\"The request remains plan-review-rejected after the agent exits.\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s\n' "$CODEX_AUTO_DEV_REQUEST_ID" "$CODEX_AUTO_DEV_AGENT_PHASE" "$CODEX_AUTO_DEV_MAX_ATTEMPTS" >> .codex-auto-dev/state/agent.log
sleep 1
"#,
    )
    .expect("issue agent should be writable");

    let first = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&first);
    let first_stdout = String::from_utf8_lossy(&first.stdout);
    assert!(first_stdout.contains("Tick dispatched 1 issue-agent(s)."));
    assert!(first_stdout.contains("Dispatched REQ-0001"));
    assert!(!first_stdout.contains("Dispatched REQ-0002"));
    assert!(first_stdout.contains("parallel limit 1"));
    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/agent.log"),
        "agent called for REQ-0001 phase=planning max=20",
    );

    let second = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&second);
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("Tick parallel limit reached: 1/1 issue-agent(s) already running.")
    );
    assert!(!second_stdout.contains("Dispatched REQ-0002"));
    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert_eq!(state.matches("planning-agent-running").count(), 1);
    wait_for_file(&workspace.join(".codex-auto-dev/state/agents/REQ-0001.exit"));
}

#[test]
fn tick_rejects_invalid_parallel_limit_without_panic() {
    let workspace = temp_workspace("tick-invalid-parallel");
    assert_success(&run(
        &workspace,
        &["new", "--name", "tick-invalid-parallel-test"],
    ));

    let output = run(&workspace, &["tick", "--parallel-limit", "0"]);
    assert_failure_contains(&output, "--parallel-limit must be greater than 0");
}

#[test]
fn tick_parallel_limit_flag_dispatches_multiple_pending_issue_agents_without_waiting() {
    let workspace = temp_workspace("tick");
    assert_success(&run(&workspace, &["new", "--name", "tick-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond tick request\\tSecond body\\thttps://example.test/2\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s prompt=%s\n' "$CODEX_AUTO_DEV_REQUEST_ID" "$CODEX_AUTO_DEV_AGENT_PHASE" "$CODEX_AUTO_DEV_MAX_ATTEMPTS" "$CODEX_AUTO_DEV_ISSUE_AGENT_PROMPT" >> .codex-auto-dev/state/agent.log
"#,
    )
    .expect("issue agent should be writable");

    let output = run(
        &workspace,
        &["tick", "--max-attempts", "20", "--parallel-limit", "2"],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tick dispatched 2 issue-agent(s)."));
    assert!(stdout.contains("Dispatched REQ-0001"));
    assert!(stdout.contains("Dispatched REQ-0002"));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("REQ-0001"));
    assert!(state.contains("REQ-0002"));
    assert_eq!(state.matches("planning-agent-running").count(), 2);
    assert!(!state.contains("finished"));
    let agent_log = workspace.join(".codex-auto-dev/state/agent.log");
    wait_for_file_contains(
        &agent_log,
        "agent called for REQ-0001 phase=planning max=20",
    );
    wait_for_file_contains(
        &agent_log,
        "agent called for REQ-0002 phase=planning max=20",
    );
    wait_for_file(&workspace.join(".codex-auto-dev/state/agents/REQ-0001.exit"));
    wait_for_file(&workspace.join(".codex-auto-dev/state/agents/REQ-0002.exit"));
    assert!(
        workspace
            .join(".codex-auto-dev/state/agents/REQ-0001.pid")
            .is_file()
    );
    assert!(
        workspace
            .join(".codex-auto-dev/state/agents/REQ-0002.pid")
            .is_file()
    );
}

#[test]
fn tick_refreshes_approved_agent_result_to_wait_update_pr() {
    let workspace = temp_workspace("tick-refresh");
    assert_success(&run(&workspace, &["new", "--name", "tick-refresh-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s\n' "$CODEX_AUTO_DEV_REQUEST_ID" "$CODEX_AUTO_DEV_AGENT_PHASE" "$CODEX_AUTO_DEV_MAX_ATTEMPTS" >> .codex-auto-dev/state/agent.log
case "$CODEX_AUTO_DEV_AGENT_PHASE" in
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$CODEX_AUTO_DEV_PLAN"
    printf '\n## 第 1 轮 - planning\n- 填写计划。\n' >> "$CODEX_AUTO_DEV_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$CODEX_AUTO_DEV_WORKTREE/feature.txt"
    printf '# 变更文档\n\n## 摘要\n\n已实现。\n\n## 实现前后对比\n\n已记录。\n\n## 关键设计点\n\n已记录。\n\n## 验证证据\n\n测试通过。\n\n## Review 结果\n\n等待汇总。\n' > "$CODEX_AUTO_DEV_CHANGE_DOC"
    printf '\n## 第 2 轮 - implementation\n- 完成实现。\n' >> "$CODEX_AUTO_DEV_AGENT_JOURNAL"
    ;;
  *)
    echo "unexpected phase: $CODEX_AUTO_DEV_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tick dispatched 1 issue-agent(s)."));

    let changes_path = workspace.join("docs/changes");
    let change_dir = fs::read_dir(changes_path)
        .expect("changes dir should be readable")
        .next()
        .expect("one change should exist")
        .expect("change entry should be readable")
        .path();
    wait_for_file(&workspace.join(".codex-auto-dev/state/agents/REQ-0001.exit"));
    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/agent.log"),
        "agent called for REQ-0001 phase=implementation max=20",
    );
    wait_for_file(&workspace.join(".codex-auto-dev/state/agents/REQ-0001.exit"));
    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/requests.tsv"),
        "wait-update-pr",
    );

    let refresh_output = run(&workspace, &["tick", "--request_id", "REQ-0001"]);
    assert_success(&refresh_output);
    let refresh_stdout = String::from_utf8_lossy(&refresh_output.stdout);
    assert!(refresh_stdout.contains("Tick complete: no pending request."));

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("wait-update-pr"));
    assert!(!state.contains("finished"));
    let change_doc =
        fs::read_to_string(change_dir.join("change-doc.md")).expect("change-doc readable");
    assert!(change_doc.contains("TestReviewer"));
    assert!(change_doc.contains("DesignReviewer"));
    assert!(
        change_dir
            .join("reviews/code-review/details/001-test-reviewer.json")
            .is_file()
    );
    assert!(
        change_dir
            .join("reviews/code-review/details/001-design-reviewer.json")
            .is_file()
    );
}

#[test]
fn agent_exit_hook_advances_request_without_waiting_for_next_tick() {
    let workspace = temp_workspace("advance-hook");
    assert_success(&run(&workspace, &["new", "--name", "advance-hook-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tHook request\\tBody from issue\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$CODEX_AUTO_DEV_REQUEST_ID" "$CODEX_AUTO_DEV_AGENT_PHASE" >> .codex-auto-dev/state/agent.log
case "$CODEX_AUTO_DEV_AGENT_PHASE" in
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$CODEX_AUTO_DEV_PLAN"
    printf '\n## hook - planning\n- 填写计划。\n' >> "$CODEX_AUTO_DEV_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$CODEX_AUTO_DEV_WORKTREE/feature.txt"
    printf '# 变更文档\n\n## 摘要\n\n已实现。\n\n## 实现前后对比\n\n已记录。\n\n## 关键设计点\n\n已记录。\n\n## 验证证据\n\n测试通过。\n\n## Review 结果\n\n等待汇总。\n' > "$CODEX_AUTO_DEV_CHANGE_DOC"
    printf '\n## hook - implementation\n- 完成实现。\n' >> "$CODEX_AUTO_DEV_AGENT_JOURNAL"
    ;;
  *)
    echo "unexpected phase: $CODEX_AUTO_DEV_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);

    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/agent.log"),
        "agent called for REQ-0001 phase=implementation",
    );
    wait_for_file_contains(
        &workspace.join(".codex-auto-dev/state/requests.tsv"),
        "wait-update-pr",
    );
    let change_dir = fs::read_dir(workspace.join("docs/changes"))
        .expect("changes dir should be readable")
        .next()
        .expect("one change should exist")
        .expect("change entry should be readable")
        .path();
    assert!(
        change_dir
            .join("approvals/change-doc.approval.json")
            .is_file()
    );
    assert!(
        workspace
            .join(".codex-auto-dev/state/agents/REQ-0001.hook.log")
            .is_file()
    );
}

#[test]
fn tick_blocks_stale_agent_running_without_exit_code() {
    let workspace = temp_workspace("tick-stale-agent");
    let change_name = format!("{}-stale-agent", current_date());
    assert_success(&run(&workspace, &["new", "--name", "tick-stale-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\n:\n",
    )
    .expect("issue connector should be replaceable");
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let state_path = workspace.join(".codex-auto-dev/state/requests.tsv");
    let state = fs::read_to_string(&state_path).expect("state should be readable");
    fs::write(
        &state_path,
        state.replace("\tplanning\t", "\tplanning-agent-running\t"),
    )
    .expect("state should be writable");
    let agents_dir = workspace.join(".codex-auto-dev/state/agents");
    fs::create_dir_all(&agents_dir).expect("agents dir should be writable");
    fs::write(agents_dir.join("REQ-0001.pid"), "999999\n").expect("pid should be writable");

    let refresh_output = run(&workspace, &["tick", "--request_id", "REQ-0001"]);
    assert_success(&refresh_output);
    let refresh_stdout = String::from_utf8_lossy(&refresh_output.stdout);
    assert!(refresh_stdout.contains("Tick refreshed 1 request status(es)."));
    assert!(refresh_stdout.contains("Tick complete: no pending request."));

    let state = fs::read_to_string(&state_path).expect("state should be readable");
    assert!(state.contains("blocked"));
    let status = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(change_name)
            .join("status.json"),
    )
    .expect("status should be readable");
    assert!(status.contains("planning agent pid"));
    assert!(status.contains("no exit code was written"));
}

#[test]
fn block_and_resume_create_recovery_package() {
    let workspace = temp_workspace("block");
    let change_name = format!("{}-blocked-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "block-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let output = run(
        &workspace,
        &[
            "block",
            "--request_id",
            "REQ-0001",
            "--stage",
            "implementation",
            "--reason",
            "code-review failed after 20 attempts",
        ],
    );
    assert_success(&output);
    let change_path = workspace.join("docs/changes").join(&change_name);
    let status = fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(status.contains("\"status\": \"blocked\""));
    assert!(status.contains("code-review failed after 20 attempts"));
    let recovery = fs::read_to_string(change_path.join("recovery.md")).expect("recovery readable");
    assert!(recovery.contains("恢复指南"));
    assert!(recovery.contains("code-review failed after 20 attempts"));

    let resume = run(&workspace, &["resume", "--request_id", "REQ-0001"]);
    assert_success(&resume);
    let stdout = String::from_utf8_lossy(&resume.stdout);
    assert!(stdout.contains("recovery.md"));
    assert!(stdout.contains("agent-journal.md"));
    assert!(stdout.contains("codex-auto-dev tick --request_id REQ-0001"));
    assert!(stdout.contains("resumed status: planning"));

    let resumed_state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("requests state readable");
    assert!(resumed_state.contains("REQ-0001"));
    assert!(resumed_state.contains("\tplanning\t"));
    let resumed_status =
        fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(resumed_status.contains("\"status\": \"planning\""));
    assert!(!resumed_status.contains("\"status\": \"blocked\""));

    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\n",
    )
    .expect("issue update should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'phase=%s\\n' \"$CODEX_AUTO_DEV_AGENT_PHASE\" >> .codex-auto-dev/state/resume-agent.log\n",
    )
    .expect("issue agent should be writable");
    let tick = run(&workspace, &["tick", "--request_id", "REQ-0001"]);
    assert_success(&tick);
    let tick_stdout = String::from_utf8_lossy(&tick.stdout);
    assert!(tick_stdout.contains("Dispatched REQ-0001 phase planning"));
}

#[test]
fn doctor_reports_workspace_and_reviewer_readiness() {
    let workspace = temp_workspace("doctor");
    assert_success(&run(&workspace, &["new", "--name", "doctor-test"]));

    let output = run(&workspace, &["doctor"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Codex Auto Dev Doctor Report"));
    assert!(stdout.contains("Workspace"));
    assert!(stdout.contains("Review schema"));
    assert!(stdout.contains("Reviewer tools"));
    assert!(stdout.contains("Agent tools"));
    assert!(stdout.contains("CodeGraph CLI"));
    assert!(stdout.contains("CodeGraph index"));
}

#[test]
fn events_stream_records_discovery_planning_and_dispatch() {
    let workspace = temp_workspace("events");
    assert_success(&run(&workspace, &["new", "--name", "events-test"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tObservable request\\tBody\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nprintf 'agent started\\n'\n",
    )
    .expect("issue agent should be writable");

    assert_success(&run(&workspace, &["update"]));
    let change_name = format!("{}-observable-request", current_date());
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(&workspace, &["tick", "--request_id", "REQ-0001"]));

    let events = fs::read_to_string(workspace.join(".codex-auto-dev/state/events.ndjson"))
        .expect("events stream should be readable");
    assert!(events.contains("\"event\": \"workspace_initialized\""));
    assert!(events.contains("\"event\": \"request_discovered\""));
    assert!(events.contains("\"event\": \"change_packet_created\""));
    assert!(events.contains("\"event\": \"agent_dispatched\""));
    assert!(events.contains("\"request_id\": \"REQ-0001\""));
}

#[test]
fn code_review_can_recommend_returning_to_planning() {
    let workspace = temp_workspace("review-return-planning");
    let change_name = format!("{}-return-planning", current_date());
    assert_success(&run(&workspace, &["new", "--name", "return-planning-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    assert_success(&run(
        &workspace,
        &[
            "approve",
            "--request_id",
            "REQ-0001",
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(&workspace, &["start", "--request_id", "REQ-0001"]));

    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"approved plan misses migration strategy\",\"process\":[\"checked plan\",\"checked diff\"],\"critical\":[],\"high\":[{\"title\":\"plan lacks migration strategy\",\"evidence\":\"approved plan does not describe migration for changed storage format\",\"impact\":\"implementation may corrupt or strand existing data without a migration path\",\"required_fix\":\"return to planning and add migration strategy before implementation continues\",\"suggested_change\":\"Update plan.md with migration steps, rollback behavior, compatibility tests, and whether existing data must be transformed.\",\"verification\":\"Rerun plan-review and confirm the updated plan contains migration and rollback validation.\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");

    let rejected = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected, "DesignReviewer rejected");

    let state = fs::read_to_string(workspace.join(".codex-auto-dev/state/requests.tsv"))
        .expect("request state should be readable");
    assert!(state.contains("plan-review-rejected"));
    let summary = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("reviews/code-review/summary.json"),
    )
    .expect("code review summary should be readable");
    assert!(summary.contains("\"recommended_next_phase\": \"planning\""));
    let status = fs::read_to_string(
        workspace
            .join("docs/changes")
            .join(&change_name)
            .join("status.json"),
    )
    .expect("status should be readable");
    assert!(status.contains("\"return_to_phase_reason\""));
    assert!(status.contains("code-review requested planning"));
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

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs()
}
