use super::*;
use std::io::Write;

const JOBS_STATE_DIR: &str = ".sandrone/state/jobs";
const LEGACY_AGENT_STATE_DIR: &str = ".sandrone/state/agents";
const LEGACY_REVIEW_STATE_DIR: &str = ".sandrone/state/reviews";

pub(crate) fn runtime_agent_job_state_dir(request_id: &str) -> PathBuf {
    Path::new(JOBS_STATE_DIR)
        .join(request_id)
        .join("agent")
        .join("current")
        .join("issue-agent")
}

pub(crate) fn runtime_review_job_state_dir(
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

pub(crate) fn job_pid_path(job_dir: &Path) -> PathBuf {
    job_dir.join("pid")
}

pub(crate) fn job_exit_path(job_dir: &Path) -> PathBuf {
    job_dir.join("exit")
}

pub(crate) fn job_stdout_path(job_dir: &Path) -> PathBuf {
    job_dir.join("stdout.log")
}

pub(crate) fn job_stderr_path(job_dir: &Path) -> PathBuf {
    job_dir.join("stderr.log")
}

pub(crate) fn job_hook_log_path(job_dir: &Path) -> PathBuf {
    job_dir.join("hook.log")
}

pub(crate) fn job_runtime_path(job_dir: &Path) -> PathBuf {
    job_dir.join("runtime.json")
}

pub(crate) fn job_events_log_path(job_dir: &Path) -> PathBuf {
    job_dir.join("events.log")
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
