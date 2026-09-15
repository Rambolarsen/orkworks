#[cfg(windows)]
use process_ownership_fixture::platform;
use process_ownership_fixture::targets;

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    #[cfg(windows)]
    match platform::run_helper_from_args(&args) {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("process ownership fixture helper failed: {error}");
            std::process::exit(2);
        }
    }
    match targets::run_from_args(args) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("process ownership fixture target failed: {error}");
            std::process::exit(2);
        }
    }
}
