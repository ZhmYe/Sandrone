use std::env;
use std::process::Command;

fn main() {
    let current_exe = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("sdr could not locate its executable path: {error}");
            std::process::exit(1);
        }
    };
    let sandrone = current_exe.with_file_name("sandrone");
    let status = Command::new(&sandrone).args(env::args().skip(1)).status();
    match status {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(error) => {
            eprintln!("sdr could not execute {}: {error}", sandrone.display());
            std::process::exit(1);
        }
    }
}
