use super::*;
use std::io::Write;

const AGENTS_RUNTIME_DIR: &str = "agents";
const AGENT_CONFIG_DIR: &str = "agents/config";
const JOBS_STATE_DIR: &str = ".sandrone/state/jobs";
const LEGACY_AGENT_STATE_DIR: &str = ".sandrone/state/agents";
const LEGACY_REVIEW_STATE_DIR: &str = ".sandrone/state/reviews";

pub(crate) fn prepare_agent_runtime_dirs() -> Result<()> {
    for kind in agent_runtime_kinds() {
        let runs_dir = agent_runtime_kind_dir(kind).join("runs");
        fs::create_dir_all(&runs_dir)?;
        write_agent_runtime_config(kind)?;
    }
    Ok(())
}

pub(crate) fn runtime_agent_job_state_dir(request_id: &str) -> PathBuf {
    if let Some(run_dir) = read_runtime_pointer(&agent_run_pointer_path(request_id)) {
        return run_dir;
    }
    Path::new(JOBS_STATE_DIR)
        .join(request_id)
        .join("agent")
        .join("current")
        .join("issue-agent")
}

pub(crate) fn create_agent_run_state_dir(request_id: &str, phase: &str) -> Result<PathBuf> {
    let kind = agent_kind_for_phase(phase);
    let run_dir = new_agent_runtime_run_dir(kind, &[request_id, phase])?;
    write_runtime_pointer(&agent_run_pointer_path(request_id), &run_dir)?;
    Ok(run_dir)
}

pub(crate) fn create_review_job_run_state_dir(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> Result<PathBuf> {
    let kind = file_stem;
    let run_dir = new_agent_runtime_run_dir(kind, &[request_id, stage, &format!("{attempt:03}")])?;
    write_runtime_pointer(
        &review_job_run_pointer_path(request_id, stage, attempt, file_stem),
        &run_dir,
    )?;
    Ok(run_dir)
}

pub(crate) fn create_named_agent_run_state_dir(
    kind: &str,
    components: &[&str],
    _stage: &str,
    _attempt: &str,
    _worker: &str,
) -> Result<PathBuf> {
    let run_dir = new_agent_runtime_run_dir(kind, components)?;
    Ok(run_dir)
}

pub(crate) fn runtime_review_job_state_dir(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    if let Some(run_dir) = read_runtime_pointer(&review_job_run_pointer_path(
        request_id, stage, attempt, file_stem,
    )) {
        return run_dir;
    }
    legacy_canonical_review_job_state_dir(request_id, stage, attempt, file_stem)
}

pub(crate) fn runtime_review_context_dir(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    job_artifacts_dir(&runtime_review_job_state_dir(
        request_id, stage, attempt, file_stem,
    ))
    .join("review-context")
}

pub(crate) fn agent_runtime_kind_dir(kind: &str) -> PathBuf {
    Path::new(AGENTS_RUNTIME_DIR).join(kind)
}

pub(crate) fn job_artifacts_dir(job_dir: &Path) -> PathBuf {
    if uses_agent_runtime_layout(job_dir) {
        job_dir.join("artifacts")
    } else {
        job_dir.to_path_buf()
    }
}

pub(crate) fn job_logs_dir(job_dir: &Path) -> PathBuf {
    if uses_agent_runtime_layout(job_dir) {
        job_dir.join("logs")
    } else {
        job_dir.to_path_buf()
    }
}

fn agent_runtime_kinds() -> &'static [&'static str] {
    &[
        "decomposition-agent",
        "plan-agent",
        "implementation-agent",
        "rebase-agent",
        "decomposition-reviewer",
        "plan-reviewer",
        "test-reviewer",
        "design-reviewer",
        "integration-reviewer",
        "merge-planner",
    ]
}

pub(crate) fn agent_kind_for_phase(phase: &str) -> &'static str {
    match phase {
        "decomposition" => "decomposition-agent",
        "planning" => "plan-agent",
        "implementation" => "implementation-agent",
        "rebase" => "rebase-agent",
        _ => "issue-agent",
    }
}

fn new_agent_runtime_run_dir(kind: &str, components: &[&str]) -> Result<PathBuf> {
    let run_id = format!(
        "{}-{}",
        filesystem_timestamp(),
        components
            .iter()
            .map(|component| slugify_run_component(component))
            .filter(|component| !component.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    );
    let run_dir = agent_runtime_kind_dir(kind).join("runs").join(run_id);
    fs::create_dir_all(job_logs_dir(&run_dir))?;
    fs::create_dir_all(job_artifacts_dir(&run_dir))?;
    fs::create_dir_all(run_dir.join("state"))?;
    write_agent_runtime_config(kind)?;
    Ok(run_dir)
}

fn filesystem_timestamp() -> String {
    let base = now_string()
        .replace([':', '-'], "")
        .replace(['.', '+'], "_");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);
    format!("{base}-{nanos:09}")
}

fn slugify_run_component(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for character in value.chars().flat_map(|character| character.to_lowercase()) {
        if character.is_ascii_alphanumeric() {
            out.push(character);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn agent_run_pointer_path(request_id: &str) -> PathBuf {
    legacy_agent_state_dir().join(format!("{request_id}.run-dir"))
}

fn review_job_run_pointer_path(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_canonical_review_job_state_dir(request_id, stage, attempt, file_stem).join("run-dir")
}

fn read_runtime_pointer(path: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(path).ok()?;
    let run_dir = PathBuf::from(content.trim());
    if run_dir.exists() {
        Some(run_dir)
    } else {
        None
    }
}

fn write_runtime_pointer(path: &Path, run_dir: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", run_dir.to_string_lossy()))?;
    Ok(())
}

fn write_agent_runtime_config(kind: &str) -> Result<()> {
    let config_dir = Path::new(AGENT_CONFIG_DIR);
    fs::create_dir_all(config_dir)?;
    let config_path = config_dir.join(format!("{kind}.json"));
    if !config_path.exists() {
        fs::write(
            &config_path,
            format!(
                "{{\n  \"kind\": \"{kind}\",\n  \"agent_backend\": \"codex-cli\",\n  \"model\": \"\",\n  \"reasoning_effort\": \"\",\n  \"api_key\": \"\",\n  \"base_url\": \"\"\n}}\n",
                kind = kind,
            ),
        )?;
    }
    Ok(())
}

pub(crate) fn legacy_agent_state_dir() -> PathBuf {
    Path::new(LEGACY_AGENT_STATE_DIR).to_path_buf()
}

pub(crate) fn legacy_review_stage_state_dir(request_id: &str, stage: &str) -> PathBuf {
    Path::new(LEGACY_REVIEW_STATE_DIR)
        .join(request_id)
        .join(stage)
}

pub(crate) fn legacy_review_job_state_dir(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    legacy_review_stage_state_dir(request_id, stage)
        .join(format!("{attempt:03}"))
        .join(file_stem)
}

fn legacy_canonical_review_job_state_dir(
    request_id: &str,
    stage: &str,
    attempt: u32,
    file_stem: &str,
) -> PathBuf {
    Path::new(JOBS_STATE_DIR)
        .join(request_id)
        .join(stage)
        .join(format!("{attempt:03}"))
        .join(file_stem)
}

fn uses_agent_runtime_layout(job_dir: &Path) -> bool {
    let components = job_dir
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    components
        .windows(3)
        .any(|window| window[0] == AGENTS_RUNTIME_DIR && window[2] == "runs")
}

pub(crate) fn job_pid_path(job_dir: &Path) -> PathBuf {
    if uses_agent_runtime_layout(job_dir) {
        job_dir.join("state").join("pid")
    } else {
        job_dir.join("pid")
    }
}

pub(crate) fn job_exit_path(job_dir: &Path) -> PathBuf {
    if uses_agent_runtime_layout(job_dir) {
        job_dir.join("state").join("exit")
    } else {
        job_dir.join("exit")
    }
}

pub(crate) fn job_stdout_path(job_dir: &Path) -> PathBuf {
    job_logs_dir(job_dir).join("stdout.log")
}

pub(crate) fn job_stderr_path(job_dir: &Path) -> PathBuf {
    job_logs_dir(job_dir).join("stderr.log")
}

pub(crate) fn job_hook_log_path(job_dir: &Path) -> PathBuf {
    job_logs_dir(job_dir).join("hook.log")
}

pub(crate) fn job_runtime_path(job_dir: &Path) -> PathBuf {
    job_artifacts_dir(job_dir).join("runtime.json")
}

pub(crate) fn job_events_log_path(job_dir: &Path) -> PathBuf {
    job_logs_dir(job_dir).join("events.log")
}

pub(crate) fn create_truncated_runtime_file(
    canonical: impl AsRef<Path>,
    legacy: Option<&Path>,
) -> Result<fs::File> {
    let canonical = canonical.as_ref();
    if let Some(parent) = canonical.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(canonical)?;
    if let Some(legacy) = legacy {
        link_legacy_runtime_file(canonical, legacy)?;
    }
    Ok(file)
}

pub(crate) fn mirror_runtime_file(canonical: &Path, mirror: &Path) -> Result<()> {
    link_legacy_runtime_file(canonical, mirror)
}

pub(crate) fn write_runtime_text(
    canonical: impl AsRef<Path>,
    content: &str,
    legacy: Option<&Path>,
) -> Result<()> {
    let canonical = canonical.as_ref();
    if let Some(parent) = canonical.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(canonical, content)?;
    if let Some(legacy) = legacy {
        if let Some(parent) = legacy.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(legacy, content)?;
    }
    Ok(())
}

pub(crate) fn remove_runtime_file(
    canonical: impl AsRef<Path>,
    legacy: Option<&Path>,
) -> Result<()> {
    remove_file_if_exists(canonical.as_ref())?;
    if let Some(legacy) = legacy {
        remove_file_if_exists(legacy)?;
    }
    Ok(())
}

pub(crate) fn read_runtime_text(
    canonical: impl AsRef<Path>,
    legacy: Option<&Path>,
) -> Result<String> {
    let canonical = canonical.as_ref();
    if canonical.exists() {
        return Ok(fs::read_to_string(canonical)?);
    }
    if let Some(legacy) = legacy
        && legacy.exists()
    {
        return Ok(fs::read_to_string(legacy)?);
    }
    Ok(String::new())
}

pub(crate) fn existing_runtime_path(canonical: PathBuf, legacy: PathBuf) -> PathBuf {
    if canonical.exists() {
        canonical
    } else if legacy.exists() {
        legacy
    } else {
        canonical
    }
}

pub(crate) fn append_job_event(path: impl AsRef<Path>, event: &str, detail: &str) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let detail = detail
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}\t{}\t{}", now_string(), event, detail)?;
    Ok(())
}

pub(crate) struct JobRuntime<'a> {
    pub kind: &'a str,
    pub request_id: &'a str,
    pub stage: &'a str,
    pub attempt: &'a str,
    pub worker: &'a str,
    pub tool: &'a str,
    pub pid: Option<u32>,
    pub status: &'a str,
}

pub(crate) fn write_job_runtime(path: impl AsRef<Path>, runtime: &JobRuntime<'_>) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pid = runtime.pid.map(|pid| pid.to_string()).unwrap_or_default();
    fs::write(
        path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"kind\": \"{}\",\n  \"request_id\": \"{}\",\n  \"stage\": \"{}\",\n  \"attempt\": \"{}\",\n  \"worker\": \"{}\",\n  \"tool\": \"{}\",\n  \"pid\": \"{}\",\n  \"status\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(runtime.kind),
            json_escape(runtime.request_id),
            json_escape(runtime.stage),
            json_escape(runtime.attempt),
            json_escape(runtime.worker),
            json_escape(runtime.tool),
            json_escape(&pid),
            json_escape(runtime.status),
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn link_legacy_runtime_file(canonical: &Path, legacy: &Path) -> Result<()> {
    if let Some(parent) = legacy.parent() {
        fs::create_dir_all(parent)?;
    }
    remove_file_if_exists(legacy)?;
    match fs::hard_link(canonical, legacy) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::write(legacy, "")?;
            Ok(())
        }
    }
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
