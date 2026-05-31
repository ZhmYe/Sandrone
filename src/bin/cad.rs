use std::env;
use std::process::Command;

fn main() {
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("cad could not locate its executable path: {error}");
            std::process::exit(1);
        }
    };
    let codex_auto_dev = current_exe.with_file_name("codex-auto-dev");
    let status = Command::new(&codex_auto_dev)
        .args(env::args().skip(1))
        .status();
    match status {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!(
                "cad could not execute {}: {error}",
                codex_auto_dev.display()
            );
            std::process::exit(1);
        }
    }
}
