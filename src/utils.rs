use super::*;
use std::io::Write;

pub(crate) fn proxy_env() -> Vec<(&'static str, String)> {
    ["https_proxy", "http_proxy", "all_proxy"]
        .iter()
        .filter_map(|key| env::var(key).ok().map(|value| (*key, value)))
        .collect()
}

pub(crate) fn today() -> String {
    if let Ok(output) = Command::new("date").arg("+%F").output()
        && output.status.success()
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    "1970-01-01".to_string()
}

pub(crate) fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

pub(crate) fn process_is_running(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub(crate) fn fallback_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

pub(crate) fn absolute_path_string(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    if path.is_absolute() {
        return path.to_string_lossy().to_string();
    }
    env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

pub(crate) fn absolute_path_string_or_empty(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    if path.as_os_str().is_empty() {
        String::new()
    } else {
        absolute_path_string(path)
    }
}

pub(crate) fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(crate) fn file_sha256(path: &Path) -> Result<String> {
    for (program, args) in [
        ("shasum", vec!["-a", "256"]),
        ("sha256sum", Vec::<&str>::new()),
    ] {
        let output = Command::new(program).args(args).arg(path).output();
        let Ok(output) = output else {
            continue;
        };
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            if let Some(hash) = stdout.split_whitespace().next()
                && !hash.trim().is_empty()
            {
                return Ok(hash.to_string());
            }
        }
    }
    Err("unable to compute sha256: neither shasum nor sha256sum succeeded".into())
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> Result<String> {
    for (program, args) in [
        ("shasum", vec!["-a", "256"]),
        ("sha256sum", Vec::<&str>::new()),
    ] {
        let spawn_result = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let Ok(mut child) = spawn_result else {
            continue;
        };
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(bytes)?;
        }
        let output = child.wait_with_output()?;
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            if let Some(hash) = stdout.split_whitespace().next()
                && !hash.trim().is_empty()
            {
                return Ok(hash.to_string());
            }
        }
    }
    Err("unable to compute sha256: neither shasum nor sha256sum succeeded".into())
}

pub(crate) fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

pub(crate) fn json_value(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let mut value = after_key[colon_index + 1..].trim_start().chars();
    if value.next()? != '"' {
        return None;
    }
    let mut escaped = false;
    let mut out = String::new();
    for ch in value {
        if escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

pub(crate) fn json_bool(content: &str, key: &str) -> Option<bool> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let value = after_key[colon_index + 1..].trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub(crate) fn json_number(content: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let value = after_key[colon_index + 1..].trim_start();
    let digits = value
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    digits.parse::<u32>().ok()
}

pub(crate) fn json_number_usize(content: &str, key: &str) -> Option<usize> {
    json_number(content, key).map(|value| value as usize)
}

pub(crate) fn json_usize_map(content: &str, key: &str) -> BTreeMap<String, usize> {
    let Some(inner) = json_object_content(content, key) else {
        return BTreeMap::new();
    };
    let mut values = BTreeMap::new();
    for item in inner.split(',') {
        let Some((raw_key, raw_value)) = item.split_once(':') else {
            continue;
        };
        let key = raw_key.trim().trim_matches('"').to_string();
        let value_text = raw_value.trim();
        if key.is_empty() {
            continue;
        }
        if let Ok(value) = value_text.parse::<usize>() {
            values.insert(key, value);
        }
    }
    values
}

fn json_object_content(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let after_colon = after_key[colon_index + 1..].trim_start();
    let rest = after_colon.strip_prefix('{')?;
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut inner = String::new();
    for ch in rest.chars() {
        if escaped {
            inner.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            inner.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            inner.push(ch);
            continue;
        }
        if !in_string {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(inner);
                }
            }
        }
        inner.push(ch);
    }
    None
}

pub(crate) fn review_findings(content: &str, severity: &str) -> Vec<ReviewFinding> {
    let Some(array) = json_array_content(content, severity) else {
        return Vec::new();
    };
    json_objects_in_array(&array)
        .into_iter()
        .map(|object| ReviewFinding {
            title: json_value(&object, "title").unwrap_or_else(|| "未提供标题".to_string()),
            evidence: json_value(&object, "evidence").unwrap_or_else(|| "未提供证据".to_string()),
            impact: json_value(&object, "impact").unwrap_or_else(|| "未提供影响".to_string()),
            required_fix: json_value(&object, "required_fix")
                .unwrap_or_else(|| "未提供必要修复".to_string()),
            suggested_change: json_value(&object, "suggested_change")
                .unwrap_or_else(|| "未提供修改建议".to_string()),
            verification: json_value(&object, "verification")
                .unwrap_or_else(|| "未提供验证方式".to_string()),
        })
        .collect()
}

pub(crate) fn review_has_blocking_findings(content: &str) -> bool {
    json_array_non_empty(content, "critical")
        || json_array_non_empty(content, "high")
        || content.contains("\"severity\":\"critical\"")
        || content.contains("\"severity\": \"critical\"")
        || content.contains("\"severity\":\"high\"")
        || content.contains("\"severity\": \"high\"")
}

pub(crate) fn json_array_content(content: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let key_index = content.find(&pattern)?;
    let after_key = &content[key_index + pattern.len()..];
    let colon_index = after_key.find(':')?;
    let after_colon = after_key[colon_index + 1..].trim_start();
    let rest = after_colon.strip_prefix('[')?;
    let mut depth = 1usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut inner = String::new();
    for ch in rest.chars() {
        if escaped {
            inner.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            inner.push(ch);
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            inner.push(ch);
            continue;
        }
        if !in_string {
            if ch == '[' {
                depth += 1;
            } else if ch == ']' {
                depth -= 1;
                if depth == 0 {
                    return Some(inner);
                }
            }
        }
        inner.push(ch);
    }
    None
}

pub(crate) fn json_objects_in_array(array: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut started = false;
    for ch in array.chars() {
        if escaped {
            if started {
                current.push(ch);
            }
            escaped = false;
            continue;
        }
        if ch == '\\' {
            if started {
                current.push(ch);
            }
            escaped = true;
            continue;
        }
        if ch == '"' {
            if started {
                current.push(ch);
            }
            in_string = !in_string;
            continue;
        }
        if !in_string {
            if ch == '{' {
                started = true;
                depth += 1;
                current.push(ch);
                continue;
            }
            if ch == '}' && started {
                current.push(ch);
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    objects.push(current.clone());
                    current.clear();
                    started = false;
                }
                continue;
            }
        }
        if started {
            current.push(ch);
        }
    }
    objects
}

fn json_array_non_empty(content: &str, key: &str) -> bool {
    json_array_content(content, key)
        .map(|inner| !inner.trim().is_empty())
        .unwrap_or(false)
}

pub(crate) fn markdown_inline(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn json_bool_literal(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

pub(crate) fn ensure_trailing_newline(value: &str) -> String {
    if value.ends_with('\n') {
        value.to_string()
    } else {
        format!("{value}\n")
    }
}

pub(crate) fn indent_json_object(content: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    content
        .trim()
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

pub(crate) fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('t') => out.push('\t'),
                Some('n') => out.push('\n'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}
