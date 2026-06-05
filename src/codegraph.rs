use super::*;

#[derive(Clone, Debug)]
pub(crate) enum CodegraphInitOutcome {
    SkippedEmptyRepo,
    AlreadyInitialized,
    Initialized,
    CommandUnavailable(String),
    Failed(String),
}

#[derive(Clone, Debug)]
pub(crate) enum CodegraphContextOutcome {
    SkippedEmptyRepo,
    Ready(String),
    CommandUnavailable(String),
    Failed(String),
}

pub(crate) fn codegraph_bin() -> String {
    env::var("SANDRONE_CODEGRAPH_BIN").unwrap_or_else(|_| "codegraph".to_string())
}

pub(crate) fn codegraph_index_ready(cwd: &str) -> bool {
    Path::new(cwd).join(".codegraph").is_dir()
}

pub(crate) fn ensure_codegraph_initialized(cwd: &str) -> CodegraphInitOutcome {
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

pub(crate) fn refresh_codegraph_context(cwd: &str) -> CodegraphContextOutcome {
    if !repo_has_commits(cwd) {
        return CodegraphContextOutcome::SkippedEmptyRepo;
    }
    match ensure_codegraph_initialized(cwd) {
        CodegraphInitOutcome::CommandUnavailable(detail) => {
            return CodegraphContextOutcome::CommandUnavailable(detail);
        }
        CodegraphInitOutcome::Failed(detail) => return CodegraphContextOutcome::Failed(detail),
        CodegraphInitOutcome::SkippedEmptyRepo
        | CodegraphInitOutcome::AlreadyInitialized
        | CodegraphInitOutcome::Initialized => {}
    }

    let output_path = codegraph_context_path();
    if let Some(parent) = output_path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        return CodegraphContextOutcome::Failed(format!(
            "failed to create {}: {error}",
            parent.display()
        ));
    }

    let bin = codegraph_bin();
    let task = "Summarize the target repository for sandrone planning: product purpose, architecture, key entry points, state/data flow, tests, extension points, risky areas, and likely files to inspect before implementing issues.";
    match Command::new(&bin)
        .args([
            "context",
            "-p",
            cwd,
            "--max-nodes",
            "80",
            "--max-code",
            "20",
            task,
        ])
        .output()
    {
        Ok(output) if output.status.success() => {
            let body = String::from_utf8_lossy(&output.stdout);
            let content = format!(
                "# CodeGraph Context\n\n- Generated: `{}`\n- Target repo: `{}`\n- Command: `{} context -p {} --max-nodes 80 --max-code 20 <task>`\n\n{}\n",
                now_string(),
                cwd,
                bin,
                cwd,
                body.trim()
            );
            match fs::write(&output_path, content) {
                Ok(()) => CodegraphContextOutcome::Ready(output_path.display().to_string()),
                Err(error) => CodegraphContextOutcome::Failed(format!(
                    "failed to write {}: {error}",
                    output_path.display()
                )),
            }
        }
        Ok(output) => {
            let stderr = review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
            CodegraphContextOutcome::Failed(format!("{bin} context failed: {stderr}"))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            CodegraphContextOutcome::CommandUnavailable(format!("{bin} unavailable: {error}"))
        }
        Err(error) => {
            CodegraphContextOutcome::Failed(format!("{bin} context could not run: {error}"))
        }
    }
}

pub(crate) fn codegraph_preflight_note(outcome: &CodegraphInitOutcome) -> String {
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

pub(crate) fn codegraph_context_preflight_note(outcome: &CodegraphContextOutcome) -> String {
    match outcome {
        CodegraphContextOutcome::SkippedEmptyRepo => {
            "CodeGraph context 跳过: 目标仓库为空。".to_string()
        }
        CodegraphContextOutcome::Ready(path) => {
            format!("CodeGraph context ready: 已刷新 {path}。")
        }
        CodegraphContextOutcome::CommandUnavailable(detail) => {
            format!(
                "CodeGraph context 不可用: {detail}。请安装 @colbymchenry/codegraph，或设置 SANDRONE_CODEGRAPH_BIN。"
            )
        }
        CodegraphContextOutcome::Failed(detail) => {
            format!(
                "CodeGraph context 刷新失败: {detail}。可手动运行 codegraph init -i dev/repo 后再运行 codegraph context -p dev/repo <task>。"
            )
        }
    }
}

pub(crate) fn print_codegraph_init_outcome(prefix: &str, outcome: &CodegraphInitOutcome) {
    println!("{prefix}{}", codegraph_preflight_note(outcome));
}

pub(crate) fn print_codegraph_context_outcome(prefix: &str, outcome: &CodegraphContextOutcome) {
    println!("{prefix}{}", codegraph_context_preflight_note(outcome));
}

pub(crate) fn codegraph_event_status(outcome: &CodegraphInitOutcome) -> &'static str {
    match outcome {
        CodegraphInitOutcome::Initialized | CodegraphInitOutcome::AlreadyInitialized => "ready",
        CodegraphInitOutcome::SkippedEmptyRepo => "skipped",
        CodegraphInitOutcome::CommandUnavailable(_) | CodegraphInitOutcome::Failed(_) => "warning",
    }
}

pub(crate) fn codegraph_outcome_detail(outcome: &CodegraphInitOutcome) -> String {
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

pub(crate) fn codegraph_refresh_required() -> Result<bool> {
    if !repo_has_commits(DEV_REPO) {
        return Ok(false);
    }
    let codegraph_path = codegraph_context_path();
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

fn codegraph_context_path() -> PathBuf {
    Path::new("obsidian/codegraph/context.md").to_path_buf()
}
