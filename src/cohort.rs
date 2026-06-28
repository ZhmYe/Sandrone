use super::*;

const COHORT_PATH: &str = ".sandrone/state/scheduler/cohort.json";
const COHORT_PROGRESS_PATH: &str = ".sandrone/state/scheduler/cohort-progress.json";
const COHORT_LAST_PATH: &str = ".sandrone/state/scheduler/last-cohort.json";
const COHORT_LAST_PROGRESS_PATH: &str = ".sandrone/state/scheduler/last-cohort-progress.json";
const COHORT_HISTORY_PATH: &str = ".sandrone/state/scheduler/cohort-history.ndjson";

#[derive(Clone, Debug)]
pub(crate) struct LoopCohort {
    pub(crate) cohort_id: String,
    pub(crate) request_ids: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

pub(crate) fn load_loop_cohort() -> Result<Option<LoopCohort>> {
    if !Path::new(COHORT_PATH).exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(COHORT_PATH)?;
    let cohort_id = json_value(&content, "cohort_id").unwrap_or_default();
    let request_ids = json_string_list_field(&content, "request_ids");
    if cohort_id.trim().is_empty() || request_ids.is_empty() {
        remove_file_if_exists(Path::new(COHORT_PATH))?;
        return Ok(None);
    }
    Ok(Some(LoopCohort {
        cohort_id,
        request_ids,
        created_at: json_value(&content, "created_at").unwrap_or_default(),
        updated_at: json_value(&content, "updated_at").unwrap_or_default(),
    }))
}

pub(crate) fn create_loop_cohort(request_ids: &[String]) -> Result<Option<LoopCohort>> {
    let mut parent_ids = Vec::new();
    for request_id in request_ids {
        if !parent_ids.contains(request_id) {
            parent_ids.push(request_id.clone());
        }
    }
    if parent_ids.is_empty() {
        return Ok(None);
    }
    let now = now_string();
    let cohort = LoopCohort {
        cohort_id: format!("cohort-{now}"),
        request_ids: parent_ids,
        created_at: now.clone(),
        updated_at: now,
    };
    write_loop_cohort(&cohort)?;
    refresh_loop_cohort_progress(&load_requests()?)?;
    append_event(
        "loop_cohort_started",
        "",
        "loop",
        "active",
        &format!(
            "cohort_id={}; requests={}",
            cohort.cohort_id,
            cohort.request_ids.join(",")
        ),
    )?;
    let _ = request_loop_wake("loop cohort started");
    Ok(Some(cohort))
}

pub(crate) fn reconcile_loop_cohort(requests: &[Request]) -> Result<Option<LoopCohort>> {
    let Some(cohort) = load_loop_cohort()? else {
        return Ok(None);
    };
    if cohort_finished(&cohort, requests)? {
        complete_loop_cohort(&cohort, requests)?;
        Ok(None)
    } else {
        Ok(Some(cohort))
    }
}

pub(crate) fn complete_loop_cohort_if_done(requests: &[Request]) -> Result<bool> {
    let Some(cohort) = load_loop_cohort()? else {
        return Ok(false);
    };
    if !cohort_finished(&cohort, requests)? {
        return Ok(false);
    }
    complete_loop_cohort(&cohort, requests)?;
    Ok(true)
}

pub(crate) fn select_new_cohort_candidates(requests: &[Request]) -> Vec<String> {
    requests
        .iter()
        .filter(|request| is_parent_request(request))
        .filter(|request| !is_agent_running_status(&request.status))
        .filter(|request| !is_terminal_status(&request.status))
        .map(|request| request.request_id.clone())
        .collect()
}

pub(crate) fn select_cohort_tick_requests(
    requests: &[Request],
    cohort: &LoopCohort,
) -> Result<Vec<String>> {
    let mut selected = Vec::new();
    for request in requests {
        if !request_belongs_to_cohort(request, cohort) {
            continue;
        }
        if is_agent_running_status(&request.status) || is_terminal_status(&request.status) {
            continue;
        }
        if is_slice_request(request) && !slice_dependencies_ready(request, requests)? {
            continue;
        }
        if !selected.contains(&request.request_id) {
            selected.push(request.request_id.clone());
        }
    }
    Ok(selected)
}

pub(crate) fn request_belongs_to_cohort(request: &Request, cohort: &LoopCohort) -> bool {
    request_belongs_to_parent_ids(request, &cohort.request_ids)
}

pub(crate) fn request_belongs_to_parent_ids(request: &Request, parent_ids: &[String]) -> bool {
    parent_ids.iter().any(|id| id == &request.request_id)
        || slice_parent_id(request)
            .map(|parent_id| parent_ids.iter().any(|id| id == &parent_id))
            .unwrap_or(false)
}

fn cohort_finished(cohort: &LoopCohort, requests: &[Request]) -> Result<bool> {
    for parent_id in &cohort.request_ids {
        let Some(parent) = requests
            .iter()
            .find(|request| request.request_id == *parent_id)
        else {
            continue;
        };
        if parent_cohort_done(parent, requests)? {
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

fn parent_cohort_done(parent: &Request, requests: &[Request]) -> Result<bool> {
    if matches!(
        canonical_status(&parent.status),
        STATUS_FINISHED | "blocked"
    ) {
        return Ok(true);
    }
    let slices = requests
        .iter()
        .filter(|request| slice_parent_id(request).as_deref() == Some(&parent.request_id))
        .collect::<Vec<_>>();
    if !slices.is_empty() {
        if slices
            .iter()
            .any(|request| canonical_status(&request.status) == "blocked")
        {
            return Ok(true);
        }
        return Ok(slices.iter().all(|request| slice_done(request)));
    }
    Ok(false)
}

fn complete_loop_cohort(cohort: &LoopCohort, requests: &[Request]) -> Result<()> {
    let now = now_string();
    let mut completed = cohort.clone();
    completed.updated_at = now.clone();
    let status_lines = cohort
        .request_ids
        .iter()
        .map(|request_id| {
            let status = requests
                .iter()
                .find(|request| request.request_id == *request_id)
                .map(|request| canonical_status(&request.status).to_string())
                .unwrap_or_else(|| "missing".to_string());
            format!(
                "    {{ \"request_id\": \"{}\", \"status\": \"{}\" }}",
                json_escape(request_id),
                json_escape(&status)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let content = format!(
        "{{\n  \"schema_version\": 1,\n  \"cohort_id\": \"{}\",\n  \"status\": \"completed\",\n  \"created_at\": \"{}\",\n  \"completed_at\": \"{}\",\n  \"request_ids\": [{}],\n  \"requests\": [\n{}\n  ]\n}}\n",
        json_escape(&completed.cohort_id),
        json_escape(&completed.created_at),
        json_escape(&now),
        json_string_array(&completed.request_ids),
        status_lines,
    );
    fs::write(COHORT_LAST_PATH, &content)?;
    append_line(COHORT_HISTORY_PATH, content.trim_end())?;
    write_loop_cohort_progress(
        cohort,
        "completed",
        requests,
        Path::new(COHORT_LAST_PROGRESS_PATH),
    )?;
    remove_file_if_exists(Path::new(COHORT_PATH))?;
    remove_file_if_exists(Path::new(COHORT_PROGRESS_PATH))?;
    append_event(
        "loop_cohort_completed",
        "",
        "loop",
        "completed",
        &format!(
            "cohort_id={}; requests={}",
            cohort.cohort_id,
            cohort.request_ids.join(",")
        ),
    )?;
    let _ = request_loop_wake("loop cohort completed");
    Ok(())
}

pub(crate) fn refresh_loop_cohort_progress(requests: &[Request]) -> Result<()> {
    let Some(cohort) = load_loop_cohort()? else {
        remove_file_if_exists(Path::new(COHORT_PROGRESS_PATH))?;
        return Ok(());
    };
    write_loop_cohort_progress(&cohort, "active", requests, Path::new(COHORT_PROGRESS_PATH))
}

fn write_loop_cohort_progress(
    cohort: &LoopCohort,
    status: &str,
    requests: &[Request],
    path: &Path,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let parent_lines = cohort
        .request_ids
        .iter()
        .map(|request_id| {
            let Some(request) = requests
                .iter()
                .find(|candidate| candidate.request_id == *request_id)
            else {
                return format!(
                    "    {{ \"request_id\": \"{}\", \"status\": \"missing\", \"done\": false }}",
                    json_escape(request_id)
                );
            };
            let done = parent_cohort_done(request, requests).unwrap_or(false);
            format!(
                "    {{ \"request_id\": \"{}\", \"title\": \"{}\", \"status\": \"{}\", \"done\": {} }}",
                json_escape(&request.request_id),
                json_escape(&request.title),
                json_escape(canonical_status(&request.status)),
                done,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let slice_lines = requests
        .iter()
        .filter(|request| request_belongs_to_cohort(request, cohort) && is_slice_request(request))
        .map(|request| {
            format!(
                "    {{ \"request_id\": \"{}\", \"parent_id\": \"{}\", \"title\": \"{}\", \"status\": \"{}\", \"done\": {} }}",
                json_escape(&request.request_id),
                json_escape(&slice_parent_id(request).unwrap_or_default()),
                json_escape(&request.title),
                json_escape(canonical_status(&request.status)),
                slice_done(request),
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    fs::write(
        path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"cohort_id\": \"{}\",\n  \"status\": \"{}\",\n  \"created_at\": \"{}\",\n  \"updated_at\": \"{}\",\n  \"request_ids\": [{}],\n  \"requests\": [\n{}\n  ],\n  \"slices\": [\n{}\n  ]\n}}\n",
            json_escape(&cohort.cohort_id),
            json_escape(status),
            json_escape(&cohort.created_at),
            json_escape(&now_string()),
            json_string_array(&cohort.request_ids),
            parent_lines,
            slice_lines,
        ),
    )?;
    Ok(())
}

fn write_loop_cohort(cohort: &LoopCohort) -> Result<()> {
    if let Some(parent) = Path::new(COHORT_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        COHORT_PATH,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"cohort_id\": \"{}\",\n  \"status\": \"active\",\n  \"created_at\": \"{}\",\n  \"updated_at\": \"{}\",\n  \"request_ids\": [{}]\n}}\n",
            json_escape(&cohort.cohort_id),
            json_escape(&cohort.created_at),
            json_escape(&cohort.updated_at),
            json_string_array(&cohort.request_ids),
        ),
    )?;
    Ok(())
}

fn json_string_list_field(content: &str, key: &str) -> Vec<String> {
    let Some(array) = json_array_content(content, key) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut current = String::new();
    for ch in array.chars() {
        if !in_string {
            if ch == '"' {
                in_string = true;
                current.clear();
            }
            continue;
        }
        if escaped {
            match ch {
                'n' => current.push('\n'),
                'r' => current.push('\r'),
                't' => current.push('\t'),
                '"' => current.push('"'),
                '\\' => current.push('\\'),
                other => current.push(other),
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            in_string = false;
            if !current.trim().is_empty() {
                values.push(current.clone());
            }
        } else {
            current.push(ch);
        }
    }
    values
}

fn json_string_array(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn append_line(path: &str, line: &str) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    use std::io::Write;
    writeln!(file, "{line}")?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
