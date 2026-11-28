//! Fixed startup messages for the macOS custody qualification.

const HEADER: [u8; 5] = *b"OWCO\x01";
pub(super) const FRAME_SIZE: usize = 54;

#[derive(Clone, Copy)]
#[repr(u8)]
pub(super) enum Direction {
    Challenge = 1,
    Ready = 2,
}

pub(super) fn encode(direction: Direction, nonce: &[u8; 16], build: &[u8; 32]) -> [u8; FRAME_SIZE] {
    let mut frame = [0; FRAME_SIZE];
    frame[..5].copy_from_slice(&HEADER);
    frame[5] = direction as u8;
    frame[6..22].copy_from_slice(nonce);
    frame[22..].copy_from_slice(build);
    frame
}

pub(super) fn matches(
    bytes: &[u8],
    direction: Direction,
    nonce: &[u8; 16],
    build: &[u8; 32],
) -> bool {
    bytes == encode(direction, nonce, build)
}

#[cfg(all(
    target_os = "macos",
    target_arch = "aarch64",
    feature = "codex-custody-qualification"
))]
pub(super) const WORKER_ARG: &str = "--orkworks-codex-custody-qualification-worker";

#[cfg(all(
    target_os = "macos",
    target_arch = "aarch64",
    feature = "codex-custody-qualification"
))]
pub(super) fn qualification_tag() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(b"orkworks:codex-custody-qualification:v1");
    for source in [
        include_bytes!("bootstrap_frame.rs").as_slice(),
        include_bytes!("qualification.rs").as_slice(),
        include_bytes!("deadline_io.rs").as_slice(),
    ] {
        hash.update((source.len() as u64).to_be_bytes());
        hash.update(source);
    }
    hash.finalize().into()
}
