use super::*;

pub(crate) fn doctor(args: &[String]) -> Result<()> {
    ensure_allowed_flags(args, &[])?;
    let codegraph_bin = codegraph_bin();
    let checks = vec![
        doctor_check(
            "Workspace",
            Path::new(CONFIG_PATH).exists() && Path::new(STATE_PATH).exists(),
            "workspace metadata exists",
            "run codex-auto-dev new first",
            true,
        ),
        doctor_command_check("Git", "git", &["--version"], false),
        doctor_command_check("Codex CLI", "codex", &["--version"], true),
        doctor_command_check("GitHub CLI", "gh", &["--version"], true),
        doctor_command_check("CodeGraph CLI", &codegraph_bin, &["--version"], true),
        doctor_check(
            "Target repo",
            Path::new(DEV_REPO).join(".git").exists(),
            "target repository exists",
            "target repository is missing",
            false,
        ),
        doctor_check(
            "Agent tools",
            Path::new(ISSUE_TOOL).exists() && Path::new(ISSUE_AGENT_TOOL).exists(),
            "issue update and issue agent connectors exist",
            "missing issue update or issue agent connector",
            false,
        ),
        doctor_check(
            "Reviewer tools",
            Path::new(PLAN_REVIEW_TOOL).exists()
                && Path::new(TEST_REVIEW_TOOL).exists()
                && Path::new(DESIGN_REVIEW_TOOL).exists(),
            "plan/test/design reviewer connectors exist",
            "missing one or more reviewer connectors",
            false,
        ),
        doctor_check(
            "Review schema",
            review_schema_ready(),
            "strict review-result schema contains required gate fields",
            "review schema is missing required strict fields",
            false,
        ),
        doctor_check(
            "Events stream",
            Path::new(EVENTS_PATH).parent().is_some_and(Path::exists),
            "state directory is ready for events.ndjson",
            "state directory is missing",
            false,
        ),
        doctor_check(
            "CodeGraph index",
            !repo_has_commits(DEV_REPO) || codegraph_index_ready(DEV_REPO),
            "target repo is empty or dev/repo/.codegraph exists",
            "target repo has commits but dev/repo/.codegraph is missing; run codex-auto-dev plan or codegraph init -i dev/repo",
            true,
        ),
    ];

    println!("Codex Auto Dev Doctor Report");
    println!();
    for check in &checks {
        println!(
            "{:<15} {:<5} {}",
            check.name,
            check.status_label(),
            check.detail
        );
    }

    let failures = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Warn)
        .count();
    println!();
    println!("Summary: {failures} failed, {warnings} warning(s)");
    if failures > 0 {
        Err("doctor found blocking issue(s)".into())
    } else {
        Ok(())
    }
}

fn doctor_check(
    name: &'static str,
    ok: bool,
    ok_detail: &str,
    problem_detail: &str,
    blocking: bool,
) -> DoctorCheck {
    DoctorCheck {
        name,
        status: if ok {
            DoctorStatus::Ok
        } else if blocking {
            DoctorStatus::Fail
        } else {
            DoctorStatus::Warn
        },
        detail: if ok {
            ok_detail.to_string()
        } else {
            problem_detail.to_string()
        },
    }
}

fn doctor_command_check(
    name: &'static str,
    program: &str,
    args: &[&str],
    optional: bool,
) -> DoctorCheck {
    match Command::new(program).args(args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("installed")
                .trim()
                .to_string();
            DoctorCheck {
                name,
                status: DoctorStatus::Ok,
                detail: fallback_empty(&version, "installed").to_string(),
            }
        }
        Ok(output) => {
            let stderr = review_diagnostic_excerpt(&String::from_utf8_lossy(&output.stderr));
            DoctorCheck {
                name,
                status: if optional {
                    DoctorStatus::Warn
                } else {
                    DoctorStatus::Fail
                },
                detail: format!("{program} failed: {stderr}"),
            }
        }
        Err(error) => DoctorCheck {
            name,
            status: if optional {
                DoctorStatus::Warn
            } else {
                DoctorStatus::Fail
            },
            detail: format!("{program} unavailable: {error}"),
        },
    }
}

fn review_schema_ready() -> bool {
    let Ok(content) = fs::read_to_string(REVIEW_SCHEMA) else {
        return false;
    };
    [
        "\"reviewer\"",
        "\"approved\"",
        "\"gate_unavailable\"",
        "\"decision\"",
        "\"recommended_next_phase\"",
        "\"summary\"",
        "\"process\"",
        "\"critical\"",
        "\"high\"",
        "\"warning\"",
        "\"info\"",
        "\"impact\"",
        "\"suggested_change\"",
        "\"verification\"",
        "\"additionalProperties\": false",
    ]
    .iter()
    .all(|needle| content.contains(needle))
}
