use process_ownership_fixture::targets;

fn main() {
    match targets::run_from_args(std::env::args_os().skip(1)) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("process ownership fixture target failed: {error}");
            std::process::exit(2);
        }
    }
}
