use super::*;

pub(crate) const SLICE_SOURCE: &str = "slice";
pub(crate) const STATUS_SLICES_READY: &str = "slices-ready";
pub(crate) const STATUS_SLICES_RUNNING: &str = "slices-running";
pub(crate) const STATUS_SLICE_FINISHED: &str = "slice-finished";

#[derive(Clone, Debug)]
pub(crate) struct SliceDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) summary: String,
    pub(crate) depends_on: Vec<String>,
}

pub(crate) fn is_slice_request(request: &Request) -> bool {
    request.source == SLICE_SOURCE || request.external_id.starts_with("slice:")
}

pub(crate) fn is_parent_request(request: &Request) -> bool {
    !is_slice_request(request)
}

pub(crate) fn slice_parent_id(request: &Request) -> Option<String> {
    if !is_slice_request(request) {
        return None;
    }
    let mut parts = request.external_id.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("slice"), Some(parent_id), Some(_slice_id)) => Some(parent_id.to_string()),
        _ => request
            .request_id
            .split_once("-S")
            .map(|(parent, _)| parent.to_string()),
    }
}

pub(crate) fn slice_id_from_request(request: &Request) -> Option<String> {
    if !is_slice_request(request) {
        return None;
    }
    let mut parts = request.external_id.split(':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some("slice"), Some(_parent_id), Some(slice_id)) => Some(slice_id.to_string()),
        _ => request
            .request_id
            .split_once('-')
            .map(|(_, slice_id)| slice_id.to_string()),
    }
}

pub(crate) fn load_slice_definitions(parent: &Request) -> Result<Vec<SliceDefinition>> {
    let decomposition_path = Path::new(&parent.change_path).join("decomposition.json");
    if !decomposition_path.exists() {
        return Ok(default_slice_definitions(parent));
    }
    let content = fs::read_to_string(decomposition_path)?;
    let Some(slices) = json_array_content(&content, "slices") else {
        return Ok(default_slice_definitions(parent));
    };
    let mut definitions = Vec::new();
    for object in json_objects_in_array(&slices) {
        let id = json_value(&object, "id")
            .map(|value| normalize_slice_id(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("S{:02}", definitions.len() + 1));
        let name = json_value(&object, "name")
            .map(|value| normalize_slice_name(&value))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("slice-{:02}", definitions.len() + 1));
        let summary = json_value(&object, "summary").unwrap_or_else(|| parent.body.clone());
        let depends_on = json_string_array(&object, "depends_on")
            .into_iter()
            .map(|value| normalize_slice_id(&value))
            .filter(|value| !value.is_empty())
            .collect();
        definitions.push(SliceDefinition {
            id,
            name,
            summary,
            depends_on,
        });
    }
    if definitions.is_empty() {
        Ok(default_slice_definitions(parent))
    } else {
        Ok(definitions)
    }
}

pub(crate) fn materialize_slices_for_parent(
    requests: &mut Vec<Request>,
    parent_index: usize,
    preflight: &PlanPreflight,
) -> Result<bool> {
    let parent = requests[parent_index].clone();
    ensure_gate_approved(&parent, "decomposition")?;
    let definitions = load_slice_definitions(&parent)?;
    let mut created = 0usize;
    for definition in definitions {
        let external_id = slice_external_id(&parent.request_id, &definition.id);
        if requests
            .iter()
            .any(|request| request.external_id == external_id)
        {
            continue;
        }
        let now = now_string();
        let request_id = slice_request_id(&parent.request_id, &definition.id);
        let change_name = format!(
            "{}-{}-{}",
            parent.change_name,
            definition.id.to_lowercase(),
            definition.name
        );
        let change_path = Path::new(&parent.change_path)
            .join("slices")
            .join(&definition.id)
            .to_string_lossy()
            .to_string();
        let slice = Request {
            request_id: request_id.clone(),
            external_id,
            source: SLICE_SOURCE.to_string(),
            title: format!("{} / {}", parent.title, definition.name),
            body: render_slice_body(&parent, &definition),
            url: parent.url.clone(),
            status: "planning".to_string(),
            change_name,
            change_path,
            branch: String::new(),
            worktree_path: String::new(),
            created_at: now.clone(),
            updated_at: now,
        };
        fs::create_dir_all(&slice.change_path)?;
        write_slice_meta(&slice, &parent, &definition)?;
        generate_plan_packet(&slice, preflight)?;
        sync_obsidian_request_note(&slice)?;
        requests.push(slice.clone());
        append_event(
            "slice_materialized",
            &slice.request_id,
            "decomposition",
            &slice.status,
            &format!(
                "parent={}; slice={}; depends_on={}",
                parent.request_id,
                definition.id,
                definition.depends_on.join(",")
            ),
        )?;
        created += 1;
    }

    let mut changed = created > 0;
    let parent_status = if created > 0 {
        STATUS_SLICES_READY
    } else {
        STATUS_SLICES_RUNNING
    };
    let mut updated_parent = parent.clone();
    if updated_parent.status != parent_status {
        updated_parent.status = parent_status.to_string();
        updated_parent.updated_at = now_string();
        requests[parent_index] = updated_parent.clone();
        write_status_json(
            &updated_parent,
            "decomposition",
            parent_status,
            "decomposition approved; slice DAG materialized",
        )?;
        append_event(
            "slices_ready",
            &updated_parent.request_id,
            "decomposition",
            parent_status,
            &format!("created={created}"),
        )?;
        upsert_session_for_request(&updated_parent, "decomposition", parent_status)?;
        changed = true;
    }
    if created > 0 {
        sync_obsidian_request_note(&requests[parent_index])?;
        sync_obsidian_project_note(requests)?;
    }
    Ok(changed)
}

pub(crate) fn slice_dependencies_ready(slice: &Request, requests: &[Request]) -> Result<bool> {
    let Some(parent_id) = slice_parent_id(slice) else {
        return Ok(true);
    };
    let Some(parent) = requests
        .iter()
        .find(|request| request.request_id == parent_id)
    else {
        return Ok(false);
    };
    let Some(slice_id) = slice_id_from_request(slice) else {
        return Ok(true);
    };
    let definitions = load_slice_definitions(parent)?;
    let Some(definition) = definitions
        .iter()
        .find(|definition| definition.id == slice_id)
    else {
        return Ok(true);
    };
    for dependency in &definition.depends_on {
        let dependency_request_id = slice_request_id(&parent_id, dependency);
        let Some(dependency_request) = requests
            .iter()
            .find(|request| request.request_id == dependency_request_id)
        else {
            return Ok(false);
        };
        if !slice_done(dependency_request) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn parent_slices_finished(parent: &Request, requests: &[Request]) -> Result<bool> {
    let definitions = load_slice_definitions(parent)?;
    if definitions.is_empty() {
        return Ok(false);
    }
    for definition in definitions {
        let request_id = slice_request_id(&parent.request_id, &definition.id);
        let Some(slice) = requests
            .iter()
            .find(|request| request.request_id == request_id)
        else {
            return Ok(false);
        };
        if !slice_done(slice) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn mark_slice_finished_by_id(request_id: &str, reason: &str) -> Result<()> {
    let mut requests = load_requests()?;
    let index = find_request_index(&requests, request_id)
        .ok_or_else(|| format!("unknown request_id: {request_id}"))?;
    let mut request = requests[index].clone();
    if !is_slice_request(&request) {
        return Err(format!("{request_id} is not a slice request").into());
    }
    ensure_gate_approved(&request, "change-doc")?;
    request.status = STATUS_SLICE_FINISHED.to_string();
    request.updated_at = now_string();
    requests[index] = request.clone();
    save_requests(&requests)?;
    write_status_json(&request, "slice", STATUS_SLICE_FINISHED, reason)?;
    append_event(
        "slice_finished",
        &request.request_id,
        "slice",
        STATUS_SLICE_FINISHED,
        reason,
    )?;
    upsert_session_for_request(&request, "implementation", STATUS_SLICE_FINISHED)?;
    if let Some(parent_id) = slice_parent_id(&request) {
        let _ = refresh_parent_slice_status(&parent_id);
    }
    Ok(())
}

pub(crate) fn refresh_parent_slice_status(parent_id: &str) -> Result<bool> {
    let mut requests = load_requests()?;
    let Some(index) = find_request_index(&requests, parent_id) else {
        return Ok(false);
    };
    let mut parent = requests[index].clone();
    if is_slice_request(&parent) || parent.change_path.trim().is_empty() {
        return Ok(false);
    }
    if is_terminal_status(&parent.status)
        || canonical_status(&parent.status) == STATUS_WAIT_UPDATE_PR
    {
        return Ok(false);
    }
    if parent_slices_finished(&parent, &requests)? {
        parent.status = STATUS_WAIT_UPDATE_PR.to_string();
        parent.updated_at = now_string();
        requests[index] = parent.clone();
        save_requests(&requests)?;
        write_status_json(
            &parent,
            "delivery",
            STATUS_WAIT_UPDATE_PR,
            "all slices finished; waiting for aggregate PR creation or update",
        )?;
        append_event(
            "all_slices_finished",
            &parent.request_id,
            "delivery",
            STATUS_WAIT_UPDATE_PR,
            "all slices reached slice-finished",
        )?;
        upsert_session_for_request(&parent, "implementation", STATUS_WAIT_UPDATE_PR)?;
        Ok(true)
    } else {
        let next_status = STATUS_SLICES_RUNNING.to_string();
        if parent.status != next_status {
            parent.status = next_status.clone();
            parent.updated_at = now_string();
            requests[index] = parent.clone();
            save_requests(&requests)?;
            write_status_json(
                &parent,
                "decomposition",
                &next_status,
                "waiting for slice DAG completion",
            )?;
            append_event(
                "slices_running",
                &parent.request_id,
                "decomposition",
                &next_status,
                "slice DAG is not complete yet",
            )?;
            return Ok(true);
        }
        Ok(false)
    }
}

pub(crate) fn slice_done(request: &Request) -> bool {
    matches!(
        canonical_status(&request.status),
        STATUS_SLICE_FINISHED | STATUS_WAIT_UPDATE_PR | STATUS_WAIT_FINISH | STATUS_FINISHED
    )
}

pub(crate) fn slice_request_id(parent_id: &str, slice_id: &str) -> String {
    format!("{}-{}", parent_id, normalize_slice_id(slice_id))
}

pub(crate) fn slice_external_id(parent_id: &str, slice_id: &str) -> String {
    format!("slice:{}:{}", parent_id, normalize_slice_id(slice_id))
}

fn default_slice_definitions(parent: &Request) -> Vec<SliceDefinition> {
    vec![SliceDefinition {
        id: "S01".to_string(),
        name: "main".to_string(),
        summary: parent.body.clone(),
        depends_on: Vec::new(),
    }]
}

fn render_slice_body(parent: &Request, definition: &SliceDefinition) -> String {
    format!(
        "父需求: {}\n父需求标题: {}\n父需求来源: {} ({})\n父需求文档: 父 request.md、decomposition.md、decomposition.json、dag.json\n\n当前 slice: {}\nSlice 名称: {}\nSlice 摘要:\n{}\n\n依赖 slice: {}\n\n调度约束:\n- 本 slice 必须只实现上述 slice 范围。\n- 必须读取父需求的需求记录、拆解文档、DAG、需求覆盖说明和已完成依赖 slice 文档。\n- 必须保留父需求的全局不变量，不得扩大范围或破坏已完成 slice 的语义。\n",
        parent.request_id,
        parent.title,
        parent.source,
        fallback_empty(&parent.url, "n/a"),
        definition.id,
        definition.name,
        fallback_empty(&definition.summary, "无"),
        if definition.depends_on.is_empty() {
            "无".to_string()
        } else {
            definition.depends_on.join(", ")
        }
    )
}

fn write_slice_meta(slice: &Request, parent: &Request, definition: &SliceDefinition) -> Result<()> {
    fs::write(
        Path::new(&slice.change_path).join("slice.json"),
        format!(
            "{{\n  \"schema_version\": 1,\n  \"request_id\": \"{}\",\n  \"parent_request_id\": \"{}\",\n  \"slice_id\": \"{}\",\n  \"name\": \"{}\",\n  \"depends_on\": [{}],\n  \"parent_change_path\": \"{}\",\n  \"updated_at\": \"{}\"\n}}\n",
            json_escape(&slice.request_id),
            json_escape(&parent.request_id),
            json_escape(&definition.id),
            json_escape(&definition.name),
            definition
                .depends_on
                .iter()
                .map(|dependency| format!("\"{}\"", json_escape(dependency)))
                .collect::<Vec<_>>()
                .join(", "),
            json_escape(&parent.change_path),
            json_escape(&now_string()),
        ),
    )?;
    Ok(())
}

fn json_string_array(content: &str, key: &str) -> Vec<String> {
    let Some(array) = json_array_content(content, key) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for ch in array.chars() {
        if escaped {
            current.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
            continue;
        }
        if in_string && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            if in_string {
                values.push(current.clone());
                current.clear();
            }
            in_string = !in_string;
            continue;
        }
        if in_string {
            current.push(ch);
        }
    }
    values
}

fn normalize_slice_id(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut normalized = trimmed
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    if normalized.chars().all(|ch| ch.is_ascii_digit()) {
        normalized = format!("S{:02}", normalized.parse::<u32>().unwrap_or(1));
    }
    normalized
}

fn normalize_slice_name(value: &str) -> String {
    let slug = slugify(value);
    if slug.is_empty() {
        "main".to_string()
    } else {
        slug
    }
}
