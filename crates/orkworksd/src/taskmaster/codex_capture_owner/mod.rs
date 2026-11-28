//! Dormant macOS startup-sealed Codex custody owner qualification modules.
//!
//! Only the early private-worker dispatch seam (`run_private_worker`) is wired
//! into `main()`; normal production startup does not construct a custody pool.

mod bootstrap_frame;
#[allow(dead_code)] // Exercised by macOS qualification tests; dormant in normal startup.
pub(crate) mod channels;
#[allow(dead_code)] // Exercised by macOS qualification tests; dormant in normal startup.
pub(crate) mod framing;
#[allow(dead_code)] // Exercised by macOS qualification tests; dormant in normal startup.
pub(crate) mod lease;
#[allow(dead_code)] // Exercised by macOS qualification tests; dormant in normal startup.
pub(crate) mod pool;
#[allow(dead_code)] // Entry point invoked from main.rs; helper constants shared with tests.
pub(crate) mod worker;

/// Deterministic 32-byte source-bound build identity derived via SHA-256 over
/// package/target/profile metadata and the custody module sources without
/// modifying frozen `build.rs` inputs.
#[allow(dead_code)] // Exercised by macOS qualification tests and worker handshake.
pub(crate) fn build_identity() -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut hash = Sha256::new();
    let profile: &[u8] = if cfg!(debug_assertions) {
        b"debug"
    } else {
        b"release"
    };
    let fields: [&[u8]; 13] = [
        b"orkworks:codex-custody-build:v1",
        env!("CARGO_PKG_NAME").as_bytes(),
        env!("CARGO_PKG_VERSION").as_bytes(),
        std::env::consts::OS.as_bytes(),
        std::env::consts::ARCH.as_bytes(),
        profile,
        include_bytes!("mod.rs"),
        include_bytes!("bootstrap_frame.rs"),
        include_bytes!("channels.rs"),
        include_bytes!("framing.rs"),
        include_bytes!("lease.rs"),
        include_bytes!("worker.rs"),
        include_bytes!("pool.rs"),
    ];
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field);
    }
    hash.finalize().into()
}

#[allow(dead_code)] // Exercised by macOS qualification integration tests.
pub(crate) fn encode_challenge_for_test(
    nonce: &[u8; 16],
    build: &[u8; 32],
) -> [u8; bootstrap_frame::FRAME_SIZE] {
    bootstrap_frame::encode(bootstrap_frame::Direction::Challenge, nonce, build)
}

#[allow(dead_code)] // Entry point invoked from main.rs.
pub(crate) fn run_private_worker() -> Option<i32> {
    worker::run_private_worker()
}
