//! Standalone std-only native executable, compiled by tests and never packaged.
use std::{
    env, fs,
    io::{self, Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

fn stdin_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();
    bytes
}

fn activation(args: &[String]) {
    assert_eq!(args.len(), 4);
    let marker = PathBuf::from(&args[1]);
    fs::write(marker.with_extension("model"), &args[0]).unwrap();
    fs::write(&marker, stdin_bytes()).unwrap();
    if args[3] == "nonzero" {
        eprint!("PRIVATE_PROVIDER_ERROR");
        std::process::exit(7);
    }
    if args[3] == "wait" {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.with_extension("release").exists() {
            if Instant::now() >= deadline {
                std::process::exit(8);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    io::stdout()
        .write_all(&fs::read(&args[2]).unwrap())
        .unwrap();
    io::stdout().flush().unwrap();
    fs::write(marker.with_extension("finished"), "finished").unwrap();
}

fn main() {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("activation") => activation(&args[1..]),
        Some("transport") => transport(&args[1..]),
        Some("hold-pipes") => {
            let root = PathBuf::from(env::var_os("CUSTOM_TEST_ROOT").unwrap());
            fs::write(root.join("descendant-started"), "started").unwrap();
            std::thread::sleep(Duration::from_secs(3));
            fs::write(root.join("descendant-survived"), "survived").unwrap();
        }
        _ => panic!("unknown fixture protocol"),
    }
}

fn transport(args: &[String]) {
    let root = PathBuf::from(env::var_os("CUSTOM_TEST_ROOT").unwrap());
    let mode = env::var("CUSTOM_TEST_MODE").unwrap();
    if mode == "timeout" {
        let _descendant = std::process::Command::new(env::current_exe().unwrap())
            .arg("hold-pipes").spawn().unwrap();
        std::thread::sleep(Duration::from_secs(3));
    }
    let mut fields = vec![env::current_dir().unwrap().to_str().unwrap().to_owned()];
    for name in [
        "HOME",
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
        "CUSTOM_PROVIDER_TOKEN",
        "ORKWORKS_REPORT_TOKEN",
        "ORKWORKS_ACTION_TOKEN",
        "BASH_ENV",
        "ENV",
        "ORKWORKS_FUTURE_CAPABILITY",
        "PATH",
        "ORKWORKS_LATE_CAPABILITY",
    ] {
        fields.push(env::var(name).unwrap_or_else(|_| "unset".into()));
    }
    fs::write(root.join("record"), fields.join("\n")).unwrap();
    let mut encoded = Vec::new();
    for arg in args {
        encoded.extend_from_slice(arg.as_bytes());
        encoded.push(0);
    }
    fs::write(root.join("args"), encoded).unwrap();
    if env::var("CUSTOM_TEST_FILE").unwrap() == "yes" {
        fs::write(root.join("prompt"), fs::read(&args[1]).unwrap()).unwrap();
        fs::write(root.join("stdin"), stdin_bytes()).unwrap();
    } else {
        fs::write(root.join("prompt"), stdin_bytes()).unwrap();
    }
    let response = env::var("CUSTOM_TEST_RESPONSE").unwrap();
    match mode.as_str() {
        "malformed" => print!("provider-secret-in-malformed-output"),
        "invalid-utf8" => {
            io::stdout().write_all(b"{\"version\":1,\"status\":\"success\",\"result\":\"{\\\"text\\\":\\\"\xff\\\"}\"}").unwrap();
        }
        "stdout-overflow" => {
            print!("{response}");
            io::stdout().write_all(&vec![b' '; 65_537]).unwrap();
        }
        "stderr-overflow" => {
            print!("{response}");
            io::stderr().write_all(&vec![b' '; 65_537]).unwrap();
        }
        "nonzero" | "fake-timeout" => {
            print!("{response}");
            io::stdout().flush().unwrap();
            eprint!(
                "{}",
                if mode == "nonzero" {
                    "provider-secret-in-stderr"
                } else {
                    "timed out"
                }
            );
            std::process::exit(7);
        }
        _ => print!("{response}"),
    }
}
