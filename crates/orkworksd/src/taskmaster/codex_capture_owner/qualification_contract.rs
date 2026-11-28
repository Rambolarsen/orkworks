//! Fixed build opt-in and startup compatibility tag for the native fixture.

pub(super) const WORKER_ARG: &str = "--orkworks-codex-custody-qualification-worker";

pub(super) fn enabled() -> bool {
    matches!(option_env!("ORKWORKS_CUSTODY_QUALIFICATION"), Some("1"))
}

pub(super) fn qualification_tag() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"orkworks:codex-custody-qualification:v1");
    for source in [
        include_bytes!("bootstrap_frame.rs").as_slice(),
        include_bytes!("qualification.rs").as_slice(),
        include_bytes!("deadline_io.rs").as_slice(),
        include_bytes!("qualification_contract.rs").as_slice(),
    ] {
        hash.update((source.len() as u64).to_be_bytes());
        hash.update(source);
    }
    hash.finalize().into()
}
