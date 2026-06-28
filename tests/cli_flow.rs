use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SOURCE_SKILL: &str = include_str!("../skills/sandrone/SKILL.md");
const MAIN_RS: &str = include_str!("../src/main.rs");
const JOBS_RS: &str = include_str!("../src/jobs.rs");

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sandrone"))
}

fn temp_workspace(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("sandrone-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).expect("temp workspace should be created");
    path
}

fn run(workspace: &Path, args: &[&str]) -> Output {
    Command::new(bin())
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("sandrone command should run")
}

fn run_with_env(workspace: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command.args(args).current_dir(workspace);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("sandrone command should run")
}

fn run_sdr_alias(workspace: &Path, args: &[&str]) -> Output {
    let alias = std::env::var_os("CARGO_BIN_EXE_sdr").expect("sdr alias should be built");
    Command::new(alias)
        .args(args)
        .current_dir(workspace)
        .output()
        .expect("sdr command should run")
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

fn request_artifact_name(request_id: &str, file: &str) -> String {
    format!("{request_id} {file}")
}

fn request_artifact_path(change_path: &Path, request_id: &str, file: &str) -> PathBuf {
    change_path.join(request_artifact_name(request_id, file))
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

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("executable test fixture should be writable");
    let mut permissions = fs::metadata(path)
        .expect("executable test fixture metadata readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("executable test fixture should be executable");
}

fn install_passing_decomposition_review_tool(workspace: &Path) {
    write_executable(
        &workspace.join("tools/decomposition-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DecompositionReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"planning\",\"summary\":\"decomposition ok\",\"process\":[\"checked decomposition\",\"checked dag\",\"checked coverage\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );
}

fn install_deterministic_request_scheduler(workspace: &Path) {
    write_executable(
        &workspace.join("tools/request-schedule-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
max="${SANDRONE_REQUEST_SCHEDULE_MAX_PARALLEL:-1}"
awk -F '	' -v max="$max" '
  NR > 1 && count < max {
    count += 1
    printf "selected\t%s\ttest scheduler selected slot %d/%d\n", $1, count, max
  }
  END {
    if (count == 0) {
      printf "defer\t\tno candidate\n"
    }
  }
' "$SANDRONE_REQUEST_SCHEDULE_QUEUE"
"#,
    );
    write_executable(
        &workspace.join("tools/request-schedule-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"RequestScheduleReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"request schedule ok\",\"process\":[\"checked selected ids\",\"checked parallel limit\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );
}

fn write_active_loop_cohort(workspace: &Path, request_ids: &[&str]) {
    let scheduler_dir = workspace.join(".sandrone/state/scheduler");
    fs::create_dir_all(&scheduler_dir).expect("scheduler state dir writable");
    let request_ids_json = request_ids
        .iter()
        .map(|request_id| format!("\"{request_id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        scheduler_dir.join("cohort.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"cohort_id\": \"test-cohort\",\n  \"status\": \"active\",\n  \"created_at\": \"test\",\n  \"updated_at\": \"test\",\n  \"request_ids\": [{request_ids_json}]\n}}\n"
        ),
    )
    .expect("cohort file writable");
}

fn decomposition_agent_case() -> &'static str {
    r#"  decomposition)
    change_dir="$(dirname "$SANDRONE_DECOMPOSITION")"
    printf '# 拆解\n\n## Slice\n\n- S01 main\n' > "$SANDRONE_DECOMPOSITION"
    printf '{"schema_version":1,"request_id":"%s","kind":"request_decomposition","classification":{"slice_strategy":"single-or-dag"},"slices":[{"id":"S01","name":"main","status":"draft","type":"feature","summary":"single slice","depends_on":[],"parallel_group":"main","conflict_domains":[],"inputs":[],"outputs":[],"acceptance":["done"],"tests":["covered"],"docs":[],"branch":"codex/%s-s01-main","worktree":""}],"global_invariants":[],"updated_at":"test"}\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_REQUEST_ID" > "$change_dir/decomposition.json"
    printf '{"schema_version":1,"request_id":"%s","nodes":[{"id":"S01","name":"main","status":"draft","depends_on":[],"parallel_group":"main","conflict_domains":[],"branch":"codex/%s-s01-main","worktree":""}],"edges":[],"parallel_policy":{"max_parallel_slices":1,"respect_conflict_domains":true},"updated_at":"test"}\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_REQUEST_ID" > "$SANDRONE_DAG"
    printf '\n## decomposition\n- 完成单 slice 拆解。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
"#
}

fn force_request_state(
    workspace: &Path,
    request_id: &str,
    status: &str,
    branch: &str,
    worktree: &str,
) {
    let state_path = workspace.join(".sandrone/state/requests.tsv");
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

fn prepare_wait_finish_request(workspace: &Path, request_id: &str, change_name: &str) {
    assert_success(&run(
        workspace,
        &["plan", "--name", change_name, "--request_id", request_id],
    ));
    assert_success(&run(
        workspace,
        &["submit", "--request_id", request_id, "--gate", "plan"],
    ));
    assert_success(&run(
        workspace,
        &[
            "approve",
            "--request_id",
            request_id,
            "--gate",
            "plan",
            "--by",
            "tester",
        ],
    ));
    assert_success(&run(workspace, &["start", "--request_id", request_id]));
    assert_success(&run(
        workspace,
        &["submit", "--request_id", request_id, "--gate", "change-doc"],
    ));
    assert_success(&run(
        workspace,
        &[
            "approve",
            "--request_id",
            request_id,
            "--gate",
            "change-doc",
            "--by",
            "tester",
        ],
    ));
    force_request_state(
        workspace,
        request_id,
        "wait-finish",
        &format!("codex/{}", request_id.to_ascii_lowercase()),
        &format!("dev/worktrees/{request_id}"),
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

fn wait_for_file_contains_any(path: &Path, expected: &[&str]) {
    for _ in 0..100 {
        if path.exists() {
            let content = fs::read_to_string(path).expect("waited file should be readable");
            if expected.iter().any(|value| content.contains(value)) {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "timed out waiting for file content: {} should contain one of {:?}",
        path.display(),
        expected
    );
}

fn advance_until_not_review_running(workspace: &Path, request_id: &str) {
    for _ in 0..100 {
        let state =
            fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).unwrap_or_default();
        let line = state
            .lines()
            .find(|line| line.starts_with(&format!("{request_id}\t")))
            .unwrap_or("");
        if !line.contains("review-running") {
            return;
        }
        let output = Command::new(bin())
            .args(["advance", "--request_id", request_id])
            .current_dir(workspace)
            .output()
            .expect("advance command should run");
        assert!(
            output.status.success(),
            "advance failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let state =
            fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).unwrap_or_default();
        let line = state
            .lines()
            .find(|line| line.starts_with(&format!("{request_id}\t")))
            .unwrap_or("");
        if !line.contains("review-running") {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for async review to finish for {request_id}");
}

fn advance_until_request_state_contains(workspace: &Path, request_id: &str, expected: &str) {
    for _ in 0..200 {
        let state =
            fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).unwrap_or_default();
        let line = state
            .lines()
            .find(|line| line.starts_with(&format!("{request_id}\t")))
            .unwrap_or("");
        if line.contains(expected) {
            return;
        }
        let output = Command::new(bin())
            .args(["advance", "--request_id", request_id])
            .current_dir(workspace)
            .output()
            .expect("advance command should run");
        assert!(
            output.status.success(),
            "advance failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {request_id} state to contain {expected}");
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
    assert!(stdout.contains("Public commands:"));
    assert!(stdout.contains("loop start [--interval-seconds 900]"));
    assert!(stdout.contains("loop restart [--request_id <REQ-0001>]"));
    assert!(stdout.contains("loop stop [--request_id <REQ-0001>]"));
    assert!(stdout.contains("dashboard [--host 127.0.0.1]"));
    assert!(stdout.contains("Internal commands still exist"));
    assert!(!stdout.contains("tick [--request_id <REQ-0001>]"));
    assert!(!stdout.contains("pr-refresh --request_id <REQ-0001>"));
}

#[test]
fn sdr_alias_prints_the_same_cli_help() {
    let workspace = temp_workspace("sdr-alias");
    let output = run_sdr_alias(&workspace, &["--help"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: sandrone <command>"));
    assert!(stdout.contains("dashboard"));
}

#[test]
fn new_refuses_to_initialize_the_framework_source_checkout() {
    let workspace = temp_workspace("self-guard");
    fs::create_dir_all(workspace.join("src")).expect("src dir writable");
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"sandrone\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("Cargo.toml writable");
    fs::write(workspace.join("src/main.rs"), "fn main() {}\n").expect("main.rs writable");
    fs::create_dir_all(workspace.join("templates")).expect("templates dir writable");
    fs::create_dir_all(workspace.join("skills/sandrone")).expect("skill dir writable");

    let output = run(&workspace, &["new", "--name", "should-not-create-dev"]);

    assert_failure_contains(&output, "refusing to initialize sandrone source checkout");
    assert!(!workspace.join("dev").exists());
    assert!(!workspace.join(".sandrone").exists());
    assert!(!workspace.join("tools").exists());
}

#[test]
fn templates_are_external_assets_not_embedded_in_main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "assets/dashboard/index.html",
        "templates/prompts/issue-agent.md",
        "templates/prompts/decomposition-agent.md",
        "templates/prompts/plan-agent.md",
        "templates/prompts/implementation-agent.md",
        "templates/prompts/decomposition-reviewer.md",
        "templates/prompts/rebase-agent.md",
        "templates/prompts/plan-reviewer.md",
        "templates/prompts/test-reviewer.md",
        "templates/prompts/design-reviewer.md",
        "templates/prompts/integration-reviewer.md",
        "templates/prompts/request-schedule-reviewer.md",
        "templates/runtime/plan.md",
        "templates/runtime/decomposition.md",
        "templates/runtime/decomposition.json",
        "templates/runtime/dag.json",
        "templates/runtime/change-doc.md",
        "templates/runtime/obsidian-change.md",
        "templates/runtime/obsidian-project.md",
        "templates/runtime/obsidian-relations.md",
        "templates/runtime/obsidian-requests.base",
        "templates/runtime/obsidian-slices.base",
        "templates/schemas/review-result.schema.json",
        "templates/scripts/issue-update.sh",
        "templates/scripts/issue-agent.sh",
        "templates/scripts/check-format.sh",
        "templates/scripts/rebase-agent.sh",
        "templates/scripts/review-tool.sh",
        "templates/scripts/pr-create.sh",
        "templates/scripts/pr-status.sh",
        "templates/scripts/request-schedule-agent.sh",
        "templates/scripts/request-schedule-review.sh",
        "templates/scripts/pr-merge.sh",
        "src/assets.rs",
        "src/dashboard.rs",
        "src/delivery.rs",
        "src/defaults.rs",
        "src/doctor.rs",
        "src/help.rs",
        "src/loop_runner.rs",
        "src/request_schedule.rs",
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
    let legacy_skill = codex_home.join("skills").join("codex-auto-dev-workflow");
    fs::create_dir_all(&legacy_skill).expect("legacy skill dir writable");
    fs::write(legacy_skill.join("SKILL.md"), "legacy\n").expect("legacy skill writable");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new("sh")
        .args([
            "scripts/install.sh",
            "--skill-only",
            "--dest",
            codex_home.to_str().expect("temp path should be utf-8"),
        ])
        .current_dir(&repo_root)
        .env("SANDRONE_SKIP_CODEGRAPH_INSTALL", "1")
        .output()
        .expect("install script should run");
    assert_success(&output);

    let installed_skill =
        fs::read_to_string(codex_home.join("skills").join("sandrone").join("SKILL.md"))
            .expect("installed skill should be readable");
    assert_eq!(installed_skill, SOURCE_SKILL);
    assert!(
        !legacy_skill.exists(),
        "installer should remove the legacy workflow skill after rename"
    );
    assert!(
        codex_home
            .join("skills")
            .join("obsidian-change-trace")
            .join("SKILL.md")
            .is_file(),
        "installer should include bundled obsidian-change-trace skill"
    );
    assert!(
        !codex_home
            .join("skills")
            .join("obsidian-note")
            .join("SKILL.md")
            .exists(),
        "installer must not overwrite the user's personal obsidian-note skill"
    );
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
    assert!(stdout.contains("raw.githubusercontent.com/ZhmYe/Sandrone"));
    assert!(stdout.contains("scripts/install.sh --force"));
}

#[test]
fn skill_requires_install_or_verify_cli_before_workspace_commands() {
    assert!(
        SOURCE_SKILL.lines().count() < 180,
        "skill should stay compact so child agents do not ingest the whole manual"
    );
    assert!(SOURCE_SKILL.contains("## 必做第一步: 安装或验证 CLI"));
    assert!(SOURCE_SKILL.contains("Before any workspace command"));
    assert!(SOURCE_SKILL.contains("sandrone --help"));
    assert!(SOURCE_SKILL.contains("Do not run workspace commands until"));
    assert!(SOURCE_SKILL.contains("bootstrap.sh | sh"));
    assert!(SOURCE_SKILL.contains("## 上下文预算"));
    assert!(SOURCE_SKILL.contains("必要 skill/plugin 可以用"));
    assert!(SOURCE_SKILL.contains("SANDRONE_AGENT_IGNORE_USER_CONFIG"));
    assert!(SOURCE_SKILL.contains("自动化子 agent 默认不继承用户个人 Codex config"));
    assert!(SOURCE_SKILL.contains("--parallel-limit"));
    assert!(SOURCE_SKILL.contains("parallel_limit = 1"));
    assert!(SOURCE_SKILL.contains("sandrone loop start"));
    assert!(SOURCE_SKILL.contains("sandrone loop restart"));
    assert!(SOURCE_SKILL.contains("sandrone loop stop"));
    assert!(SOURCE_SKILL.contains("RequestScheduleAgent"));
    assert!(SOURCE_SKILL.contains("RequestScheduleReviewer"));
    assert!(SOURCE_SKILL.contains("PR 交付和合并默认自动串行执行"));
    assert!(SOURCE_SKILL.contains("SANDRONE_REVIEW_CONTEXT"));
    assert!(SOURCE_SKILL.contains("不得读取其他 reviewer 输出"));
    assert!(SOURCE_SKILL.contains("SANDRONE_AGENT_STATUS_DOC"));
    assert!(SOURCE_SKILL.contains("不能替代 reviewer"));
    assert!(SOURCE_SKILL.contains("审批/门禁是显式状态化流程"));
    assert!(SOURCE_SKILL.contains("交付文档中的 checklist 必须全部打勾"));
    assert!(SOURCE_SKILL.contains("无法由当前流程完成的事项不得保留为未勾选 checklist"));
    assert!(SOURCE_SKILL.contains("docs/workspace-layout.md"));
    assert!(SOURCE_SKILL.contains("安装态 `~/.codex/skills/sandrone` 默认只包含本 `SKILL.md`"));
    assert!(SOURCE_SKILL.contains("sandrone dashboard"));
    assert!(SOURCE_SKILL.contains("需求分析"));
    assert!(SOURCE_SKILL.contains("Slice 1"));
    assert!(SOURCE_SKILL.contains("Review 结果"));
    assert!(SOURCE_SKILL.contains("marked"));
    assert!(SOURCE_SKILL.contains("jsoneditor"));
    assert!(SOURCE_SKILL.contains("PlanReviewer 提交前自检"));
    assert!(SOURCE_SKILL.contains("Code Review 提交前自检"));
    assert!(SOURCE_SKILL.contains("docs/development.md"));
    assert!(SOURCE_SKILL.contains("tools/prompts/issue-agent.md"));
    assert!(SOURCE_SKILL.contains("PR 状态门禁"));
    assert!(SOURCE_SKILL.contains("退回 implementation"));
    assert!(SOURCE_SKILL.contains("ImplementationAgent 必须保留 base/master 新代码"));
}

#[test]
fn agent_wrapper_falls_back_from_legacy_binary_names() {
    assert!(MAIN_RS.contains("create_truncated_runtime_file"));
    assert!(JOBS_RS.contains(".truncate(true)"));
    assert!(MAIN_RS.contains("SANDRONE_AGENT_STATUS_DOC"));
    assert!(MAIN_RS.contains("agent_document_status_is_submitted"));
    assert!(MAIN_RS.contains("agent_document_status_used"));
    assert!(MAIN_RS.contains("resolve_sandrone_bin()"));
    assert!(MAIN_RS.contains("command -v sandrone"));
    assert!(MAIN_RS.contains("command -v sdr"));
    assert!(MAIN_RS.contains("codex-auto-dev)"));
    assert!(MAIN_RS.contains("Sandrone-agent-wrapper: sandrone CLI not found"));
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
    let config = fs::read_to_string(workspace.join(".sandrone/config.toml"))
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
    assert!(workspace.join("tools/request-schedule-agent.sh").is_file());
    assert!(workspace.join("tools/request-schedule-review.sh").is_file());
    assert!(workspace.join("tools/pr-merge.sh").is_file());
    assert!(
        workspace
            .join("agents/config/implementation-agent.json")
            .is_file()
    );
    assert!(workspace.join("agents/config/test-reviewer.json").is_file());
    assert!(
        workspace
            .join("agents/config/request-schedule-agent.json")
            .is_file()
    );
    assert!(
        workspace
            .join("agents/config/request-schedule-reviewer.json")
            .is_file()
    );
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
            .join("tools/prompts/request-schedule-reviewer.md")
            .is_file()
    );
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
        ("tools/pr-merge.sh", "tools/pr-merge.example.sh"),
        (
            "tools/request-schedule-agent.sh",
            "tools/request-schedule-agent.example.sh",
        ),
        (
            "tools/request-schedule-review.sh",
            "tools/request-schedule-review.example.sh",
        ),
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
            "tools/prompts/request-schedule-reviewer.md",
            "tools/prompts/request-schedule-reviewer.example.md",
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
    assert!(issue_agent.contains("SANDRONE_REQUEST_SOURCE"));
    assert!(issue_agent.contains("SANDRONE_AGENT_PHASE"));
    assert!(issue_agent.contains("trap '' HUP"));
    assert!(issue_agent.contains("nohup \"$codex_bin\" exec"));
    assert!(issue_agent.contains("SANDRONE_AGENT_RESUME_SESSION_ID"));
    assert!(issue_agent.contains("exec resume"));
    assert!(issue_agent.contains("Resume session id"));
    assert!(issue_agent.contains("SANDRONE_CODEX_BIN"));
    assert!(issue_agent.contains("SANDRONE_CODEX_APP"));
    assert!(issue_agent.contains("resolve_codex_bin"));
    assert!(!issue_agent.contains("/Applications/Codex.app"));
    assert!(issue_agent.contains("MUST NOT call sandrone approve/reject"));
    assert!(issue_agent.contains("gate_unavailable=true"));
    assert!(issue_agent.contains("shell_environment_policy.inherit"));
    assert!(issue_agent.contains("Latest review detail files"));
    assert!(issue_agent.contains("SANDRONE_LATEST_ACTIONABLE_REVIEW_DETAILS"));
    assert!(issue_agent.contains("SANDRONE_AGENT_STATUS_DOC"));
    assert!(issue_agent.contains("Agent status doc"));
    assert!(issue_agent.contains("SANDRONE_AGENT_IGNORE_USER_CONFIG"));
    assert!(issue_agent.contains("resolve_agent_ignore_user_config"));
    assert!(issue_agent.contains("value:-1"));
    assert!(issue_agent.contains("--ignore-user-config"));
    let issue_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/issue-agent.md"))
        .expect("issue agent prompt readable");
    assert!(issue_agent_prompt.contains("共享 agent 契约"));
    assert!(issue_agent_prompt.contains("## 绝对边界"));
    assert!(issue_agent_prompt.contains("上下文预算与读取顺序"));
    assert!(issue_agent_prompt.contains("Skill/plugin 使用原则"));
    assert!(issue_agent_prompt.contains("完整 `skills/sandrone/SKILL.md`"));
    assert!(issue_agent_prompt.contains("## Journal 格式"));
    assert!(issue_agent_prompt.contains("每条 reviewer critical/high 都必须有对应处理说明"));
    assert!(issue_agent_prompt.contains("## Reviewer 提交前自检"));
    assert!(issue_agent_prompt.contains("PlanReviewer 提交前自检"));
    assert!(issue_agent_prompt.contains("TestReviewer"));
    assert!(issue_agent_prompt.contains("DesignReviewer"));
    assert!(issue_agent_prompt.contains("历史 `gate_unavailable` 不是 block 理由"));
    assert!(issue_agent_prompt.contains("## 文档提交状态"));
    assert!(issue_agent_prompt.contains("agent_status: submitted"));
    assert!(issue_agent_prompt.contains("agent_ready_for_review: true"));
    assert!(issue_agent_prompt.contains("不能替代 reviewer"));
    let decomposition_agent_prompt =
        fs::read_to_string(workspace.join("tools/prompts/decomposition-agent.md"))
            .expect("decomposition agent prompt readable");
    assert!(decomposition_agent_prompt.contains("只把它当作历史诊断记录到 journal"));
    assert!(
        !decomposition_agent_prompt
            .contains("如果 summary 中任一 reviewer 的 `gate_unavailable` 为 `true`，立即 block")
    );
    let plan_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/plan-agent.md"))
        .expect("plan agent prompt readable");
    assert!(plan_agent_prompt.contains("## 启动前检查"));
    assert!(plan_agent_prompt.contains("## Plan 必须包含"));
    assert!(plan_agent_prompt.contains("## PlanReviewer 提交前自检清单"));
    assert!(plan_agent_prompt.contains("逐项核对 PlanReviewer"));
    assert!(plan_agent_prompt.contains("不得退出交给 PlanReviewer"));
    assert!(plan_agent_prompt.contains("不运行 `submit`、`plan-review`"));
    assert!(plan_agent_prompt.contains("只把它当作历史诊断记录到 journal"));
    assert!(
        !plan_agent_prompt
            .contains("如果 summary 中任一 reviewer 的 `gate_unavailable` 为 `true`，立即 block")
    );
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
    assert!(implementation_agent_prompt.contains("不要再次 block"));
    assert!(
        !implementation_agent_prompt
            .contains("如果 summary 中任一 reviewer 的 `gate_unavailable` 为 `true`，立即 block")
    );
    assert!(implementation_agent_prompt.contains("PR 状态门禁退回"));
    assert!(implementation_agent_prompt.contains("base/master drift"));
    assert!(implementation_agent_prompt.contains("不得 commit、push、finish、merge"));
    let rebase_agent =
        fs::read_to_string(workspace.join("tools/rebase-agent.sh")).expect("rebase tool readable");
    assert!(rebase_agent.contains("Connector contract"));
    assert!(rebase_agent.contains("SANDRONE_AGENT_PHASE"));
    assert!(rebase_agent.contains("rebase"));
    assert!(rebase_agent.contains("resolve_codex_bin"));
    assert!(rebase_agent.contains("SANDRONE_AGENT_IGNORE_USER_CONFIG"));
    assert!(rebase_agent.contains("resolve_agent_ignore_user_config"));
    assert!(rebase_agent.contains("--ignore-user-config"));
    assert!(rebase_agent.contains("SANDRONE_AGENT_STATUS_DOC"));
    let rebase_agent_prompt = fs::read_to_string(workspace.join("tools/prompts/rebase-agent.md"))
        .expect("rebase agent prompt readable");
    assert!(rebase_agent_prompt.contains("RebaseAgent"));
    assert!(rebase_agent_prompt.contains("保留 base/master"));
    assert!(rebase_agent_prompt.contains("不能为了自己分支的修改删除 base/master 新代码"));
    assert!(rebase_agent_prompt.contains("不得扩大需求范围"));
    assert!(rebase_agent_prompt.contains("SANDRONE_AGENT_STATUS_DOC"));
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
    let pr_merge_tool =
        fs::read_to_string(workspace.join("tools/pr-merge.sh")).expect("pr merge tool readable");
    assert!(pr_merge_tool.contains("Connector contract"));
    assert!(pr_merge_tool.contains("SANDRONE_PR_STATUS"));
    assert!(pr_merge_tool.contains("pr-status reported a safe merge state"));
    assert!(pr_merge_tool.contains("merged<TAB>url<TAB>detail"));
    assert!(pr_merge_tool.contains("blocked<TAB>url<TAB>detail"));
    let review_tool =
        fs::read_to_string(workspace.join("tools/plan-review.sh")).expect("review tool readable");
    assert!(review_tool.contains("Connector contract"));
    assert!(review_tool.contains("exactly one JSON object"));
    assert!(review_tool.contains("gate_unavailable=true"));
    assert!(review_tool.contains("SANDRONE_REVIEW_CONTEXT"));
    assert!(review_tool.contains("artifact-index.md"));
    assert!(review_tool.contains("SANDRONE_REVIEW_FORBIDDEN_PATHS"));
    assert!(review_tool.contains("SANDRONE_REVIEW_CODEX_HOME"));
    assert!(review_tool.contains("SANDRONE_REVIEW_BACKEND"));
    assert!(review_tool.contains("codex-cli"));
    assert!(review_tool.contains("codex-api"));
    assert!(review_tool.contains("claude-code"));
    assert!(review_tool.contains("LLM_API_KEY"));
    assert!(review_tool.contains("LLM_BASE_URL"));
    assert!(review_tool.contains("model_provider"));
    assert!(review_tool.contains("model_providers."));
    assert!(review_tool.contains("SANDRONE_CODEX_WIRE_API"));
    assert!(review_tool.contains("SANDRONE_CODEX_MODEL_CATALOG_JSON"));
    assert!(review_tool.contains("model_catalog_json"));
    assert!(review_tool.contains("debug models --bundled"));
    assert!(review_tool.contains("SANDRONE_CODEX_BIN"));
    assert!(review_tool.contains("SANDRONE_CODEX_APP"));
    assert!(review_tool.contains("resolve_codex_bin"));
    assert!(review_tool.contains("write_minimal_review_config"));
    assert!(review_tool.contains("plugin_hooks = false"));
    assert!(review_tool.contains("goals = false"));
    assert!(review_tool.contains("js_repl = false"));
    assert!(!review_tool.contains("cp \"$source_codex_home/config.toml\""));
    assert!(review_tool.contains("CODEX_HOME=\"$review_codex_home\" \"$codex_bin\" exec"));
    assert!(review_tool.contains("--output-last-message \"$review_output_file\""));
    assert!(review_tool.contains("- < \"$review_prompt_file\" 1>&2"));
    assert!(review_tool.contains("cat \"$review_output_file\""));
    assert!(!review_tool.contains("/Applications/Codex.app"));
    assert!(review_tool.contains("--ephemeral"));
    assert!(review_tool.contains("--ignore-user-config"));
    assert!(review_tool.contains("features.plugin_hooks=false"));
    assert!(review_tool.contains("--sandbox workspace-write"));
    assert!(review_tool.contains(
        "[ \"$review_backend\" = \"codex-cli\" ] || [ \"$review_backend\" = \"codex-api\" ]"
    ));
    let issue_agent =
        fs::read_to_string(workspace.join("tools/issue-agent.sh")).expect("agent tool readable");
    assert!(issue_agent.contains("SANDRONE_AGENT_BACKEND"));
    assert!(issue_agent.contains("SANDRONE_DECOMPOSITION_AGENT_BACKEND"));
    assert!(issue_agent.contains("SANDRONE_PLAN_AGENT_BACKEND"));
    assert!(issue_agent.contains("SANDRONE_IMPLEMENTATION_AGENT_BACKEND"));
    assert!(issue_agent.contains("codex-api"));
    assert!(issue_agent.contains("model_provider"));
    assert!(issue_agent.contains("model_providers."));
    assert!(issue_agent.contains("SANDRONE_CODEX_WIRE_API"));
    assert!(issue_agent.contains("SANDRONE_CODEX_MODEL_CATALOG_JSON"));
    assert!(issue_agent.contains("model_catalog_json"));
    assert!(issue_agent.contains(
        "[ \"$agent_backend\" = \"codex-cli\" ] || [ \"$agent_backend\" = \"codex-api\" ]"
    ));
    let env_example =
        fs::read_to_string(workspace.join(".env.example")).expect("workspace env example readable");
    assert!(env_example.contains("SANDRONE_AGENT_BACKEND=codex-api"));
    assert!(env_example.contains("SANDRONE_REVIEW_BACKEND=codex-cli"));
    assert!(env_example.contains("SANDRONE_REVIEW_BACKEND=codex-api"));
    assert!(env_example.contains("SANDRONE_REVIEW_BACKEND=claude-code"));
    assert!(env_example.contains("LLM_API_KEY="));
    assert!(env_example.contains("LLM_BASE_URL=https://api.openai.com/v1"));
    assert!(env_example.contains("SANDRONE_CODEX_MODEL_PROVIDER=sandrone-api"));
    assert!(env_example.contains("SANDRONE_CODEX_WIRE_API=responses"));
    assert!(env_example.contains("SANDRONE_CODEX_MODEL_CATALOG_JSON="));
    assert!(env_example.contains("SANDRONE_REVIEW_TIMEOUT_SECONDS=1800"));
    assert!(env_example.contains("SANDRONE_AGENT_IGNORE_USER_CONFIG=1"));
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
    assert!(workspace.join("skills/sandrone/SKILL.md").is_file());
    let generated_skill =
        fs::read_to_string(workspace.join("skills").join("sandrone").join("SKILL.md"))
            .expect("generated skill should be readable");
    assert_eq!(generated_skill, SOURCE_SKILL);
    assert!(generated_skill.starts_with("---\nname: sandrone"));
    assert!(workspace.join("obsidian/changes").is_dir());
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
        fs::read_dir(workspace.join("obsidian/changes"))
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
        &[("SANDRONE_HOME", registry_home_str)],
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
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    let list = run_with_env(
        &workspace,
        &["list"],
        &[("SANDRONE_HOME", registry_home_str)],
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
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    let upgraded_registry =
        fs::read_to_string(&registry_path).expect("workspace registry should survive upgrade");
    assert!(upgraded_registry.contains("\"repo_name\": \"registry-project\""));
    assert!(upgraded_registry.contains("\"last_status\": \"ready\""));
}

#[test]
fn dashboard_imports_legacy_registry_and_workspace_state_after_rename() {
    let home = temp_workspace("legacy-registry-home");
    let workspace = temp_workspace("legacy-workspace");
    let legacy_state = workspace.join(".codex-auto-dev/state");
    fs::create_dir_all(&legacy_state).expect("legacy state dir writable");
    fs::write(
        workspace.join(".codex-auto-dev/config.toml"),
        "schema_version = 2\nrepo_name = \"legacy-project\"\ngit_url = \"https://example.test/legacy.git\"\nbase_branch = \"master\"\nparallel_limit = 1\n",
    )
    .expect("legacy config writable");
    fs::write(
        legacy_state.join("requests.tsv"),
        "# Sandrone requests v2\nREQ-0001\tlegacy:1\ttest\tLegacy request\tLegacy body\thttps://example.test/legacy\tdiscovered\t2026-06-05-legacy-request\tobsidian/changes/2026-06-05-legacy-request\t\t\t1\t1\n",
    )
    .expect("legacy requests writable");
    let legacy_global = home.join(".codex-auto-dev");
    fs::create_dir_all(&legacy_global).expect("legacy global registry dir writable");
    fs::write(
        legacy_global.join("workspaces.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"workspaces\": [\n    {{ \"key\": \"{}\", \"repo_name\": \"legacy-project\", \"git_url\": \"https://example.test/legacy.git\", \"workspace_path\": \"{}\", \"target_repo\": \"{}/dev/repo\", \"last_status\": \"ready\", \"request_count\": 1, \"status_counts\": {{\"discovered\": 1}}, \"updated_at\": \"1\" }}\n  ]\n}}\n",
            workspace.display(),
            workspace.display(),
            workspace.display(),
        ),
    )
    .expect("legacy global registry writable");

    let output = run_with_env(
        &workspace,
        &["dashboard", "--json"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"repo_name\": \"legacy-project\""));
    assert!(stdout.contains("Legacy request"));
    assert!(
        workspace.join(".sandrone/config.toml").is_file(),
        "dashboard refresh should migrate legacy workspace state"
    );
    assert!(
        home.join(".sandrone/workspaces.json").is_file(),
        "dashboard refresh should save the imported registry in the new location"
    );
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
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    assert_success(&run_with_env(
        &workspace_two,
        &["new", "--name", "dashboard-two"],
        &[("SANDRONE_HOME", registry_home_str)],
    ));

    fs::write(
        workspace_one.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tDashboard request\\tDashboard body\\thttps://example.test/dashboard\\n'\n",
    )
    .expect("issue connector should be writable");
    assert_success(&run_with_env(
        &workspace_one,
        &["update"],
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    let change_name = format!("{}-dashboard-request", current_date());
    assert_success(&run_with_env(
        &workspace_one,
        &[
            "decompose",
            "--name",
            &change_name,
            "--request_id",
            "REQ-0001",
        ],
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    install_passing_decomposition_review_tool(&workspace_one);
    assert_success(&run_with_env(
        &workspace_one,
        &[
            "submit",
            "--request_id",
            "REQ-0001",
            "--gate",
            "decomposition",
        ],
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    assert_success(&run_with_env(
        &workspace_one,
        &["decomposition-review", "--request_id", "REQ-0001"],
        &[("SANDRONE_HOME", registry_home_str)],
    ));
    advance_until_not_review_running(&workspace_one, "REQ-0001");

    let change_path = workspace_one.join("obsidian/changes").join(&change_name);
    let slice_path = change_path.join("slices/S01");
    let plan_review_details = slice_path.join("reviews/plan-review/details");
    let code_review_details = slice_path.join("reviews/code-review/details");
    fs::create_dir_all(&plan_review_details).expect("plan review details dir writable");
    fs::create_dir_all(&code_review_details).expect("code review details dir writable");
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
    let worker_dir =
        workspace_one.join(".sandrone/state/reviews/REQ-0001-S01/code-review/002/test-reviewer");
    fs::create_dir_all(&worker_dir).expect("review worker state dir writable");
    fs::write(worker_dir.join("pid"), format!("{}\n", std::process::id()))
        .expect("review worker pid writable");
    fs::write(
        worker_dir.join("stdout.log"),
        "TestReviewer is reading targeted tests\n",
    )
    .expect("review worker stdout writable");
    fs::write(
        worker_dir.join("stderr.log"),
        "Codex reviewer progress is visible\n",
    )
    .expect("review worker stderr writable");
    fs::write(worker_dir.join("hook.log"), "waiting for detail JSON\n")
        .expect("review worker hook writable");
    fs::write(
        request_artifact_path(&slice_path, "REQ-0001-S01", "change-doc.md"),
        "# Change\n\n## 摘要\n\nSlice implementation。\n",
    )
    .expect("slice change doc writable");
    force_request_state(
        &workspace_one,
        "REQ-0001-S01",
        "change-doc-submitted",
        "codex/req-0001-s01",
        "dev/worktrees/REQ-0001-S01",
    );

    let dashboard_root = temp_workspace("dashboard-root");
    let output = run_with_env(
        &dashboard_root,
        &["dashboard", "--json"],
        &[("SANDRONE_HOME", registry_home_str)],
    );
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"projects\""));
    assert!(stdout.contains("\"repo_name\": \"dashboard-one\""));
    assert!(stdout.contains("\"repo_name\": \"dashboard-two\""));
    assert!(stdout.contains("\"request_count\": 1"));
    assert!(stdout.contains("\"decomposition\""));
    assert!(stdout.contains("\"label\": \"需求分析\""));
    assert!(!stdout.contains("\"analysis\""));
    assert!(stdout.contains("\"slices\""));
    assert!(stdout.contains("REQ-0001-S01"));
    assert!(
        !stdout.contains("\"requests\": [\n        {\n          \"request_id\": \"REQ-0001-S01\""),
        "dashboard top-level request list must contain only parent requests"
    );
    assert!(stdout.contains("\"stage_id\": \"request\""));
    assert!(stdout.contains("\"stage_id\": \"decomposition\""));
    assert!(stdout.contains("\"stage_id\": \"decomposition-review\""));
    assert!(stdout.contains("\"stage_id\": \"plan\""));
    assert!(stdout.contains("\"stage_id\": \"plan-review\""));
    assert!(stdout.contains("\"stage_id\": \"implementation\""));
    assert!(stdout.contains("\"stage_id\": \"code-review\""));
    assert!(stdout.contains("\"pr\": { \"label\": \"PR\""));
    assert!(stdout.contains("\"stage_id\": \"finish-pr\""));
    assert!(stdout.contains("\"artifact_path\": \"obsidian/changes"));
    assert!(stdout.contains("\"artifact_name\": \"REQ-0001 request.md\""));
    assert!(stdout.contains("\"artifact_name\": \"REQ-0001-S01 plan.md\""));
    assert!(stdout.contains("\"artifact_name\": \"REQ-0001-S01 change-doc.md\""));
    let slice_section_tail = stdout
        .split("\"request_id\": \"REQ-0001-S01\"")
        .nth(1)
        .expect("slice dashboard object should exist");
    let slice_section_end = slice_section_tail
        .find("\n          ],\n          \"pr\":")
        .expect("slice section should end before parent PR tab");
    let slice_section = &slice_section_tail[..slice_section_end];
    let first_slice_stage = slice_section
        .find("\"stage_id\"")
        .map(|index| &slice_section[index..])
        .expect("slice should have stages");
    assert!(
        first_slice_stage.starts_with("\"stage_id\": \"plan\""),
        "slice timeline should start at plan instead of request"
    );
    assert!(
        !slice_section.contains("\"stage_id\": \"request\""),
        "slice timeline should not render a request stage"
    );
    assert!(
        !slice_section.contains("\"stage_id\": \"finish-pr\""),
        "slice timeline should not render PR delivery"
    );
    assert!(
        !slice_section.contains("\"stage_id\": \"pr-refresh\""),
        "slice timeline should not render PR refresh"
    );
    assert!(
        !slice_section.contains("\"stage_id\": \"integration-review\""),
        "slice timeline should not render integration review"
    );
    let pr_section = stdout
        .split("\"pr\": { \"label\": \"PR\"")
        .nth(1)
        .expect("parent PR tab should exist");
    assert!(pr_section.contains("\"stage_id\": \"finish-pr\""));
    assert!(
        !pr_section.contains("\"stage_id\": \"pr-refresh\""),
        "PR refresh should stay hidden until there is a refresh or integration record"
    );
    assert!(stdout.contains("plan.md"));
    assert!(stdout.contains("change-doc.md"));
    assert!(stdout.contains("\"review_attempts\""));
    assert!(stdout.contains("\"attempt\": 1"));
    assert!(stdout.contains("\"attempt\": 2"));
    assert!(stdout.contains("\"runtime_status\": \"running\""));
    assert!(stdout.contains("TestReviewer is reading targeted tests"));
    assert!(stdout.contains("Codex reviewer progress is visible"));
    assert!(stdout.contains("waiting for detail JSON"));
    assert!(stdout.contains("001-plan-reviewer.json"));
    assert!(stdout.contains("002-plan-reviewer.json"));
    assert!(stdout.contains("TestReviewer"));
    assert!(stdout.contains("DesignReviewer"));
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
    assert!(stdout.contains("CodeGraph context"));
    assert!(stdout.contains("obsidian/codegraph/context.md"));

    assert!(workspace.join("dev/repo/README.md").is_file());
    assert!(workspace.join("dev/repo/.codegraph").is_dir());
    let codegraph_log_content =
        fs::read_to_string(codegraph_log).expect("fake codegraph log readable");
    assert!(codegraph_log_content.contains("init -i dev/repo"));
    assert!(codegraph_log_content.contains("context -p dev/repo"));
    assert!(git_success(
        &workspace.join("dev/repo"),
        &["rev-parse", "--verify", "HEAD"]
    ));
    assert!(workspace.join("tools/issue-update.sh").is_file());
    assert!(workspace.join("tools/pr-create.sh").is_file());
    assert!(workspace.join("skills/sandrone/SKILL.md").is_file());
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

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert_eq!(state.matches("REQ-0001").count(), 1);
    assert!(!state.contains("REQ-0002"));

    let change_name = format!("{}-issue-body", current_date());
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    let change_path = workspace.join("obsidian/changes").join(change_name);
    let request = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "request.md",
    ))
    .expect("request should be readable");
    let plan = fs::read_to_string(request_artifact_path(&change_path, "REQ-0001", "plan.md"))
        .expect("plan should be readable");
    assert!(request.contains("## 需求标题"));
    assert!(request.contains("First request"));
    assert!(request.contains("## 需求描述"));
    assert!(request.contains("Detailed body line one\nline two"));
    assert!(plan.contains("本计划模板不内嵌需求正文或摘要"));
    assert!(plan.contains("- 需求记录:"));
    assert!(!plan.contains("Detailed body line one\nline two"));
    assert!(!plan.contains("Detailed body line one line two"));
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
        !workspace
            .join("obsidian/changes")
            .join(change_name)
            .exists(),
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

    let change_path = workspace.join("obsidian/changes").join(&change_name);
    for artifact in [
        "REQ-0001 index.md",
        "REQ-0001 request.md",
        "REQ-0001 plan.md",
        "REQ-0001 change-doc.md",
        "REQ-0001 pr-doc.md",
        "REQ-0001 agent-journal.md",
        "status.json",
    ] {
        assert!(
            change_path.join(artifact).is_file(),
            "missing artifact: {artifact}"
        );
    }
    for legacy_artifact in [
        "request.md",
        "plan.md",
        "change-doc.md",
        "pr-doc.md",
        "agent-journal.md",
    ] {
        assert!(
            !change_path.join(legacy_artifact).exists(),
            "new change packet should not create legacy artifact: {legacy_artifact}"
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

    let plan = fs::read_to_string(request_artifact_path(&change_path, "REQ-0001", "plan.md"))
        .expect("plan should be readable");
    let change_doc = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change doc should be readable");
    let journal = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "agent-journal.md",
    ))
    .expect("agent journal should be readable");
    let status =
        fs::read_to_string(change_path.join("status.json")).expect("status should be readable");
    let project = fs::read_to_string(workspace.join("obsidian/project.md"))
        .expect("project note should be readable");

    assert!(plan.contains("这是空白计划模板"));
    assert!(plan.contains("## 规范化需求记录"));
    assert!(plan.contains("- Request ID: `REQ-0001`"));
    assert!(plan.contains("- External ID: `manual:REQ-0001`"));
    assert!(plan.contains("- Source: `manual`"));
    assert!(plan.contains("### 需求名称"));
    assert!(plan.contains("### 需求描述索引"));
    assert!(plan.contains("本计划模板不内嵌需求正文或摘要"));
    assert!(plan.contains("- 父需求记录: not-applicable"));
    assert!(plan.contains("- Slice 元数据: not-applicable"));
    assert!(plan.contains("如果这是 slice，本文件同时是 slice request 与 plan"));
    assert!(plan.contains("目标项目内部要求"));
    assert!(!plan.contains("执行任务清单"));
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
    assert!(change_doc.contains("Agent journal: [[REQ-0001 agent-journal|agent-journal.md]]"));
    assert!(journal.contains("agent 每轮"));
    assert!(status.contains("\"stage\": \"planning\""));
    assert!(status.contains("\"obsidian_note\": \"obsidian/changes/"));
    assert!(status.contains("/REQ-0001 index.md\""));
    assert!(status.contains("\"obsidian_project\": \"obsidian/project.md\""));
    assert!(project.contains("## 需求索引"));
    assert!(project.contains(&format!("### {}", current_date())));
    assert!(project.contains(&format!(
        "[[changes/{change_name}/REQ-0001 index|REQ-0001 first feature]]"
    )));
    assert!(project.contains("`obsidian/relations.md`"));
    assert!(!project.contains("[[relations|relations.md]]"));
    assert!(workspace.join("obsidian/relations.md").is_file());
    assert!(workspace.join("obsidian/views/requests.base").is_file());
    assert!(workspace.join("obsidian/views/slices.base").is_file());
    assert!(workspace.join("obsidian/derived/requests.json").is_file());
    assert!(workspace.join("obsidian/derived/slices.json").is_file());
    let derived_requests = fs::read_to_string(workspace.join("obsidian/derived/requests.json"))
        .expect("derived requests json should be readable");
    let derived_slices = fs::read_to_string(workspace.join("obsidian/derived/slices.json"))
        .expect("derived slices json should be readable");
    assert!(derived_requests.contains("\"schema_version\": 1"));
    assert!(derived_requests.contains("\"request_id\":\"REQ-0001\""));
    assert!(derived_requests.contains("\"note\":\"changes/"));
    assert!(derived_slices.contains("\"slices\""));
    let canvas = fs::read_to_string(workspace.join("obsidian/project.canvas"))
        .expect("project canvas should be readable");
    assert!(canvas.contains("\"nodes\""));
    assert!(canvas.contains("\"edges\""));
    assert!(canvas.contains("project.md"));
    fs::write(
        workspace.join("obsidian/relations.md"),
        "1. 先读 [[project|project.md]] 和本文件。\n\n## 关系记录\n\n保留人工内容。\n",
    )
    .expect("legacy relations note should be writable");
    let legacy_change_doc_path = request_artifact_path(&change_path, "REQ-0001", "change-doc.md");
    fs::write(
        &legacy_change_doc_path,
        "# Legacy Change\n\n- Project root: [[project|project.md]]\n- Relations: [[relations|relations.md]]\n\n保留变更正文。\n",
    )
    .expect("legacy change doc should be writable");
    let legacy_pr_doc_path = request_artifact_path(&change_path, "REQ-0001", "pr-doc.md");
    fs::write(
        &legacy_pr_doc_path,
        "# Legacy PR\n\n- Project root: [[project|project.md]]\n- Project Relations: [[relations|relations.md]]\n\n保留 PR 正文。\n",
    )
    .expect("legacy pr doc should be writable");
    fs::remove_file(workspace.join("obsidian/project.canvas"))
        .expect("project canvas should be removable for refresh test");
    let refreshed = run(&workspace, &["obsidian-refresh"]);
    assert_success(&refreshed);
    let refresh_stdout = String::from_utf8_lossy(&refreshed.stdout);
    assert!(refresh_stdout.contains("Obsidian artifacts refreshed"));
    assert!(refresh_stdout.contains("obsidian/derived/requests.json"));
    assert!(workspace.join("obsidian/project.canvas").is_file());
    let refreshed_relations = fs::read_to_string(workspace.join("obsidian/relations.md"))
        .expect("relations note should be readable after refresh");
    assert!(refreshed_relations.contains("1. 先读 `obsidian/project.md` 和本文件。"));
    assert!(refreshed_relations.contains("保留人工内容"));
    assert!(!refreshed_relations.contains("[[project|project.md]]"));
    let migrated_change_doc =
        fs::read_to_string(legacy_change_doc_path).expect("change doc should be readable");
    assert!(migrated_change_doc.contains("保留变更正文"));
    assert!(migrated_change_doc.contains("上级索引"));
    assert!(migrated_change_doc.contains("- Relations: `obsidian/relations.md`"));
    assert!(!migrated_change_doc.contains("[[project|project.md]]"));
    let migrated_pr_doc =
        fs::read_to_string(legacy_pr_doc_path).expect("pr doc should be readable");
    assert!(migrated_pr_doc.contains("保留 PR 正文"));
    assert!(migrated_pr_doc.contains("- Project Relations: `obsidian/relations.md`"));
    assert!(!migrated_pr_doc.contains("[[project|project.md]]"));
    assert!(!change_path.join("approvals").exists());
}

#[test]
fn status_sync_preserves_obsidian_index_relationships_and_summary() {
    let workspace = temp_workspace("obsidian-preserve");
    let change_name = format!("{}-preserve-summary", current_date());
    assert_success(&run(&workspace, &["new", "--name", "obsidian-preserve"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let note_path = workspace
        .join("obsidian/changes")
        .join(&change_name)
        .join("REQ-0001 index.md");
    let note = fs::read_to_string(&note_path).expect("obsidian index readable");
    let customized = note.replace(
        "请由 agent 在计划、实现、PR 刷新或恢复时维护本节的短摘要。",
        "自定义摘要: agent 已经记录了需求关系和恢复上下文。",
    );
    fs::write(&note_path, customized).expect("obsidian index writable");

    assert_success(&run(
        &workspace,
        &[
            "block",
            "--request_id",
            "REQ-0001",
            "--stage",
            "planning",
            "--reason",
            "test status refresh",
        ],
    ));

    let refreshed = fs::read_to_string(&note_path).expect("obsidian index readable after sync");
    assert!(refreshed.contains("status: blocked"));
    assert!(refreshed.contains("自定义摘要: agent 已经记录了需求关系和恢复上下文。"));
    assert!(refreshed.contains("- 上级导航: [[project|project.md]]"));
    assert!(refreshed.contains("- Agent 日志: [[REQ-0001 agent-journal|agent-journal.md]]"));
}

#[test]
fn decompose_creates_slice_artifacts_obsidian_note_and_gate() {
    let workspace = temp_workspace("decompose");
    let change_name = format!("{}-large-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "decompose-test"]));

    assert_success(&run(
        &workspace,
        &[
            "decompose",
            "--name",
            &change_name,
            "--request_id",
            "REQ-0001",
        ],
    ));

    let change_path = workspace.join("obsidian/changes").join(&change_name);
    for artifact in [
        "REQ-0001 request.md",
        "REQ-0001 decomposition.md",
        "REQ-0001 pr-doc.md",
        "REQ-0001 agent-journal.md",
        "decomposition.json",
        "dag.json",
        "status.json",
        ".decomposition-kind",
    ] {
        assert!(
            change_path.join(artifact).is_file(),
            "missing decomposition artifact: {artifact}"
        );
    }
    for artifact in ["REQ-0001 plan.md", "REQ-0001 change-doc.md"] {
        assert!(
            !change_path.join(artifact).exists(),
            "parent decomposition packet should not create {artifact}"
        );
    }
    let obsidian_note = change_path.join("REQ-0001 index.md");
    assert!(obsidian_note.is_file(), "missing Obsidian request note");
    let note = fs::read_to_string(obsidian_note).expect("obsidian note readable");
    assert!(note.contains("type: request"));
    assert!(note.contains("- 上级导航: [[project|project.md]]"));
    assert!(note.contains("[[REQ-0001 agent-journal|agent-journal.md]]"));
    let derived_slices = fs::read_to_string(workspace.join("obsidian/derived/slices.json"))
        .expect("derived slices json should be readable");
    assert!(derived_slices.contains("\"parent_request_id\":\"REQ-0001\""));
    assert!(derived_slices.contains("\"slice_request_id\":\"REQ-0001-S01\""));
    assert!(derived_slices.contains("\"slice_id\":\"S01\""));
    let project_canvas = fs::read_to_string(workspace.join("obsidian/project.canvas"))
        .expect("project canvas should be readable");
    assert!(project_canvas.contains("\"label\":\"slice\""));

    let rejected_start = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start, "decomposition gate approval required");

    install_passing_decomposition_review_tool(&workspace);

    assert_success(&run(
        &workspace,
        &[
            "submit",
            "--request_id",
            "REQ-0001",
            "--gate",
            "decomposition",
        ],
    ));
    assert_success(&run(
        &workspace,
        &["decomposition-review", "--request_id", "REQ-0001"],
    ));
    advance_until_not_review_running(&workspace, "REQ-0001");
    let decomposition_doc = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "decomposition.md",
    ))
    .expect("decomposition gate should be readable");
    assert!(decomposition_doc.contains("gate_name: decomposition"));
    assert!(decomposition_doc.contains("gate_status: approved"));
    assert!(!change_path.join("approvals").exists());
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "REQ-0001-S01",
    );
    let state =
        fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).expect("state readable");
    assert!(state.contains("REQ-0001-S01"));
    assert!(state.contains("slice:REQ-0001:S01"));
    assert!(
        change_path
            .join("slices/S01")
            .join("REQ-0001-S01 plan.md")
            .is_file()
    );
    assert!(
        change_path
            .join("slices/S01")
            .join("REQ-0001-S01 change-doc.md")
            .is_file()
    );
    assert!(
        change_path
            .join("slices/S01")
            .join("REQ-0001-S01 agent-journal.md")
            .is_file()
    );
    assert!(
        !change_path
            .join("slices/S01")
            .join("REQ-0001-S01 request.md")
            .exists()
    );
    assert!(
        !change_path
            .join("slices/S01")
            .join("REQ-0001-S01 pr-doc.md")
            .exists()
    );

    let refreshed_parent_note =
        fs::read_to_string(change_path.join("REQ-0001 index.md")).expect("parent note readable");
    assert!(refreshed_parent_note.contains("[[slices/S01/REQ-0001-S01 index|REQ-0001-S01"));
    let slice_note =
        fs::read_to_string(change_path.join("slices/S01").join("REQ-0001-S01 index.md"))
            .expect("slice note readable");
    assert!(slice_note.contains("[[REQ-0001 index|REQ-0001 index.md]]"));
    assert!(!slice_note.contains("[[project|project.md]]"));
    let project_note =
        fs::read_to_string(workspace.join("obsidian/project.md")).expect("project note readable");
    assert!(project_note.contains("[[changes/"));
    assert!(project_note.contains("REQ-0001 index"));
    assert!(
        !project_note.contains("REQ-0001-S01 index"),
        "project root must not link directly to slice index"
    );

    let slice_path = change_path.join("slices/S01");
    fs::write(change_path.join("plan.md"), "# legacy parent plan\n")
        .expect("legacy parent plan writable");
    fs::write(
        change_path.join("REQ-0001 change-doc.md"),
        "# obsolete parent change doc\n",
    )
    .expect("obsolete parent change doc writable");
    let legacy_parent_archive = change_path.join("archived-old");
    fs::create_dir_all(&legacy_parent_archive).expect("legacy parent archive dir writable");
    fs::write(
        legacy_parent_archive.join("old.md"),
        "# old parent archive\n",
    )
    .expect("legacy parent archive file writable");
    fs::write(slice_path.join("request.md"), "# legacy slice request\n")
        .expect("legacy slice request writable");
    fs::write(
        slice_path.join("REQ-0001-S01 pr-doc.md"),
        "# obsolete slice pr doc\n",
    )
    .expect("obsolete slice pr doc writable");
    let legacy_slice_archive = slice_path.join("archived-old");
    fs::create_dir_all(&legacy_slice_archive).expect("legacy slice archive dir writable");
    fs::write(legacy_slice_archive.join("old.md"), "# old slice archive\n")
        .expect("legacy slice archive file writable");

    assert_success(&run(&workspace, &["upgrade"]));

    for artifact in [
        "plan.md",
        "REQ-0001 plan.md",
        "change-doc.md",
        "REQ-0001 change-doc.md",
    ] {
        assert!(
            !change_path.join(artifact).exists(),
            "upgrade should remove obsolete parent artifact: {artifact}"
        );
    }
    assert_no_archived_entries(&change_path);
    for artifact in [
        "request.md",
        "REQ-0001-S01 request.md",
        "pr-doc.md",
        "REQ-0001-S01 pr-doc.md",
    ] {
        assert!(
            !slice_path.join(artifact).exists(),
            "upgrade should remove obsolete slice artifact: {artifact}"
        );
    }
    assert_no_archived_entries(&slice_path);
}

#[test]
fn slice_plan_template_references_parent_request_without_inlining_long_body() {
    let workspace = temp_workspace("slice-plan-short");
    let change_name = format!("{}-slice-plan-short", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "slice-plan-short-test"],
    ));
    let long_body = format!(
        "LONG_PARENT_BODY_MARKER {}",
        "large parent requirement body ".repeat(300)
    );
    fs::write(
        workspace.join("tools/issue-update.sh"),
        format!(
            "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tLarge parent request\\t{}\\thttps://example.test/1\\n'\n",
            long_body
        ),
    )
    .expect("issue connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    assert_success(&run(
        &workspace,
        &[
            "decompose",
            "--name",
            &change_name,
            "--request_id",
            "REQ-0001",
        ],
    ));
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    fs::write(
        change_path.join("decomposition.json"),
        r#"{"schema_version":1,"request_id":"REQ-0001","kind":"request_decomposition","classification":{"slice_strategy":"single-or-dag"},"slices":[{"id":"S01","name":"small-slice","status":"draft","type":"feature","summary":"Small slice summary only","depends_on":[],"parallel_group":"main","conflict_domains":[],"inputs":[],"outputs":[],"acceptance":["done"],"tests":["covered"],"docs":[],"branch":"codex/req-0001-s01-small-slice","worktree":""}],"global_invariants":[],"updated_at":"test"}"#,
    )
    .expect("decomposition json writable");
    fs::write(
        change_path.join("dag.json"),
        r#"{"schema_version":1,"request_id":"REQ-0001","nodes":[{"id":"S01","name":"small-slice","status":"draft","depends_on":[],"parallel_group":"main","conflict_domains":[],"branch":"codex/req-0001-s01-small-slice","worktree":""}],"edges":[],"parallel_policy":{"max_parallel_slices":1,"respect_conflict_domains":true},"updated_at":"test"}"#,
    )
    .expect("dag json writable");
    install_passing_decomposition_review_tool(&workspace);
    assert_success(&run(
        &workspace,
        &[
            "submit",
            "--request_id",
            "REQ-0001",
            "--gate",
            "decomposition",
        ],
    ));
    assert_success(&run(
        &workspace,
        &["decomposition-review", "--request_id", "REQ-0001"],
    ));
    advance_until_not_review_running(&workspace, "REQ-0001");

    let slice_plan_path = change_path.join("slices/S01").join("REQ-0001-S01 plan.md");
    wait_for_file(&slice_plan_path);
    let slice_plan = fs::read_to_string(slice_plan_path).expect("slice plan should be readable");
    assert!(slice_plan.contains("- 父需求记录: obsidian/changes/"));
    assert!(slice_plan.contains("- Slice 元数据: obsidian/changes/"));
    assert!(slice_plan.contains("- DAG: obsidian/changes/"));
    assert!(!slice_plan.contains("Small slice summary only"));
    assert!(
        !slice_plan.contains("LONG_PARENT_BODY_MARKER"),
        "slice plan template should not inline the full parent issue body"
    );
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
    assert_failure_contains(&rejected_start, "plan gate approval required");

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

    let change_path = workspace.join("obsidian/changes").join(change_name);
    let status =
        fs::read_to_string(change_path.join("status.json")).expect("status should be readable");
    assert!(status.contains("\"stage\": \"implementation\""));
    let change_doc = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change doc should be readable");
    assert!(change_doc.contains("实现前后对比"));
    assert!(change_doc.contains("关键设计点"));
    assert!(change_doc.contains("不需要完整文件清单"));
    assert!(change_doc.contains("Pre-commit"));
    assert!(change_doc.contains("AI review"));

    let sessions = fs::read_to_string(workspace.join(".sandrone/sessions.json"))
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
    write_executable(
        &workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nexit 0\n",
    );
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
    write_executable(
        &workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nexit 0\n",
    );
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
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("requests state readable");
    assert!(state.contains("\tblocked\t"));
    let status = fs::read_to_string(
        workspace
            .join("obsidian/changes")
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
        "#!/usr/bin/env sh\nset -eu\ncp \"$SANDRONE_PR_BODY_FILE\" .sandrone/state/captured-pr-body.md\nprintf 'https://example.test/pr/1\\n'\n",
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
    assert_failure_contains(&rejected_finish, "change-doc gate approval required");

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
        .join("obsidian/changes")
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
        "{\"schema_version\":1,\"request_id\":\"REQ-0001\",\"stage\":\"code-review\",\"attempt\":1,\"approved\":true,\"reviewers\":[{\"reviewer\":\"TestReviewer\",\"approved\":true,\"has_blocking_findings\":false,\"gate_unavailable\":false,\"recommended_next_phase\":\"implementation\",\"summary\":\"tests are adequate with a follow-up warning\",\"diagnostic\":\"\",\"path\":\"obsidian/changes/reviews/code-review/details/001-test-reviewer.json\"},{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"has_blocking_findings\":false,\"gate_unavailable\":false,\"recommended_next_phase\":\"implementation\",\"summary\":\"design is acceptable with review notes\",\"diagnostic\":\"\",\"path\":\"obsidian/changes/reviews/code-review/details/001-design-reviewer.json\"}],\"updated_at\":\"2026-05-31T00:00:00Z\"}\n",
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

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("wait-finish"));
    assert!(!state.contains("\tfinished\t"));
    let pushed_commit = git_output(&origin, &["rev-parse", "refs/heads/codex/req-0001"]);
    assert!(!pushed_commit.is_empty());
    let commit_message = git_output(&worktree, &["log", "-1", "--pretty=%s"]);
    assert_eq!(commit_message, "feat: deliver first feature");
    let pr_body = fs::read_to_string(workspace.join(".sandrone/state/captured-pr-body.md"))
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

    let change_doc_gate = fs::read_to_string(request_artifact_path(
        &workspace.join("obsidian/changes").join(change_name),
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change-doc gate frontmatter should be readable");
    assert!(change_doc_gate.contains("gate_name: change-doc"));
    assert!(change_doc_gate.contains("gate_status: approved"));
    assert!(change_doc_gate.contains("gate_body_sha256:"));
    assert!(change_doc_gate.contains("变更文档已检查"));

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
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
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
fn pr_merge_requires_safe_pr_status() {
    let workspace = temp_workspace("pr-merge-gate");
    let change_name = format!("{}-merge-gate-feature", current_date());
    let origin = create_bare_origin_with_master("pr-merge-gate-origin");
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
        "#!/usr/bin/env sh\nprintf 'github:owner/repo#45\\tgithub\\tMerge gate feature\\tBody\\thttps://github.com/owner/repo/issues/45\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-create.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'existing\\thttps://example.test/pr/merge-gate\\n'\n",
    )
    .expect("pr connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'safe\\thttps://example.test/pr/merge-gate\\tchecks green and branch current\\n'\n",
    )
    .expect("pr status connector should be writable");
    fs::write(
        workspace.join("tools/pr-merge.sh"),
        "#!/usr/bin/env sh\nset -eu\ntest \"$SANDRONE_PR_STATUS\" = safe\nprintf '%s\\n' \"$SANDRONE_SCHEDULER_DECISION_ID\" >> .sandrone/state/pr-merge-called.log\nprintf 'merged\\thttps://example.test/pr/merge-gate\\tmerged after safety probe\\n'\n",
    )
    .expect("pr merge connector should be writable");

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
    fs::write(worktree.join("merge-gate.txt"), "implemented\n")
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
    assert_success(&run(
        &workspace,
        &[
            "finish",
            "--request_id",
            "REQ-0001",
            "--message",
            "feat: deliver merge gate feature",
        ],
    ));

    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'open\\thttps://example.test/pr/merge-gate\\twaiting for platform checks\\n'\n",
    )
    .expect("pr status connector should be writable");
    let open_merge = run(&workspace, &["pr-merge", "--request_id", "REQ-0001"]);
    assert_success(&open_merge);
    let open_stdout = String::from_utf8_lossy(&open_merge.stdout);
    assert!(open_stdout.contains("expected safe before merge"));
    assert!(
        !workspace
            .join(".sandrone/state/pr-merge-called.log")
            .exists()
    );

    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'safe\\thttps://example.test/pr/merge-gate\\tchecks green and branch current\\n'\n",
    )
    .expect("pr status connector should be writable");
    let merged = run(&workspace, &["pr-merge", "--request_id", "REQ-0001"]);
    assert_success(&merged);
    let merged_stdout = String::from_utf8_lossy(&merged.stdout);
    assert!(merged_stdout.contains("PR merge check for REQ-0001: merged"));
    assert!(merged_stdout.contains("request status: finished"));
    assert!(
        workspace
            .join(".sandrone/state/pr-merge-called.log")
            .is_file()
    );
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("\tfinished\t"));
    let decisions = fs::read_dir(workspace.join(".sandrone/state/scheduler/decisions"))
        .expect("decision dir should be readable")
        .map(|entry| {
            fs::read_to_string(entry.expect("decision entry readable").path())
                .expect("decision file readable")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(decisions.contains("\"pr_status\": \"open\""));
    assert!(decisions.contains("\"action\": \"merged\""));
}

#[test]
fn tick_automatically_merges_wait_finish_request() {
    let workspace = temp_workspace("tick-auto-merge");
    let change_name = format!("{}-auto-merge", current_date());
    assert_success(&run(&workspace, &["new", "--name", "auto-merge"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tAuto merge config\\tBody\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'safe\\thttps://example.test/pr/config\\tready to merge\\n'\n",
    )
    .expect("pr status connector should be writable");
    fs::write(
        workspace.join("tools/pr-merge.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf '%s\\n' \"$SANDRONE_REQUEST_ID\" >> .sandrone/state/pr-merge-called.log\nprintf 'merged\\thttps://example.test/pr/config\\tmerged by config scheduler\\n'\n",
    )
    .expect("pr merge connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\n",
    )
    .expect("issue connector should be writable");
    prepare_wait_finish_request(&workspace, "REQ-0001", &change_name);

    let enabled = run(&workspace, &["tick"]);
    assert_success(&enabled);
    let enabled_stdout = String::from_utf8_lossy(&enabled.stdout);
    assert!(enabled_stdout.contains("PR merge check for REQ-0001: merged"));
    let merge_log = fs::read_to_string(workspace.join(".sandrone/state/pr-merge-called.log"))
        .expect("merge log should be readable");
    assert_eq!(merge_log.lines().count(), 1);
    assert!(merge_log.contains("REQ-0001"));
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("\tfinished\t"));
}

#[test]
fn tick_automatic_merge_processes_at_most_one_wait_finish_request_per_run() {
    let workspace = temp_workspace("tick-merge-one");
    let first_change_name = format!("{}-merge-one-a", current_date());
    let second_change_name = format!("{}-merge-one-b", current_date());
    assert_success(&run(&workspace, &["new", "--name", "merge-one"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst auto merge\\tBody\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond auto merge\\tBody\\thttps://example.test/2\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'safe\\thttps://example.test/pr/%s\\tready to merge\\n' \"$SANDRONE_REQUEST_ID\"\n",
    )
    .expect("pr status connector should be writable");
    fs::write(
        workspace.join("tools/pr-merge.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf '%s\\n' \"$SANDRONE_REQUEST_ID\" >> .sandrone/state/pr-merge-called.log\nprintf 'merged\\thttps://example.test/pr/%s\\tmerged one at a time\\n' \"$SANDRONE_REQUEST_ID\"\n",
    )
    .expect("pr merge connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\n",
    )
    .expect("issue connector should be writable");
    prepare_wait_finish_request(&workspace, "REQ-0001", &first_change_name);
    prepare_wait_finish_request(&workspace, "REQ-0002", &second_change_name);

    let output = run(&workspace, &["tick"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PR merge check for REQ-0001: merged"));
    assert!(!stdout.contains("PR merge check for REQ-0002"));
    let merge_log = fs::read_to_string(workspace.join(".sandrone/state/pr-merge-called.log"))
        .expect("merge log should be readable");
    assert_eq!(merge_log.lines().count(), 1);
    assert!(merge_log.contains("REQ-0001"));
    assert!(!merge_log.contains("REQ-0002"));
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("REQ-0001"));
    assert!(state.contains("REQ-0002"));
    assert_eq!(state.matches("\tfinished\t").count(), 1);
    assert!(state.contains("REQ-0002"));
    assert!(state.contains("wait-finish"));
}

#[test]
fn pr_refresh_unsafe_status_returns_to_implementation_review_gate() {
    let workspace = temp_workspace("pr-status-unsafe");
    let change_name = format!("{}-pr-status-unsafe-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "pr-status-unsafe"]));
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'github:owner/repo#44\\tgithub\\tPR status unsafe feature\\tBody\\thttps://github.com/owner/repo/issues/44\\n'\n",
    )
    .expect("issue connector should be writable");
    fs::write(
        workspace.join("tools/pr-status.sh"),
        "#!/usr/bin/env sh\nprintf 'unsafe\\thttps://example.test/pr/unsafe\\tbase branch advanced and checks require update\\n'\n",
    )
    .expect("pr status connector should be writable");

    assert_success(&run(&workspace, &["update"]));
    prepare_wait_finish_request(&workspace, "REQ-0001", &change_name);

    let refresh = run(&workspace, &["pr-refresh", "--request_id", "REQ-0001"]);
    assert_success(&refresh);
    let refresh_stdout = String::from_utf8_lossy(&refresh.stdout);
    assert!(refresh_stdout.contains("returned REQ-0001 to implementation"));
    assert!(refresh_stdout.contains("unsafe"));
    assert!(!refresh_stdout.contains("rebase-agent"));
    assert!(!refresh_stdout.contains("IntegrationReviewer"));

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("REQ-0001"));
    assert!(state.contains("\tcode-review-rejected\t"));
    let status = fs::read_to_string(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("status.json"),
    )
    .expect("status should be readable");
    assert!(status.contains("\"stage\": \"implementation\""));
    assert!(status.contains("\"status\": \"code-review-rejected\""));
    assert!(status.contains("PR status requires implementation refresh"));

    let pr_status = fs::read_to_string(workspace.join(".sandrone/state/REQ-0001-pr-status.tsv"))
        .expect("pr status snapshot should be readable");
    assert!(pr_status.contains("unsafe"));
    assert!(pr_status.contains("base branch advanced"));

    let change_doc = fs::read_to_string(request_artifact_path(
        &workspace.join("obsidian/changes").join(&change_name),
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change doc readable");
    assert!(change_doc.contains("gate_name: change-doc"));
    assert!(change_doc.contains("gate_status: rejected"));
    assert!(change_doc.contains("gate_by: pr-status"));
    assert!(change_doc.contains("gate_source: pr-status"));
    assert!(change_doc.contains("PR status requires implementation refresh"));

    let events = fs::read_to_string(workspace.join(".sandrone/state/events.ndjson"))
        .expect("events readable");
    assert!(events.contains("pr_status_requires_implementation"));
    assert!(
        !workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/integration-review/details/001-integration-reviewer.json")
            .exists()
    );
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
    assert_success(&rejected_plan_review);
    advance_until_not_review_running(&workspace, "REQ-0001");
    let plan_review_path = workspace
        .join("obsidian/changes")
        .join(&change_name)
        .join("reviews/plan-review/details/001-plan-reviewer.json");
    let rejected_review =
        fs::read_to_string(&plan_review_path).expect("plan review should be written");
    assert!(rejected_review.contains("\"high\""));
    let rejected_start_after_review = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start_after_review, "plan gate approval required");

    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan is implementation-grade\",\"process\":[\"read issue\",\"read plan\",\"read repo\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[{\"title\":\"minor follow-up\",\"evidence\":\"plan could add one follow-up note\",\"impact\":\"non-blocking documentation clarity improvement\",\"required_fix\":\"optional follow-up only\",\"suggested_change\":\"Optionally add a short follow-up note after implementation scope.\",\"verification\":\"No gate rerun required unless the plan text is changed materially.\"}]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${SANDRONE_AGENT_PHASE:-}" != "implementation" ]; then
  echo "unexpected phase: ${SANDRONE_AGENT_PHASE:-}" >&2
  exit 1
fi
printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
cat > "$SANDRONE_CHANGE_DOC" <<EOF
---
sandrone_schema: 1
request_id: $SANDRONE_REQUEST_ID
document_type: change-doc
agent_phase: implementation
agent_status: submitted
agent_ready_for_review: true
format_check_status: passed
format_check_exit_code: 0
updated_at: test
---

# 变更文档

## 摘要

已实现。

## 实现前后对比

已记录。

## 关键设计点

已记录。

## 验证证据

测试通过。

## Review 结果

等待汇总。
EOF
"#,
    )
    .expect("issue agent connector should be writable");
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
    assert_success(&run(
        &workspace,
        &["plan-review", "--request_id", "REQ-0001"],
    ));
    advance_until_request_state_contains(&workspace, "REQ-0001", "code-review-rejected");
    let plan_doc = fs::read_to_string(request_artifact_path(
        &workspace.join("obsidian/changes").join(&change_name),
        "REQ-0001",
        "plan.md",
    ))
    .expect("plan doc should be readable");
    assert!(plan_doc.contains("gate_name: plan"));
    assert!(plan_doc.contains("gate_status: approved"));
    assert!(plan_doc.contains("PlanReviewer"));

    let rejected_finish_after_review = run(&workspace, &["finish", "--request_id", "REQ-0001"]);
    assert_failure_contains(
        &rejected_finish_after_review,
        "change-doc gate approval required",
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
    advance_until_request_state_contains(&workspace, "REQ-0001", "wait-update-pr");
    let change_doc_gate = fs::read_to_string(request_artifact_path(
        &workspace.join("obsidian/changes").join(&change_name),
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change-doc gate frontmatter should be readable");
    assert!(change_doc_gate.contains("gate_name: change-doc"));
    assert!(change_doc_gate.contains("gate_status: approved"));
    assert!(change_doc_gate.contains("code-review"));
    assert!(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/code-review/details/001-test-reviewer.json")
            .is_file()
    );
    assert!(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/code-review/details/001-design-reviewer.json")
            .is_file()
    );
    let change_doc = fs::read_to_string(request_artifact_path(
        &workspace.join("obsidian/changes").join(&change_name),
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change-doc should be readable");
    assert!(change_doc.contains("## Review 结果"));
    assert!(change_doc.contains("PlanReviewer"));
    assert!(change_doc.contains("TestReviewer"));
    assert!(change_doc.contains("DesignReviewer"));
}

#[test]
fn retrying_implementation_resets_change_doc_ready_status_before_agent_writes() {
    let workspace = temp_workspace("retry-resets-ready");
    let change_name = format!("{}-retry-resets-ready", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "retry-resets-ready-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    write_executable(
        &workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nexit 0\n",
    );
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
    write_executable(
        &workspace.join("tools/check-format.sh"),
        "#!/usr/bin/env sh\nexit 0\n",
    );
    write_executable(
        &workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );
    write_executable(
        &workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"implementation\",\"summary\":\"needs another implementation round\",\"process\":[\"checked change doc\"],\"critical\":[],\"high\":[{\"title\":\"retry required\",\"evidence\":\"first implementation is intentionally rejected\",\"impact\":\"framework must dispatch another implementation round\",\"required_fix\":\"rerun implementation\",\"suggested_change\":\"Let the implementation agent revise change-doc and code before another code-review.\",\"verification\":\"The next implementation round starts from a draft change-doc status.\"}],\"warning\":[],\"info\":[]}'\n",
    );
    write_executable(
        &workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
if [ "${SANDRONE_AGENT_PHASE:-}" != "implementation" ]; then
  echo "unexpected phase: ${SANDRONE_AGENT_PHASE:-}" >&2
  exit 1
fi
count_file=.sandrone/state/implementation-count
count=0
if [ -f "$count_file" ]; then
  count=$(cat "$count_file")
fi
count=$((count + 1))
printf '%s\n' "$count" > "$count_file"
printf 'implementation round=%s started resume=%s\n' "$count" "${SANDRONE_AGENT_RESUME_SESSION_ID:-}" >> .sandrone/state/agent.log
if [ "$count" -eq 1 ]; then
  printf 'session id: 11111111-2222-3333-4444-555555555555\n'
fi
if [ "$count" -eq 2 ]; then
  i=0
  while [ ! -f .sandrone/state/release-second-implementation ] && [ "$i" -lt 600 ]; do
    i=$((i + 1))
    sleep 0.05
  done
fi
printf 'round %s\n' "$count" > "$SANDRONE_WORKTREE/feature.txt"
cat > "$SANDRONE_CHANGE_DOC" <<EOF
---
sandrone_schema: 1
request_id: $SANDRONE_REQUEST_ID
document_type: change-doc
agent_phase: implementation
agent_status: submitted
agent_ready_for_review: true
format_check_status: passed
format_check_exit_code: 0
updated_at: test
---

# 变更文档

## 摘要

第 $count 轮实现。

## 实现前后对比

已记录。

## 关键设计点

已记录。

## 验证证据

测试通过。

## Review 结果

等待汇总。
EOF
"#,
    );

    assert_success(&run(
        &workspace,
        &["tick", "--request_id", "REQ-0001", "--max-attempts", "20"],
    ));
    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "implementation round=1 started",
    );
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    let change_doc_path = request_artifact_path(&change_path, "REQ-0001", "change-doc.md");
    wait_for_file_contains(
        &change_path.join("reviews/code-review/details/001-design-reviewer.json"),
        "needs another implementation round",
    );
    advance_until_not_review_running(&workspace, "REQ-0001");

    assert_success(&run(
        &workspace,
        &["tick", "--request_id", "REQ-0001", "--max-attempts", "20"],
    ));
    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "implementation round=2 started",
    );
    let agent_log = fs::read_to_string(workspace.join(".sandrone/state/agent.log"))
        .expect("agent log readable");
    assert!(
        agent_log
            .contains("implementation round=2 started resume=11111111-2222-3333-4444-555555555555"),
        "retry implementation should receive the previous Codex session id for resume:\n{agent_log}"
    );
    let reset_change_doc = fs::read_to_string(&change_doc_path).expect("change-doc readable");
    assert!(
        reset_change_doc.contains("agent_status: draft"),
        "retry dispatch must clear the previous implementation submitted marker before the agent writes new output:\n{reset_change_doc}"
    );
    assert!(
        reset_change_doc.contains("agent_ready_for_review: false"),
        "retry dispatch must clear the previous implementation ready marker before the agent writes new output:\n{reset_change_doc}"
    );

    write_executable(
        &workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"retry implementation is ready\",\"process\":[\"checked retry change doc\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );
    fs::write(
        workspace.join(".sandrone/state/release-second-implementation"),
        "release",
    )
    .expect("release marker should be writable");
    advance_until_request_state_contains(&workspace, "REQ-0001", "wait-update-pr");
}

#[test]
fn review_worker_exit_without_detail_blocks_instead_of_waiting_forever() {
    let workspace = temp_workspace("review-worker-missing-detail");
    let change_name = format!("{}-missing-review-detail", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "review-worker-missing-detail-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));

    let state_path = workspace.join(".sandrone/state/requests.tsv");
    let state = fs::read_to_string(&state_path).expect("request state readable");
    assert!(state.contains("\tplan-submitted\t"));
    fs::write(
        &state_path,
        state.replace("\tplan-submitted\t", "\tplan-review-running\t"),
    )
    .expect("request state writable");

    let job_dir = workspace.join(".sandrone/state/reviews/REQ-0001/plan-review/001/plan-reviewer");
    fs::create_dir_all(&job_dir).expect("review worker state dir writable");
    fs::write(job_dir.join("exit"), "1\n").expect("review worker exit writable");
    fs::write(job_dir.join("stdout.log"), "").expect("review worker stdout writable");
    fs::write(job_dir.join("stderr.log"), "worker crashed before detail\n")
        .expect("review worker stderr writable");

    let advance = run(&workspace, &["advance", "--request_id", "REQ-0001"]);
    assert_success(&advance);

    let final_state = fs::read_to_string(&state_path).expect("request state readable");
    assert!(final_state.contains("\tblocked\t"));
    let detail = fs::read_to_string(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/plan-review/details/001-plan-reviewer.json"),
    )
    .expect("fallback detail should be written");
    assert!(detail.contains("review worker failed"));
    assert!(detail.contains("exited with code 1 before writing detail JSON"));
    assert!(detail.contains("\"gate_unavailable\": true"));
}

#[test]
fn review_worker_streams_reviewer_tool_output_to_runtime_logs() {
    let workspace = temp_workspace("review-worker-streams-output");
    let change_name = format!("{}-review-worker-streams-output", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "review-worker-streams-output-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "plan"],
    ));
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf 'live reviewer stderr before JSON\\n' >&2\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");

    let review = run(&workspace, &["plan-review", "--request_id", "REQ-0001"]);
    assert_success(&review);
    let stderr_log =
        workspace.join(".sandrone/state/reviews/REQ-0001/plan-review/001/plan-reviewer/stderr.log");
    wait_for_file_contains(&stderr_log, "live reviewer stderr before JSON");
    wait_for_file_contains(
        &workspace.join(".sandrone/state/jobs/REQ-0001/plan-review/001/plan-reviewer/stderr.log"),
        "live reviewer stderr before JSON",
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/jobs/REQ-0001/plan-review/001/plan-reviewer/events.log"),
        "review-tool-started",
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/jobs/REQ-0001/plan-review/001/plan-reviewer/events.log"),
        "review-tool-exited",
    );
    let plan_reviewer_runs = workspace.join("agents/plan-reviewer/runs");
    let run_dir = fs::read_dir(&plan_reviewer_runs)
        .expect("plan reviewer runs dir should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.join("logs/stderr.log").is_file())
        .expect("plan reviewer run should include logs");
    wait_for_file_contains(
        &run_dir.join("logs/stderr.log"),
        "live reviewer stderr before JSON",
    );
    assert!(
        run_dir
            .join("artifacts/review-context/artifact-index.md")
            .is_file()
    );
    assert!(run_dir.join("artifacts/runtime.json").is_file());
    advance_until_not_review_running(&workspace, "REQ-0001");
    let detail = fs::read_to_string(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/plan-review/details/001-plan-reviewer.json"),
    )
    .expect("plan review detail should be written");
    assert!(detail.contains("\"approved\":true") || detail.contains("\"approved\": true"));
}

#[test]
fn code_review_runs_check_format_before_reviewers_and_returns_to_implementation_on_failure() {
    let workspace = temp_workspace("format-gate");
    let change_name = format!("{}-format-gate", current_date());
    assert_success(&run(&workspace, &["new", "--name", "format-gate-test"]));
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

    write_executable(
        &workspace.join("tools/check-format.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'check-format %s worktree=%s\n' "${1:-}" "${SANDRONE_WORKTREE:-}" >> .sandrone/state/check-format.log
case "${1:-}" in
  --check)
    echo 'cargo fmt --check failed: expected formatted Rust code' >&2
    exit 42
    ;;
  *)
    echo "unexpected mode: ${1:-}" >&2
    exit 2
    ;;
esac
"#,
    );
    write_executable(
        &workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf 'test-review should not run\\n' >> .sandrone/state/reviewer.log\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );
    write_executable(
        &workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf 'design-review should not run\\n' >> .sandrone/state/reviewer.log\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    );

    let review = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_failure_contains(&review, "format check failed before code-review");
    assert!(
        !workspace.join(".sandrone/state/reviewer.log").exists(),
        "TestReviewer and DesignReviewer must not run when format gate fails"
    );
    let check_log = fs::read_to_string(workspace.join(".sandrone/state/check-format.log"))
        .expect("check-format should have been invoked");
    assert!(check_log.contains("check-format --check"));

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("code-review-rejected"));
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    assert!(
        !change_path.join("checks/format-check.md").exists(),
        "format check should not create a separate markdown record"
    );
    let change_doc = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "change-doc.md",
    ))
    .expect("change doc readable");
    assert!(change_doc.contains("format_check_status: failed"));
    assert!(change_doc.contains("format_check_exit_code: 42"));
    assert!(!change_doc.contains("format_check_record"));
    let status = fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(status.contains("format check failed before code-review"));
    assert!(status.contains("cargo fmt --check failed"));
    assert!(status.contains("exit_code=42"));
    assert!(!status.contains("\"format_check\""));
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

    let change_path = workspace.join("obsidian/changes").join(&change_name);
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
if [ "${SANDRONE_CHANGE_PATH:-}" != "${SANDRONE_REVIEW_CONTEXT:-}" ]; then
  echo "review context was not isolated" >&2
  exit 2
fi
if [ -e "$SANDRONE_CHANGE_PATH/reviews" ]; then
  echo "historical review output leaked into TestReviewer context" >&2
  exit 3
fi
if [ ! -f "$SANDRONE_REVIEW_CONTEXT/artifact-index.md" ]; then
  echo "artifact index was not generated" >&2
  exit 5
fi
if [ -e "$SANDRONE_REVIEW_CONTEXT/plan.md" ] || [ -e "$SANDRONE_REVIEW_CONTEXT/change-doc.md" ]; then
  echo "large artifacts were copied into lightweight review context" >&2
  exit 6
fi
case "$(cat "$SANDRONE_REVIEW_CONTEXT/artifact-index.md")" in
  *"Plan:"*"Change doc:"*"Recommended"*|*"Plan:"*"Change doc:"*"推荐读取顺序"*) ;;
  *) echo "artifact index does not include key paths and reading order" >&2; exit 7 ;;
esac
case "${SANDRONE_REVIEW_FORBIDDEN_PATHS:-}" in
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
if [ "${SANDRONE_CHANGE_PATH:-}" != "${SANDRONE_REVIEW_CONTEXT:-}" ]; then
  echo "review context was not isolated" >&2
  exit 2
fi
if [ -e "$SANDRONE_CHANGE_PATH/reviews" ]; then
  echo "TestReviewer or historical output leaked into DesignReviewer context" >&2
  exit 3
fi
if [ ! -f "$SANDRONE_REVIEW_CONTEXT/artifact-index.md" ]; then
  echo "artifact index was not generated" >&2
  exit 5
fi
if [ -e "$SANDRONE_REVIEW_CONTEXT/plan.md" ] || [ -e "$SANDRONE_REVIEW_CONTEXT/change-doc.md" ]; then
  echo "large artifacts were copied into lightweight review context" >&2
  exit 6
fi
case "${SANDRONE_REVIEW_FORBIDDEN_PATHS:-}" in
  *reviews/code-review*) ;;
  *) echo "forbidden review paths were not declared" >&2; exit 4 ;;
esac
printf '{"reviewer":"DesignReviewer","approved":true,"gate_unavailable":false,"decision":"approved","recommended_next_phase":"implementation","summary":"isolated design review ok","process":["checked isolated review context"],"critical":[],"high":[],"warning":[],"info":[]}\n'
"#,
    )
    .expect("design review connector should be writable");

    let review = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_success(&review);
    advance_until_not_review_running(&workspace, "REQ-0001");
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
        "#!/usr/bin/env sh\nprintf 'plan-review\\n' >> .sandrone/state/review.log\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf 'test-review\\n' >> .sandrone/state/review.log\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf 'design-review\\n' >> .sandrone/state/review.log\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nprintf 'implementation dispatched\\n' >> .sandrone/state/agent-dispatch.log\n",
    )
    .expect("issue agent should be writable");

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
    assert_success(&run(
        &workspace,
        &["submit", "--request_id", "REQ-0001", "--gate", "change-doc"],
    ));
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    let status = fs::read_to_string(change_path.join("status.json"))
        .expect("status json should be readable");
    assert!(status.contains("\"status\": \"change-doc-submitted\""));
    let plan_doc = fs::read_to_string(request_artifact_path(&change_path, "REQ-0001", "plan.md"))
        .expect("plan gate should be readable");
    assert!(plan_doc.contains("gate_name: plan"));
    assert!(plan_doc.contains("gate_status: approved"));

    let review_log_before =
        fs::read_to_string(workspace.join(".sandrone/state/review.log")).unwrap_or_default();
    assert_eq!(review_log_before.matches("plan-review").count(), 0);
    let dispatch_log_before =
        fs::read_to_string(workspace.join(".sandrone/state/agent-dispatch.log"))
            .unwrap_or_default();
    let dispatch_count_before = dispatch_log_before
        .matches("implementation dispatched")
        .count();
    force_request_state(&workspace, "REQ-0001", "plan-submitted", "", "");

    let advance = run(&workspace, &["advance", "--request_id", "REQ-0001"]);
    assert_success(&advance);
    advance_until_not_review_running(&workspace, "REQ-0001");

    let review_log = fs::read_to_string(workspace.join(".sandrone/state/review.log"))
        .expect("review log readable");
    assert_eq!(
        review_log.matches("plan-review").count(),
        0,
        "advance must not rerun plan-review when status.json is already change-doc-submitted"
    );
    assert!(review_log.contains("test-review"));
    assert!(review_log.contains("design-review"));
    let dispatch_log_after =
        fs::read_to_string(workspace.join(".sandrone/state/agent-dispatch.log"))
            .unwrap_or_default();
    assert_eq!(
        dispatch_log_after
            .matches("implementation dispatched")
            .count(),
        dispatch_count_before,
        "advance must not dispatch a duplicate implementation agent"
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "wait-update-pr",
    );
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
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

    let change_path = workspace.join("obsidian/changes").join(&change_name);
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

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
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
    assert_success(&failed_review);
    advance_until_not_review_running(&workspace, "REQ-0001");

    let change_path = workspace.join("obsidian/changes").join(&change_name);
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

    let requests = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
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
    assert_success(&failed_review);
    advance_until_not_review_running(&workspace, "REQ-0001");

    let change_path = workspace.join("obsidian/changes").join(&change_name);
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
    assert_success(&failed_review);
    advance_until_not_review_running(&workspace, "REQ-0001");

    let change_path = workspace.join("obsidian/changes").join(&change_name);
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
        .join("obsidian/changes")
        .join(change_name)
        .join("REQ-0001 plan.md");
    let approved_plan = fs::read_to_string(&plan_path).expect("plan should be readable");
    fs::write(&plan_path, format!("{approved_plan}\n\n审批后被修改。\n"))
        .expect("plan should be writable");

    let rejected_start = run(&workspace, &["start", "--request_id", "REQ-0001"]);
    assert_failure_contains(&rejected_start, "gate approval is stale");
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
        workspace.join(".sandrone/config.toml"),
        "repo_name = \"upgrade-test\"\ngit_url = \"local:upgrade-test\"\nbase_branch = \"main\"\n",
    )
    .expect("old config should be writable");
    fs::remove_file(workspace.join(".sandrone/sessions.json"))
        .expect("old workspace should not have sessions file");
    fs::remove_file(workspace.join("tools/pr-create.sh"))
        .expect("old workspace should not have pr connector");
    fs::remove_file(workspace.join("tools/pr-merge.sh"))
        .expect("old workspace should not have pr merge connector");
    fs::remove_file(workspace.join("tools/plan-review.sh"))
        .expect("old workspace should not have plan review connector");
    fs::remove_file(workspace.join("tools/test-review.sh"))
        .expect("old workspace should not have test review connector");
    fs::remove_file(workspace.join("tools/design-review.sh"))
        .expect("old workspace should not have design review connector");
    fs::remove_file(workspace.join("tools/issue-agent.sh"))
        .expect("old workspace should not have issue agent connector");
    let legacy_approvals_dir = workspace
        .join("obsidian/changes")
        .join(&change_name)
        .join("approvals");
    fs::create_dir_all(&legacy_approvals_dir).expect("legacy approvals dir should be writable");
    fs::write(
        legacy_approvals_dir.join("decomposition.approval.json"),
        "{\"status\":\"approved\",\"artifact\":\"legacy-decomposition.md\",\"artifact_sha256\":\"legacy-hash\",\"updated_at\":\"2026-05-31T00:00:00Z\"}\n",
    )
    .expect("legacy approval record should be writable");
    let success_marker = workspace.join(".sandrone/state/agents/REQ-0001.success");
    fs::create_dir_all(
        success_marker
            .parent()
            .expect("success marker should have parent"),
    )
    .expect("agents dir should be writable");
    fs::write(
        &success_marker,
        "status=success\nrequest_id=REQ-0001\nphase=planning\n",
    )
    .expect("legacy success marker should be writable");
    let decomposition_path = workspace
        .join("obsidian/changes")
        .join(&change_name)
        .join("REQ-0001 decomposition.md");
    fs::write(
        &decomposition_path,
        "# Legacy Decomposition\n\nA decomposition document without Sandrone frontmatter.\n",
    )
    .expect("legacy decomposition should be writable");
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'custom connector\\n'\n",
    )
    .expect("custom issue tool should be writable");
    let journal_path = workspace
        .join("obsidian/changes")
        .join(&change_name)
        .join("REQ-0001 agent-journal.md");
    fs::write(
        &journal_path,
        "# Agent Journal: REQ-0001\n\n这个文件用于避免上下文过长后无法恢复。agent 每轮都必须追加记录。\n\n## Attempt 1 - planning\n\n- Read: 原始需求和目标项目文档。\n- Changed: 填写 plan.md。\n",
    )
    .expect("agent journal should be writable");

    let dry_run = run(&workspace, &["upgrade", "--dry-run"]);
    assert_success(&dry_run);
    let dry_run_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_run_stdout.contains("Would create .sandrone/sessions.json"));
    assert!(dry_run_stdout.contains("Would refresh tools/issue-update.example.sh"));
    assert!(dry_run_stdout.contains("Would refresh tools/pr-merge.example.sh"));
    assert!(dry_run_stdout.contains("Would refresh tools/plan-review.example.sh"));
    assert!(dry_run_stdout.contains("Would refresh tools/prompts/plan-reviewer.example.md"));
    assert!(
        dry_run_stdout.contains("Would refresh tools/schemas/review-result.example.schema.json")
    );
    assert!(dry_run_stdout.contains("Would remove"));
    assert!(dry_run_stdout.contains("Would remove obsolete agent success marker"));
    assert!(dry_run_stdout.contains("Would normalize document status"));
    assert!(dry_run_stdout.contains("不会替换正式 connector、prompt 或 review schema"));
    assert!(dry_run_stdout.contains("sandrone upgrade --default"));

    assert_success(&run(&workspace, &["upgrade"]));

    let config = fs::read_to_string(workspace.join(".sandrone/config.toml"))
        .expect("config should be readable");
    assert!(config.contains("schema_version = 4"));
    assert!(config.contains("parallel_limit = 1"));
    assert!(workspace.join(".sandrone/sessions.json").is_file());
    assert!(
        !workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("approvals")
            .exists()
    );
    assert!(
        !success_marker.exists(),
        "upgrade should remove obsolete agent success marker files"
    );
    let normalized_decomposition = fs::read_to_string(&decomposition_path)
        .expect("normalized decomposition should be readable");
    assert!(normalized_decomposition.contains("sandrone_schema: 1"));
    assert!(normalized_decomposition.contains("request_id: REQ-0001"));
    assert!(normalized_decomposition.contains("agent_phase: decomposition"));
    assert!(normalized_decomposition.contains("agent_status: submitted"));
    assert!(normalized_decomposition.contains("agent_ready_for_review: true"));
    assert!(normalized_decomposition.contains("gate_name: decomposition"));
    assert!(normalized_decomposition.contains("gate_status: approved"));
    assert!(normalized_decomposition.contains("gate_source: legacy-gate-migration"));
    assert_no_archived_entries(&workspace.join("obsidian/changes").join(&change_name));
    let issue_tool =
        fs::read_to_string(workspace.join("tools/issue-update.sh")).expect("tool readable");
    assert!(issue_tool.contains("custom connector"));
    let issue_tool_example = fs::read_to_string(workspace.join("tools/issue-update.example.sh"))
        .expect("issue tool example readable");
    assert!(issue_tool_example.contains("Connector contract"));
    assert!(issue_tool_example.contains("gh api --method GET"));
    assert!(!workspace.join("tools/pr-create.sh").exists());
    assert!(workspace.join("tools/pr-create.example.sh").is_file());
    assert!(!workspace.join("tools/pr-merge.sh").exists());
    assert!(workspace.join("tools/pr-merge.example.sh").is_file());
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
fn upgrade_migrates_legacy_docs_changes_to_obsidian_package() {
    let workspace = temp_workspace("upgrade-legacy-docs");
    let change_name = format!("{}-legacy-feature", current_date());
    assert_success(&run(&workspace, &["new", "--name", "legacy-docs-test"]));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let new_path = workspace.join("obsidian/changes").join(&change_name);
    let legacy_path = workspace.join("docs/changes").join(&change_name);
    fs::create_dir_all(
        legacy_path
            .parent()
            .expect("legacy path should have parent"),
    )
    .expect("legacy parent writable");
    fs::rename(&new_path, &legacy_path).expect("legacy change package move should work");

    let state_path = workspace.join(".sandrone/state/requests.tsv");
    let state = fs::read_to_string(&state_path).expect("state readable");
    fs::write(
        &state_path,
        state.replace(
            &format!("obsidian/changes/{change_name}"),
            &format!("docs/changes/{change_name}"),
        ),
    )
    .expect("legacy state writable");

    assert_success(&run(&workspace, &["upgrade"]));

    assert!(
        new_path.join("REQ-0001 request.md").is_file(),
        "upgrade should copy legacy package into obsidian/changes"
    );
    assert!(
        legacy_path.join("REQ-0001 request.md").is_file(),
        "upgrade should preserve legacy directory as a non-destructive backup"
    );
    let migrated_state = fs::read_to_string(&state_path).expect("state readable after upgrade");
    assert!(migrated_state.contains(&format!("obsidian/changes/{change_name}")));
    assert!(!migrated_state.contains(&format!("docs/changes/{change_name}")));
    let status = fs::read_to_string(new_path.join("status.json")).expect("status readable");
    assert!(status.contains(&format!(
        "obsidian/changes/{change_name}/REQ-0001 request.md"
    )));
    assert!(status.contains(&format!("obsidian/changes/{change_name}/REQ-0001 index.md")));
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
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond tick request\\tSecond body\\thttps://example.test/2\\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":false,\"gate_unavailable\":false,\"decision\":\"rejected\",\"recommended_next_phase\":\"planning\",\"summary\":\"keep planning\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[{\"title\":\"hold request\",\"evidence\":\"test keeps planning request from advancing to implementation\",\"impact\":\"without this the hook could dispatch another phase during the test\",\"required_fix\":\"leave request in planning for this test\",\"suggested_change\":\"No production change; this connector is test-only.\",\"verification\":\"The request remains plan-review-rejected after the agent exits.\"}],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" "$SANDRONE_MAX_ATTEMPTS" >> .sandrone/state/agent.log
if [ "$SANDRONE_AGENT_PHASE" = "decomposition" ]; then
  sleep 1
  exit 0
fi
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
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001 phase=decomposition max=20",
    );

    let second = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&second);
    let second_stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        second_stdout.contains("Tick parallel limit reached: 1/1 issue-agent(s) already running.")
    );
    assert!(!second_stdout.contains("Dispatched REQ-0002"));
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert_eq!(state.matches("decomposition-agent-running").count(), 1);
    wait_for_file(&workspace.join(".sandrone/state/agents/REQ-0001.exit"));
}

#[test]
fn tick_dispatches_ready_slice_when_parent_request_is_first_candidate() {
    let workspace = temp_workspace("tick-parent-slice");
    let change_name = format!("{}-parent-slice", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "tick-parent-slice-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    assert_success(&run(
        &workspace,
        &[
            "decompose",
            "--name",
            &change_name,
            "--request_id",
            "REQ-0001",
        ],
    ));
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    fs::write(
        change_path.join("decomposition.json"),
        r#"{"schema_version":1,"request_id":"REQ-0001","kind":"request_decomposition","classification":{"slice_strategy":"serial-dag"},"slices":[{"id":"S01","name":"foundation","status":"draft","type":"feature","summary":"first slice","depends_on":[],"parallel_group":"serial-01","conflict_domains":[],"inputs":[],"outputs":[],"acceptance":["done"],"tests":["covered"],"docs":[],"branch":"codex/req-0001-s01-foundation","worktree":""},{"id":"S02","name":"follow-up","status":"draft","type":"feature","summary":"second slice","depends_on":["S01"],"parallel_group":"serial-02","conflict_domains":[],"inputs":[],"outputs":[],"acceptance":["done"],"tests":["covered"],"docs":[],"branch":"codex/req-0001-s02-follow-up","worktree":""}],"global_invariants":[],"updated_at":"test"}"#,
    )
    .expect("decomposition json writable");
    fs::write(
        change_path.join("dag.json"),
        r#"{"schema_version":1,"request_id":"REQ-0001","nodes":[{"id":"S01","name":"foundation","status":"draft","depends_on":[],"parallel_group":"serial-01","conflict_domains":[],"branch":"codex/req-0001-s01-foundation","worktree":""},{"id":"S02","name":"follow-up","status":"draft","depends_on":["S01"],"parallel_group":"serial-02","conflict_domains":[],"branch":"codex/req-0001-s02-follow-up","worktree":""}],"edges":[{"from":"S01","to":"S02","reason":"test serial dependency"}],"parallel_policy":{"max_parallel_slices":1,"respect_conflict_domains":true},"updated_at":"test"}"#,
    )
    .expect("dag json writable");
    install_passing_decomposition_review_tool(&workspace);
    assert_success(&run(
        &workspace,
        &[
            "submit",
            "--request_id",
            "REQ-0001",
            "--gate",
            "decomposition",
        ],
    ));
    assert_success(&run(
        &workspace,
        &["decomposition-review", "--request_id", "REQ-0001"],
    ));
    advance_until_not_review_running(&workspace, "REQ-0001");
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "REQ-0001-S02",
    );
    force_request_state(&workspace, "REQ-0001-S01", "slice-finished", "", "");
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\n",
    )
    .expect("issue update connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" >> .sandrone/state/agent.log
sleep 1
"#,
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Dispatched REQ-0001-S02"),
        "tick should dispatch the ready child slice instead of spending the only slot on the parent\nstdout:\n{stdout}"
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001-S02 phase=planning",
    );
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
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond tick request\\tSecond body\\thttps://example.test/2\\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s prompt=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" "$SANDRONE_MAX_ATTEMPTS" "$SANDRONE_ISSUE_AGENT_PROMPT" >> .sandrone/state/agent.log
sleep 1
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

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("REQ-0001"));
    assert!(state.contains("REQ-0002"));
    assert_eq!(state.matches("decomposition-agent-running").count(), 2);
    assert!(!state.contains("finished"));
    let agent_log = workspace.join(".sandrone/state/agent.log");
    wait_for_file_contains(
        &agent_log,
        "agent called for REQ-0001 phase=decomposition max=20",
    );
    wait_for_file_contains(
        &agent_log,
        "agent called for REQ-0002 phase=decomposition max=20",
    );
    wait_for_file(&workspace.join(".sandrone/state/agents/REQ-0001.exit"));
    wait_for_file(&workspace.join(".sandrone/state/agents/REQ-0002.exit"));
    assert!(
        workspace
            .join(".sandrone/state/agents/REQ-0001.pid")
            .is_file()
    );
    assert!(
        workspace
            .join(".sandrone/state/agents/REQ-0002.pid")
            .is_file()
    );
}

#[test]
fn tick_active_cohort_blocks_new_parent_until_selected_requests_finish() {
    let workspace = temp_workspace("tick-cohort-blocks-new-parent");
    assert_success(&run(
        &workspace,
        &["new", "--name", "tick-cohort-blocks-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst request\\tBody 1\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond request\\tBody 2\\thttps://example.test/2\\nexternal-3\\ttest\\tThird request\\tBody 3\\thttps://example.test/3\\n'\n",
    )
    .expect("issue connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    write_active_loop_cohort(&workspace, &["REQ-0001", "REQ-0002"]);
    force_request_state(&workspace, "REQ-0001", "finished", "", "");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'agent called for %s phase=%s\\n' \"$SANDRONE_REQUEST_ID\" \"$SANDRONE_AGENT_PHASE\" >> .sandrone/state/agent.log\nsleep 1\n",
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--parallel-limit", "2"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dispatched REQ-0002"));
    assert!(!stdout.contains("Dispatched REQ-0003"));
    let cohort = fs::read_to_string(workspace.join(".sandrone/state/scheduler/cohort.json"))
        .expect("cohort should remain active");
    assert!(cohort.contains("REQ-0001"));
    assert!(cohort.contains("REQ-0002"));
    let progress =
        fs::read_to_string(workspace.join(".sandrone/state/scheduler/cohort-progress.json"))
            .expect("active cohort progress should be readable");
    assert!(progress.contains("\"status\": \"active\""));
    assert!(progress.contains("\"request_id\": \"REQ-0001\""));
    assert!(progress.contains("\"request_id\": \"REQ-0002\""));
    assert!(workspace.join(".sandrone/state/loop/wake").is_file());
    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0002 phase=decomposition",
    );
}

#[test]
fn tick_completes_cohort_before_scheduling_next_parent_request() {
    let workspace = temp_workspace("tick-cohort-next-batch");
    assert_success(&run(
        &workspace,
        &["new", "--name", "tick-cohort-next-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst request\\tBody 1\\thttps://example.test/1\\nexternal-2\\ttest\\tSecond request\\tBody 2\\thttps://example.test/2\\nexternal-3\\ttest\\tThird request\\tBody 3\\thttps://example.test/3\\n'\n",
    )
    .expect("issue connector should be writable");
    assert_success(&run(&workspace, &["update"]));
    write_active_loop_cohort(&workspace, &["REQ-0001", "REQ-0002"]);
    force_request_state(&workspace, "REQ-0001", "finished", "", "");
    force_request_state(&workspace, "REQ-0002", "blocked", "", "");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nset -eu\nprintf 'agent called for %s phase=%s\\n' \"$SANDRONE_REQUEST_ID\" \"$SANDRONE_AGENT_PHASE\" >> .sandrone/state/agent.log\nsleep 1\n",
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--parallel-limit", "2"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dispatched REQ-0003"));
    assert!(!stdout.contains("Dispatched REQ-0001"));
    assert!(!stdout.contains("Dispatched REQ-0002"));
    let cohort = fs::read_to_string(workspace.join(".sandrone/state/scheduler/cohort.json"))
        .expect("new cohort should be active");
    assert!(cohort.contains("REQ-0003"));
    assert!(!cohort.contains("REQ-0001"));
    assert!(
        workspace
            .join(".sandrone/state/scheduler/last-cohort.json")
            .is_file()
    );
    let last_progress =
        fs::read_to_string(workspace.join(".sandrone/state/scheduler/last-cohort-progress.json"))
            .expect("completed cohort progress should be readable");
    assert!(last_progress.contains("\"status\": \"completed\""));
    assert!(last_progress.contains("\"request_id\": \"REQ-0001\""));
    assert!(last_progress.contains("\"request_id\": \"REQ-0002\""));
    let current_progress =
        fs::read_to_string(workspace.join(".sandrone/state/scheduler/cohort-progress.json"))
            .expect("new cohort progress should be readable");
    assert!(current_progress.contains("\"request_id\": \"REQ-0003\""));
}

#[test]
fn tick_refreshes_approved_agent_result_to_wait_update_pr() {
    let workspace = temp_workspace("tick-refresh");
    assert_success(&run(&workspace, &["new", "--name", "tick-refresh-test"]));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tFirst tick request\\tBody from issue\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
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
        format!(
            r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s max=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" "$SANDRONE_MAX_ATTEMPTS" >> .sandrone/state/agent.log
case "$SANDRONE_AGENT_PHASE" in
{}
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$SANDRONE_PLAN"
    printf '\n## 第 1 轮 - planning\n- 填写计划。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
    printf '# 变更文档\n\n## 摘要\n\n已实现。\n\n## 实现前后对比\n\n已记录。\n\n## 关键设计点\n\n已记录。\n\n## 验证证据\n\n测试通过。\n\n## Review 结果\n\n等待汇总。\n' > "$SANDRONE_CHANGE_DOC"
    printf '\n## 第 2 轮 - implementation\n- 完成实现。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  *)
    echo "unexpected phase: $SANDRONE_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
            decomposition_agent_case()
        ),
    )
    .expect("issue agent should be writable");
    fs::create_dir_all(workspace.join(".sandrone/state/agents"))
        .expect("agents state dir should be writable");
    fs::write(
        workspace.join(".sandrone/state/agents/REQ-0001.hook.log"),
        "stale hook line\n",
    )
    .expect("stale hook log should be writable");
    fs::write(
        workspace.join(".sandrone/state/agents/REQ-0001.stdout.log"),
        "stale stdout line\n",
    )
    .expect("stale stdout log should be writable");
    fs::write(
        workspace.join(".sandrone/state/agents/REQ-0001.stderr.log"),
        "stale stderr line\n",
    )
    .expect("stale stderr log should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tick dispatched 1 issue-agent(s)."));

    let changes_path = workspace.join("obsidian/changes");
    let change_dir = fs::read_dir(changes_path)
        .expect("changes dir should be readable")
        .next()
        .expect("one change should exist")
        .expect("change entry should be readable")
        .path();
    wait_for_file(&workspace.join(".sandrone/state/agents/REQ-0001.exit"));
    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001-S01 phase=implementation max=20",
    );
    let implementation_runs = workspace.join("agents/implementation-agent/runs");
    let implementation_run = fs::read_dir(&implementation_runs)
        .expect("implementation agent runs dir should exist")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.join("logs/stdout.log").is_file())
        .expect("implementation run should include logs");
    assert!(implementation_run.join("logs/stderr.log").is_file());
    assert!(implementation_run.join("artifacts/runtime.json").is_file());
    wait_for_file(&workspace.join(".sandrone/state/agents/REQ-0001.exit"));
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "wait-update-pr",
    );

    let refresh_output = run(&workspace, &["tick", "--request_id", "REQ-0001"]);
    assert_success(&refresh_output);
    let refresh_stdout = String::from_utf8_lossy(&refresh_output.stdout);
    assert!(refresh_stdout.contains("Tick delivery checked REQ-0001: wait-update-pr"));

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("wait-update-pr"));
    assert!(state.contains("REQ-0001-S01"));
    assert!(state.contains("slice-finished"));
    assert!(!state.contains("\tfinished\t"));
    let slice_dir = change_dir.join("slices/S01");
    let change_doc = fs::read_to_string(request_artifact_path(
        &slice_dir,
        "REQ-0001-S01",
        "change-doc.md",
    ))
    .expect("change-doc readable");
    assert!(change_doc.contains("TestReviewer"));
    assert!(change_doc.contains("DesignReviewer"));
    assert!(
        change_dir
            .join("slices/S01/reviews/code-review/details/001-test-reviewer.json")
            .is_file()
    );
    assert!(
        change_dir
            .join("slices/S01/reviews/code-review/details/001-design-reviewer.json")
            .is_file()
    );
}

#[test]
fn agent_exit_hook_advances_request_without_waiting_for_next_tick() {
    let workspace = temp_workspace("advance-hook");
    assert_success(&run(&workspace, &["new", "--name", "advance-hook-test"]));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\\ttest\\tHook request\\tBody from issue\\thttps://example.test/1\\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
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
        format!(
            r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" >> .sandrone/state/agent.log
case "$SANDRONE_AGENT_PHASE" in
{}
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$SANDRONE_PLAN"
    printf '\n## hook - planning\n- 填写计划。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
    printf '# 变更文档\n\n## 摘要\n\n已实现。\n\n## 实现前后对比\n\n已记录。\n\n## 关键设计点\n\n已记录。\n\n## 验证证据\n\n测试通过。\n\n## Review 结果\n\n等待汇总。\n' > "$SANDRONE_CHANGE_DOC"
    printf '\n## hook - implementation\n- 完成实现。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  *)
    echo "unexpected phase: $SANDRONE_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
            decomposition_agent_case()
        ),
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);

    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001-S01 phase=implementation",
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "wait-update-pr",
    );
    let change_dir = fs::read_dir(workspace.join("obsidian/changes"))
        .expect("changes dir should be readable")
        .next()
        .expect("one change should exist")
        .expect("change entry should be readable")
        .path();
    let slice_change_doc = fs::read_to_string(
        change_dir
            .join("slices/S01")
            .join("REQ-0001-S01 change-doc.md"),
    )
    .expect("slice change-doc gate should be readable");
    assert!(slice_change_doc.contains("gate_name: change-doc"));
    assert!(slice_change_doc.contains("gate_status: approved"));
    assert!(
        workspace
            .join(".sandrone/state/agents/REQ-0001.hook.log")
            .is_file()
    );
    let hook_log = fs::read_to_string(workspace.join(".sandrone/state/agents/REQ-0001.hook.log"))
        .expect("hook log should be readable");
    assert!(!hook_log.contains("stale hook line"));
    let stdout_log =
        fs::read_to_string(workspace.join(".sandrone/state/agents/REQ-0001.stdout.log"))
            .expect("stdout log should be readable");
    assert!(!stdout_log.contains("stale stdout line"));
    let stderr_log =
        fs::read_to_string(workspace.join(".sandrone/state/agents/REQ-0001.stderr.log"))
            .expect("stderr log should be readable");
    assert!(!stderr_log.contains("stale stderr line"));
}

#[test]
fn agent_nonzero_exit_with_document_status_advances_to_review() {
    let workspace = temp_workspace("agent-document-status");
    assert_success(&run(
        &workspace,
        &["new", "--name", "agent-document-status-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\ttest\tMarker request\tBody from issue\thttps://example.test/1\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
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
        format!(
            r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" >> .sandrone/state/agent.log
case "$SANDRONE_AGENT_PHASE" in
{}
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$SANDRONE_PLAN"
    printf '\n## document status - planning\n- 填写计划。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
    cat > "$SANDRONE_CHANGE_DOC" <<EOF
---
sandrone_schema: 1
request_id: $SANDRONE_REQUEST_ID
document_type: change-doc
agent_phase: implementation
agent_status: submitted
agent_ready_for_review: true
format_check_status: passed
format_check_exit_code: 0
updated_at: test
---

# 变更文档

## 摘要

已实现。

## 实现前后对比

已记录。

## 关键设计点

已记录。

## 验证证据

测试通过。

## Review 结果

等待汇总。
EOF
    printf '\n## document status - implementation\n- 完成实现。\n' >> "$SANDRONE_AGENT_JOURNAL"
    exit 1
    ;;
  *)
    echo "unexpected phase: $SANDRONE_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
            decomposition_agent_case()
        ),
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);

    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001-S01 phase=implementation",
    );
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "wait-update-pr",
    );
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(!state.contains("blocked"));
    let exit_code = fs::read_to_string(workspace.join(".sandrone/state/agents/REQ-0001-S01.exit"))
        .expect("agent exit should be readable");
    assert_eq!(exit_code.trim(), "1");
    let events_path = workspace.join(".sandrone/state/events.ndjson");
    wait_for_file_contains(&events_path, "agent_document_status_used");
    let events = fs::read_to_string(events_path).expect("events should be readable");
    assert!(events.contains("agent_document_status_used"));
    assert!(events.contains("artifact=obsidian/changes"));
    assert!(
        !workspace
            .join(".sandrone/state/agents/REQ-0001-S01.success")
            .exists(),
        "new workflow should not require a separate success file"
    );
    let change_doc = fs::read_to_string(
        workspace
            .join("obsidian/changes")
            .read_dir()
            .expect("changes readable")
            .next()
            .expect("change exists")
            .expect("change entry readable")
            .path()
            .join("slices/S01/REQ-0001-S01 change-doc.md"),
    )
    .expect("change doc readable");
    assert!(change_doc.contains("agent_status: submitted"));
    assert!(change_doc.contains("agent_ready_for_review: true"));
    let doc_status = run(&workspace, &["doc-status", "--request_id", "REQ-0001-S01"]);
    assert_success(&doc_status);
    let stdout = String::from_utf8_lossy(&doc_status.stdout);
    assert!(stdout.contains("document_type: change-doc"));
    assert!(stdout.contains("agent_status: submitted"));
    assert!(stdout.contains("format_check_status: passed"));
    assert!(stdout.contains("gate_name: change-doc"));
    assert!(stdout.contains("gate_status: approved"));
}

#[test]
fn agent_nonzero_exit_without_document_status_still_blocks() {
    let workspace = temp_workspace("agent-no-document-status");
    assert_success(&run(
        &workspace,
        &["new", "--name", "agent-no-document-status-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\ttest\tNo status request\tBody from issue\thttps://example.test/1\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
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
        format!(
            r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" >> .sandrone/state/agent.log
case "$SANDRONE_AGENT_PHASE" in
{}
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$SANDRONE_PLAN"
    printf '\n## no document status - planning\n- 填写计划。\n' >> "$SANDRONE_AGENT_JOURNAL"
    ;;
  implementation)
    printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
    printf '# 变更文档\n\n## 摘要\n\n已实现。\n\n## 实现前后对比\n\n已记录。\n\n## 关键设计点\n\n已记录。\n\n## 验证证据\n\n测试通过。\n\n## Review 结果\n\n等待汇总。\n' > "$SANDRONE_CHANGE_DOC"
    printf '\n## no document status - implementation\n- 完成实现但没有提交状态。\n' >> "$SANDRONE_AGENT_JOURNAL"
    exit 1
    ;;
  *)
    echo "unexpected phase: $SANDRONE_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
            decomposition_agent_case()
        ),
    )
    .expect("issue agent should be writable");

    let output = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&output);

    wait_for_file_contains(
        &workspace.join(".sandrone/state/agent.log"),
        "agent called for REQ-0001-S01 phase=implementation",
    );
    wait_for_file_contains(&workspace.join(".sandrone/state/requests.tsv"), "blocked");
    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("state should be readable");
    assert!(state.contains("REQ-0001-S01"));
    assert!(state.contains("blocked"));
    assert!(!state.contains("wait-update-pr"));
    assert!(
        !workspace
            .join(".sandrone/state/agents/REQ-0001-S01.success")
            .exists()
    );
    let events_path = workspace.join(".sandrone/state/events.ndjson");
    wait_for_file_contains(&events_path, "implementation agent exited with code 1");
    let events = fs::read_to_string(events_path).expect("events should be readable");
    assert!(events.contains("implementation agent exited with code 1"));
    assert!(!events.contains("agent_document_status_used"));
}

#[test]
fn resume_code_review_gate_unavailable_retries_review_without_dispatching_impl() {
    let workspace = temp_workspace("resume-code-review-gate-unavailable");
    assert_success(&run(
        &workspace,
        &["new", "--name", "resume-code-review-gate-test"],
    ));
    install_deterministic_request_scheduler(&workspace);
    fs::write(
        workspace.join("tools/issue-update.sh"),
        "#!/usr/bin/env sh\nprintf 'external-1\ttest\tReview gate request\tBody from issue\thttps://example.test/1\n'\n",
    )
    .expect("issue connector should be writable");
    install_passing_decomposition_review_tool(&workspace);
    fs::write(
        workspace.join("tools/plan-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"PlanReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"plan ok\",\"process\":[\"checked plan\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("plan review connector should be writable");
    fs::write(
        workspace.join("tools/check-format.sh"),
        "#!/usr/bin/env sh\nexit 0\n",
    )
    .expect("format connector should be writable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":false,\"gate_unavailable\":true,\"decision\":\"rejected\",\"recommended_next_phase\":\"blocked\",\"summary\":\"review backend offline\",\"process\":[\"checked backend\"],\"critical\":[{\"title\":\"review backend offline\",\"evidence\":\"test reviewer backend is unavailable\",\"impact\":\"code-review cannot make a reliable decision\",\"required_fix\":\"restore reviewer backend\",\"suggested_change\":\"rerun code-review after backend is available\",\"verification\":\"summary gate_unavailable becomes false\"}],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be writable");
    fs::write(
        workspace.join("tools/design-review.sh"),
        "#!/usr/bin/env sh\nprintf '{\"reviewer\":\"DesignReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"design ok\",\"process\":[\"checked design\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("design review connector should be writable");
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        format!(
            r#"#!/usr/bin/env sh
set -eu
printf 'agent called for %s phase=%s\n' "$SANDRONE_REQUEST_ID" "$SANDRONE_AGENT_PHASE" >> .sandrone/state/agent.log
case "$SANDRONE_AGENT_PHASE" in
{}
  planning)
    printf '# 计划\n\n## 规范化需求记录\n\n保留。\n\n## 需求理解\n\n已填写。\n\n## 测试与验证\n\n已规划。\n' > "$SANDRONE_PLAN"
    ;;
  implementation)
    printf 'implemented\n' > "$SANDRONE_WORKTREE/feature.txt"
    cat > "$SANDRONE_CHANGE_DOC" <<EOF
---
sandrone_schema: 1
request_id: $SANDRONE_REQUEST_ID
document_type: change-doc
agent_phase: implementation
agent_status: submitted
agent_ready_for_review: true
format_check_status: passed
format_check_exit_code: 0
updated_at: test
---

# 变更文档

## 摘要

已实现。

## 实现前后对比

已记录。

## 关键设计点

已记录。

## 验证证据

测试通过。

## Review 结果

等待汇总。
EOF
    ;;
  *)
    echo "unexpected phase: $SANDRONE_AGENT_PHASE" >&2
    exit 1
    ;;
esac
"#,
            decomposition_agent_case()
        ),
    )
    .expect("issue agent should be writable");

    let first_tick = run(&workspace, &["tick", "--max-attempts", "20"]);
    assert_success(&first_tick);
    wait_for_file_contains(
        &workspace.join(".sandrone/state/requests.tsv"),
        "REQ-0001-S01",
    );
    wait_for_file_contains(&workspace.join(".sandrone/state/requests.tsv"), "blocked");

    let change_root = workspace
        .join("obsidian/changes")
        .read_dir()
        .expect("changes readable")
        .next()
        .expect("change exists")
        .expect("change entry readable")
        .path();
    let status_path = change_root.join("slices/S01/status.json");
    let blocked_status = fs::read_to_string(&status_path).expect("status readable");
    assert!(blocked_status.contains("\"status\": \"blocked\""));
    assert!(blocked_status.contains("code-review gate unavailable"));
    let summary =
        fs::read_to_string(change_root.join("slices/S01/reviews/code-review/summary.json"))
            .expect("summary readable");
    assert!(summary.contains("\"gate_unavailable\": true"));

    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nprintf 'unexpected agent phase=%s\\n' \"$SANDRONE_AGENT_PHASE\" >> .sandrone/state/unexpected-agent.log\nexit 1\n",
    )
    .expect("issue agent should be replaceable");
    fs::write(
        workspace.join("tools/test-review.sh"),
        "#!/usr/bin/env sh\nprintf 'test-review rerun\\n' >> .sandrone/state/review-rerun.log\nprintf '{\"reviewer\":\"TestReviewer\",\"approved\":true,\"gate_unavailable\":false,\"decision\":\"approved\",\"recommended_next_phase\":\"implementation\",\"summary\":\"tests ok after backend recovery\",\"process\":[\"checked tests\"],\"critical\":[],\"high\":[],\"warning\":[],\"info\":[]}'\n",
    )
    .expect("test review connector should be replaceable");

    let resume = run(&workspace, &["resume", "--request_id", "REQ-0001-S01"]);
    assert_success(&resume);
    let resume_stdout = String::from_utf8_lossy(&resume.stdout);
    assert!(resume_stdout.contains("resumed status: change-doc-submitted"));
    let resumed_state =
        fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).expect("state readable");
    assert!(resumed_state.contains("REQ-0001-S01"));
    assert!(resumed_state.contains("\tchange-doc-submitted\t"));

    let second_tick = run(
        &workspace,
        &[
            "tick",
            "--request_id",
            "REQ-0001-S01",
            "--max-attempts",
            "20",
        ],
    );
    assert_success(&second_tick);
    advance_until_not_review_running(&workspace, "REQ-0001-S01");
    assert!(
        !workspace
            .join(".sandrone/state/unexpected-agent.log")
            .exists(),
        "resume from gate_unavailable should rerun code-review without dispatching implementation"
    );
    let review_log = fs::read_to_string(workspace.join(".sandrone/state/review-rerun.log"))
        .expect("review rerun log readable");
    assert!(review_log.contains("test-review rerun"));
    wait_for_file_contains_any(
        &workspace.join(".sandrone/state/requests.tsv"),
        &["slice-finished", "wait-update-pr"],
    );
    let final_state =
        fs::read_to_string(workspace.join(".sandrone/state/requests.tsv")).expect("state readable");
    assert!(final_state.contains("REQ-0001-S01"));
    assert!(final_state.contains("slice-finished") || final_state.contains("wait-update-pr"));
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

    let state_path = workspace.join(".sandrone/state/requests.tsv");
    let state = fs::read_to_string(&state_path).expect("state should be readable");
    fs::write(
        &state_path,
        state.replace("\tplanning\t", "\tplanning-agent-running\t"),
    )
    .expect("state should be writable");
    let agents_dir = workspace.join(".sandrone/state/agents");
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
            .join("obsidian/changes")
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
    let change_path = workspace.join("obsidian/changes").join(&change_name);
    let status = fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(status.contains("\"status\": \"blocked\""));
    assert!(status.contains("code-review failed after 20 attempts"));
    let recovery = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "recovery.md",
    ))
    .expect("recovery readable");
    assert!(recovery.contains("恢复指南"));
    assert!(recovery.contains("code-review failed after 20 attempts"));

    let resume = run(&workspace, &["resume", "--request_id", "REQ-0001"]);
    assert_success(&resume);
    let stdout = String::from_utf8_lossy(&resume.stdout);
    assert!(stdout.contains("REQ-0001 recovery.md"));
    assert!(stdout.contains("REQ-0001 agent-journal.md"));
    assert!(stdout.contains("sandrone loop restart --request_id REQ-0001"));
    assert!(stdout.contains("sandrone loop start"));
    assert!(stdout.contains("resumed status: planning"));

    let resumed_state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
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
        "#!/usr/bin/env sh\nset -eu\nprintf 'phase=%s\\n' \"$SANDRONE_AGENT_PHASE\" >> .sandrone/state/resume-agent.log\n",
    )
    .expect("issue agent should be writable");
    let tick = run(&workspace, &["tick", "--request_id", "REQ-0001"]);
    assert_success(&tick);
    let tick_stdout = String::from_utf8_lossy(&tick.stdout);
    assert!(tick_stdout.contains("Dispatched REQ-0001 phase decomposition"));
}

#[test]
fn loop_stop_and_restart_wrap_request_block_and_resume() {
    let workspace = temp_workspace("loop-stop-restart");
    let change_name = format!("{}-loop-stop-restart", current_date());
    assert_success(&run(
        &workspace,
        &["new", "--name", "loop-stop-restart-test"],
    ));
    assert_success(&run(
        &workspace,
        &["plan", "--name", &change_name, "--request_id", "REQ-0001"],
    ));

    let stop = run(
        &workspace,
        &[
            "loop",
            "stop",
            "--request_id",
            "REQ-0001",
            "--reason",
            "pause from public loop stop",
        ],
    );
    assert_success(&stop);
    let stop_stdout = String::from_utf8_lossy(&stop.stdout);
    assert!(stop_stdout.contains("Request REQ-0001 stopped."));
    assert!(stop_stdout.contains("status: blocked"));
    assert!(stop_stdout.contains("pause from public loop stop"));

    let change_path = workspace.join("obsidian/changes").join(&change_name);
    let status = fs::read_to_string(change_path.join("status.json")).expect("status readable");
    assert!(status.contains("\"status\": \"blocked\""));
    assert!(status.contains("pause from public loop stop"));
    let recovery = fs::read_to_string(request_artifact_path(
        &change_path,
        "REQ-0001",
        "recovery.md",
    ))
    .expect("recovery readable");
    assert!(recovery.contains("sandrone loop restart --request_id REQ-0001"));

    let restart = run(&workspace, &["loop", "restart", "--request_id", "REQ-0001"]);
    assert_success(&restart);
    let restart_stdout = String::from_utf8_lossy(&restart.stdout);
    assert!(restart_stdout.contains("Loop restart prepared 1 resumed request(s)."));
    assert!(restart_stdout.contains("next: sandrone loop start"));

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("requests state readable");
    assert!(state.contains("REQ-0001"));
    assert!(state.contains("\tplan-review-rejected\t"));
    assert!(!workspace.join(".sandrone/state/loop/pid").exists());
}

#[test]
fn doctor_reports_workspace_and_reviewer_readiness() {
    let workspace = temp_workspace("doctor");
    assert_success(&run(&workspace, &["new", "--name", "doctor-test"]));

    let output = run(&workspace, &["doctor"]);
    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sandrone Doctor Report"));
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

    let events = fs::read_to_string(workspace.join(".sandrone/state/events.ndjson"))
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
    fs::write(
        workspace.join("tools/issue-agent.sh"),
        "#!/usr/bin/env sh\nprintf 'agent phase=%s\\n' \"$SANDRONE_AGENT_PHASE\" >> .sandrone/state/agent-dispatch.log\n",
    )
    .expect("issue agent should be writable");

    let rejected = run(&workspace, &["code-review", "--request_id", "REQ-0001"]);
    assert_success(&rejected);
    advance_until_not_review_running(&workspace, "REQ-0001");

    let state = fs::read_to_string(workspace.join(".sandrone/state/requests.tsv"))
        .expect("request state should be readable");
    assert!(
        state.contains("plan-review-rejected")
            || state.contains("planning-agent-running")
            || state.contains("plan-submitted")
            || state.contains("plan-review-running")
            || state.contains("blocked"),
        "code-review planning recommendation should return to planning, submit/review the revised plan, or block on the follow-up planning gate; state was:\n{state}"
    );
    let summary = fs::read_to_string(
        workspace
            .join("obsidian/changes")
            .join(&change_name)
            .join("reviews/code-review/summary.json"),
    )
    .expect("code review summary should be readable");
    assert!(summary.contains("\"recommended_next_phase\": \"planning\""));
    let events = fs::read_to_string(workspace.join(".sandrone/state/events.ndjson"))
        .expect("events should be readable");
    assert!(events.contains("code-review requested planning"));
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

fn assert_no_archived_entries(path: &Path) {
    let entries = fs::read_dir(path).expect("directory should be readable");
    for entry in entries {
        let entry = entry.expect("directory entry should be readable");
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        assert!(
            !file_name.starts_with("archived-"),
            "upgrade should remove obsolete artifacts instead of keeping {file_name}"
        );
    }
}
