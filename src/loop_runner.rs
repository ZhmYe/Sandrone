use super::*;
use notify::{RecursiveMode, Watcher};
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) fn loop_command(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    let Some(action) = args.first().map(String::as_str) else {
        return Err("usage: sandrone loop <start|restart|stop>".into());
    };
    let rest = &args[1..];
    match action {
        "run-once" => {
            write_loop_status("run-once", None, "running one loop iteration in foreground")?;
            let result = tick(rest);
            match &result {
                Ok(()) => write_loop_status("idle", None, "run-once completed")?,
                Err(error) => write_loop_status("error", None, &error.to_string())?,
            }
            result
        }
        "start" => loop_start(rest),
        "restart" => loop_restart(rest),
        "stop" => loop_stop(rest),
        "status" => loop_status(),
        _ => Err("usage: sandrone loop <start|restart|stop>".into()),
    }
}

fn loop_start(args: &[String]) -> Result<()> {
    ensure_allowed_flags(
        args,
        &[
            "--interval-seconds",
            "--max-attempts",
            "--parallel-limit",
            "--parallel_limit",
        ],
    )?;
    fs::create_dir_all(loop_state_dir())?;
    if let Some(pid) = read_loop_pid()?
        && process_is_running(pid)
    {
        println!("Loop already running pid {pid}.");
        println!("  status: {}", loop_status_path().display());
        return Ok(());
    }
    remove_file_if_exists_local(&loop_stop_path())?;
    let interval = parse_loop_interval(flag_value(args, "--interval-seconds")?)?;
    let stdout = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(loop_stdout_path())?;
    let stderr = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(loop_stderr_path())?;
    let current_exe = env::current_exe()?;
    let worker_args = loop_tick_args(args);
    let mut command = Command::new(current_exe);
    command
        .arg("__loop-worker")
        .arg("--interval-seconds")
        .arg(interval.to_string())
        .args(&worker_args)
        .current_dir(".")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .envs(proxy_env());
    command.process_group(0);
    let child = command.spawn()?;
    fs::write(loop_pid_path(), format!("{}\n", child.id()))?;
    write_loop_status("running", Some(child.id()), "loop worker started")?;
    append_event(
        "loop_started",
        "",
        "loop",
        "running",
        &format!("pid={}; interval_seconds={interval}", child.id()),
    )?;
    println!("Loop started pid {}.", child.id());
    println!("  interval_seconds: {interval}");
    println!(
        "  logs: {} | {}",
        loop_stdout_path().display(),
        loop_stderr_path().display()
    );
    Ok(())
}

fn loop_stop(args: &[String]) -> Result<()> {
    ensure_allowed_flags(
        args,
        &[
            "--force",
            "--request_id",
            "--request-id",
            "--stage",
            "--reason",
        ],
    )?;
    if let Some(request_id) =
        flag_value(args, "--request_id")?.or(flag_value(args, "--request-id")?)
    {
        let stage = flag_value(args, "--stage")?.unwrap_or_else(|| {
            loop_stop_default_stage(&request_id).unwrap_or_else(|_| "implementation".to_string())
        });
        let reason = flag_value(args, "--reason")?
            .unwrap_or_else(|| "stopped by sandrone loop stop".to_string());
        block_request_by_id(&request_id, &stage, &reason)?;
        append_event(
            "loop_request_stopped",
            &request_id,
            &stage,
            "blocked",
            &reason,
        )?;
        println!("Request {request_id} stopped.");
        println!("  status: blocked");
        println!("  stage: {stage}");
        println!("  reason: {reason}");
        return Ok(());
    }
    fs::create_dir_all(loop_state_dir())?;
    fs::write(loop_stop_path(), format!("requested_at={}\n", now_string()))?;
    let mut detail = "soft stop requested; running agents are not killed".to_string();
    if flag_present(args, "--force")
        && let Some(pid) = read_loop_pid()?
        && process_is_running(pid)
    {
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();
        detail =
            format!("force stop sent TERM to loop worker pid {pid}; running agents are not killed");
    }
    write_loop_status("stopping", read_loop_pid()?, &detail)?;
    append_event("loop_stop_requested", "", "loop", "stopping", &detail)?;
    println!("Loop stop requested.");
    println!("  {detail}");
    Ok(())
}

fn loop_stop_default_stage(request_id: &str) -> Result<String> {
    let requests = load_requests()?;
    let request = requests
        .iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    Ok(inferred_document_phase(request).as_str().to_string())
}

fn loop_restart(args: &[String]) -> Result<()> {
    ensure_allowed_flags(args, &["--request_id", "--request-id"])?;
    remove_file_if_exists_local(&loop_stop_path())?;
    let request_id = flag_value(args, "--request_id")?.or(flag_value(args, "--request-id")?);
    let resumed = if let Some(request_id) = request_id {
        resume_request(&["--request_id".to_string(), request_id])?;
        1
    } else {
        resume_all_blocked_requests()?
    };
    println!("Loop restart prepared {resumed} resumed request(s).");
    println!("  next: sandrone loop start");
    Ok(())
}

fn resume_all_blocked_requests() -> Result<usize> {
    let request_ids = load_requests()?
        .into_iter()
        .filter(|request| request.status == "blocked")
        .map(|request| request.request_id)
        .collect::<Vec<_>>();
    if request_ids.is_empty() {
        println!("No blocked request to resume.");
        return Ok(0);
    }
    for request_id in &request_ids {
        resume_request(&["--request_id".to_string(), request_id.clone()])?;
    }
    Ok(request_ids.len())
}

fn loop_status() -> Result<()> {
    fs::create_dir_all(loop_state_dir())?;
    let status = fs::read_to_string(loop_status_path()).unwrap_or_else(|_| {
        "{\n  \"status\": \"stopped\",\n  \"detail\": \"loop has not been started\"\n}\n"
            .to_string()
    });
    print!("{status}");
    if let Some(pid) = read_loop_pid()? {
        println!(
            "pid_running: {}",
            if process_is_running(pid) {
                "true"
            } else {
                "false"
            }
        );
    }
    Ok(())
}

pub(crate) fn loop_worker(args: &[String]) -> Result<()> {
    ensure_initialized()?;
    ensure_allowed_flags(
        args,
        &[
            "--interval-seconds",
            "--max-attempts",
            "--parallel-limit",
            "--parallel_limit",
        ],
    )?;
    let interval = parse_loop_interval(flag_value(args, "--interval-seconds")?)?;
    write_loop_status("running", Some(std::process::id()), "loop worker active")?;
    loop {
        if loop_stop_path().exists() {
            break;
        }
        append_event("loop_iteration_started", "", "loop", "running", "")?;
        let tick_args = loop_tick_args(args);
        match tick(&tick_args) {
            Ok(()) => {
                append_event("loop_iteration_completed", "", "loop", "running", "ok")?;
                write_loop_status(
                    "sleeping",
                    Some(std::process::id()),
                    "last iteration completed",
                )?;
            }
            Err(error) => {
                append_event(
                    "loop_iteration_failed",
                    "",
                    "loop",
                    "error",
                    &error.to_string(),
                )?;
                write_loop_status("error", Some(std::process::id()), &error.to_string())?;
            }
        }
        if loop_stop_path().exists() {
            break;
        }
        let wake_token = loop_wake_token();
        write_loop_status(
            "sleeping",
            Some(std::process::id()),
            &loop_sleep_detail(interval),
        )?;
        sleep_until_wake_or_timeout(interval, wake_token);
    }
    write_loop_status("stopped", Some(std::process::id()), "loop worker stopped")?;
    append_event("loop_stopped", "", "loop", "stopped", "")?;
    Ok(())
}

fn loop_tick_args(args: &[String]) -> Vec<String> {
    let mut tick_args = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--interval-seconds" => index += 2,
            flag => {
                tick_args.push(flag.to_string());
                if matches!(
                    flag,
                    "--interval-seconds"
                        | "--max-attempts"
                        | "--parallel-limit"
                        | "--parallel_limit"
                ) && index + 1 < args.len()
                {
                    tick_args.push(args[index + 1].clone());
                    index += 2;
                } else {
                    index += 1;
                }
            }
        }
    }
    tick_args
}

fn loop_sleep_detail(interval: u64) -> String {
    match loop_has_active_work() {
        Ok(true) => {
            format!("active request exists; waiting for wake signal or {interval}s fallback")
        }
        Ok(false) => format!("no active request; sleeping {interval}s"),
        Err(error) => format!("unable to inspect active work ({error}); sleeping {interval}s"),
    }
}

fn loop_has_active_work() -> Result<bool> {
    if load_loop_cohort()?.is_some() {
        return Ok(true);
    }
    Ok(load_requests()?.iter().any(loop_request_is_active))
}

fn loop_request_is_active(request: &Request) -> bool {
    match canonical_status(&request.status) {
        STATUS_FINISHED | STATUS_SLICE_FINISHED | "blocked" => false,
        STATUS_WAIT_UPDATE_PR | STATUS_WAIT_FINISH => true,
        status => !is_terminal_status(status),
    }
}

fn sleep_until_wake_or_timeout(seconds: u64, wake_token: Option<String>) {
    let timeout = Duration::from_secs(seconds.max(1));
    if loop_stop_path().exists() || loop_wake_token() != wake_token {
        return;
    }

    let state_dir = loop_state_dir();
    if fs::create_dir_all(&state_dir).is_err() {
        std::thread::sleep(timeout);
        return;
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |event| {
        let _ = tx.send(event);
    }) {
        Ok(watcher) => watcher,
        Err(_) => {
            std::thread::sleep(timeout);
            return;
        }
    };
    if watcher
        .watch(&state_dir, RecursiveMode::NonRecursive)
        .is_err()
    {
        std::thread::sleep(timeout);
        return;
    }

    let deadline = Instant::now() + timeout;
    loop {
        if loop_stop_path().exists() {
            break;
        }
        if loop_wake_token() != wake_token {
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(Ok(event)) => {
                if event.paths.is_empty()
                    || event
                        .paths
                        .iter()
                        .any(|path| path == &loop_wake_path() || path == &loop_stop_path())
                {
                    continue;
                }
            }
            Ok(Err(_)) => continue,
            Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

pub(crate) fn request_loop_wake(reason: &str) -> Result<()> {
    fs::create_dir_all(loop_state_dir())?;
    let sequence = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    fs::write(
        loop_wake_path(),
        format!(
            "sequence={sequence}\nwake_at={}\nreason={}\npid={}\n",
            now_string(),
            reason.replace('\n', " "),
            std::process::id(),
        ),
    )?;
    Ok(())
}

fn loop_wake_token() -> Option<String> {
    fs::read_to_string(loop_wake_path()).ok()
}

fn parse_loop_interval(value: Option<String>) -> Result<u64> {
    let interval = value
        .as_deref()
        .unwrap_or("900")
        .parse::<u64>()
        .map_err(|_| "--interval-seconds must be a positive integer")?;
    Ok(interval.max(1))
}

fn write_loop_status(status: &str, pid: Option<u32>, detail: &str) -> Result<()> {
    fs::create_dir_all(loop_state_dir())?;
    fs::write(
        loop_status_path(),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"status\": \"{}\",\n  \"pid\": \"{}\",\n  \"detail\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(status),
            pid.map(|pid| pid.to_string()).unwrap_or_default(),
            json_escape(detail),
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn read_loop_pid() -> Result<Option<u32>> {
    let path = loop_pid_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    Ok(content.trim().parse::<u32>().ok())
}

fn loop_state_dir() -> PathBuf {
    Path::new(".sandrone").join("state").join("loop")
}

fn loop_pid_path() -> PathBuf {
    loop_state_dir().join("pid")
}

fn loop_stop_path() -> PathBuf {
    loop_state_dir().join("stop")
}

fn loop_wake_path() -> PathBuf {
    loop_state_dir().join("wake")
}

fn loop_status_path() -> PathBuf {
    loop_state_dir().join("status.json")
}

fn loop_stdout_path() -> PathBuf {
    loop_state_dir().join("stdout.log")
}

fn loop_stderr_path() -> PathBuf {
    loop_state_dir().join("stderr.log")
}

fn remove_file_if_exists_local(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
