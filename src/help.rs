pub(crate) fn print_help() {
    println!(
        "Usage: sandrone <command>\n\nPublic commands:\n  loop start [--interval-seconds 900] [--parallel-limit 1]\n      Start the automation loop.\n  loop restart [--request_id <REQ-0001>]\n      Resume blocked work; run loop start afterwards to continue automation.\n  loop stop [--request_id <REQ-0001>] [--reason <reason>]\n      Stop the loop worker, or actively block one request when request_id is provided.\n  dashboard [--host 127.0.0.1] [--port 47217] [--json]\n      Open or export the local Sandrone dashboard.\n\nInternal commands still exist for hooks, generated scripts, recovery, and tests, but ordinary operation should go through loop and dashboard."
    );
}
