use super::*;

fn codex_auto_dev_home() -> PathBuf {
    if let Ok(value) = env::var("CODEX_AUTO_DEV_HOME")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value);
    }
    if let Ok(value) = env::var("HOME")
        && !value.trim().is_empty()
    {
        return PathBuf::from(value).join(".codex-auto-dev");
    }
    PathBuf::from(".codex-auto-dev-global")
}

pub(crate) fn global_workspaces_path() -> PathBuf {
    codex_auto_dev_home().join(GLOBAL_WORKSPACES_FILE)
}

pub(crate) fn refresh_current_workspace_registry_or_warn(last_status: &str) {
    if let Err(error) = upsert_current_workspace_registry(last_status) {
        eprintln!(
            "workspace registry warning: could not update {}: {error}",
            global_workspaces_path().display()
        );
    }
}

fn upsert_current_workspace_registry(last_status: &str) -> Result<()> {
    let record = current_workspace_record(last_status)?;
    let mut records = load_workspace_records()?;
    if let Some(existing) = records.iter_mut().find(|existing| {
        existing.key == record.key || existing.workspace_path == record.workspace_path
    }) {
        *existing = record;
    } else {
        records.push(record);
    }
    records.sort_by(|left, right| {
        left.repo_name
            .cmp(&right.repo_name)
            .then_with(|| left.workspace_path.cmp(&right.workspace_path))
    });
    save_workspace_records(&records)
}

fn current_workspace_record(last_status: &str) -> Result<WorkspaceRecord> {
    let config = load_config()?;
    let requests = load_requests()?;
    let status_counts = request_status_counts(&requests);
    let workspace_path = absolute_path_string(".");
    Ok(WorkspaceRecord {
        key: workspace_path.clone(),
        repo_name: config.repo_name,
        git_url: config.git_url,
        workspace_path,
        target_repo: absolute_path_string(DEV_REPO),
        last_status: last_status.to_string(),
        request_count: requests.len(),
        status_counts,
        updated_at: now_string(),
    })
}

fn request_status_counts(requests: &[Request]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for request in requests {
        *counts.entry(request.status.clone()).or_default() += 1;
    }
    counts
}

fn load_workspace_records() -> Result<Vec<WorkspaceRecord>> {
    let path = global_workspaces_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    let mut records = Vec::new();
    for line in content.lines() {
        let line = line.trim().trim_end_matches(',');
        if !line.starts_with('{') || !line.contains("\"workspace_path\"") {
            continue;
        }
        let workspace_path = json_value(line, "workspace_path").unwrap_or_default();
        if workspace_path.trim().is_empty() {
            continue;
        }
        records.push(WorkspaceRecord {
            key: json_value(line, "key").unwrap_or_else(|| workspace_path.clone()),
            repo_name: json_value(line, "repo_name").unwrap_or_else(|| "unknown".to_string()),
            git_url: json_value(line, "git_url").unwrap_or_default(),
            workspace_path,
            target_repo: json_value(line, "target_repo").unwrap_or_default(),
            last_status: json_value(line, "last_status").unwrap_or_else(|| "unknown".to_string()),
            request_count: json_number_usize(line, "request_count").unwrap_or(0),
            status_counts: json_usize_map(line, "status_counts"),
            updated_at: json_value(line, "updated_at").unwrap_or_default(),
        });
    }
    Ok(records)
}

fn save_workspace_records(records: &[WorkspaceRecord]) -> Result<()> {
    let path = global_workspaces_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut content = String::from("{\n  \"schema_version\": 1,\n  \"workspaces\": [\n");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            content.push_str(",\n");
        }
        content.push_str(&format!(
            "    {{ \"key\": \"{}\", \"repo_name\": \"{}\", \"git_url\": \"{}\", \"workspace_path\": \"{}\", \"target_repo\": \"{}\", \"last_status\": \"{}\", \"request_count\": {}, \"status_counts\": {}, \"updated_at\": \"{}\" }}",
            json_escape(&record.key),
            json_escape(&record.repo_name),
            json_escape(&record.git_url),
            json_escape(&record.workspace_path),
            json_escape(&record.target_repo),
            json_escape(&record.last_status),
            record.request_count,
            render_usize_map_json(&record.status_counts),
            json_escape(&record.updated_at),
        ));
    }
    content.push_str("\n  ]\n}\n");
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn refresh_registered_workspaces() -> Result<Vec<WorkspaceRecord>> {
    let records = load_workspace_records()?;
    let mut refreshed = Vec::new();
    for record in records {
        refreshed.push(refresh_workspace_record(&record));
    }
    save_workspace_records(&refreshed)?;
    Ok(refreshed)
}

fn refresh_workspace_record(record: &WorkspaceRecord) -> WorkspaceRecord {
    let workspace_path = Path::new(&record.workspace_path);
    if !workspace_path.join(CONFIG_PATH).exists() {
        let mut missing = record.clone();
        missing.last_status = "missing".to_string();
        missing.updated_at = now_string();
        return missing;
    }
    match with_current_dir(workspace_path, || {
        sync_all_requests_from_status_json()?;
        current_workspace_record("ready")
    }) {
        Ok(record) => record,
        Err(error) => {
            let mut failed = record.clone();
            failed.last_status =
                format!("error: {}", review_diagnostic_excerpt(&error.to_string()));
            failed.updated_at = now_string();
            failed
        }
    }
}

pub(crate) fn with_current_dir<T>(path: &Path, action: impl FnOnce() -> Result<T>) -> Result<T> {
    let previous = env::current_dir()?;
    env::set_current_dir(path)?;
    let result = action();
    let restore_result = env::set_current_dir(previous);
    match (result, restore_result) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Err(error), Err(_)) => Err(error),
    }
}

pub(crate) fn render_usize_map_json(values: &BTreeMap<String, usize>) -> String {
    let mut rendered = String::from("{");
    for (index, (key, value)) in values.iter().enumerate() {
        if index > 0 {
            rendered.push_str(", ");
        }
        rendered.push_str(&format!("\"{}\": {}", json_escape(key), value));
    }
    rendered.push('}');
    rendered
}
