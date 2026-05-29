use std::env;
use std::process::ExitCode;
use upbit_mock::{run_blocking_server, ServerConfig};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let addr = args.next().unwrap_or_else(|| "127.0.0.1:8001".to_owned());
    let repo_root = args
        .next()
        .map(Into::into)
        .unwrap_or_else(|| env::current_dir().expect("current directory should be readable"));

    match run_blocking_server(ServerConfig::new(addr, repo_root)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
