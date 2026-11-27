//! Fixed startup messages for the macOS custody qualification.

const HEADER: [u8; 5] = *b"OWCO\x01";
const FRAME_SIZE: usize = 54;

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
