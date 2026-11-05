//! Application-wide retained file-job limits, including switched-away workspaces.
use super::*;
use std::sync::{Mutex, OnceLock};
pub(super) const MAX_BYTES: usize = 1024 * 1024;
const MAX_JOBS: usize = 32;
#[derive(Default)]
struct Budget(Mutex<(usize, usize)>);
pub(super) struct Reservation {
    budget: Arc<Budget>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        let mut used = self.budget.0.lock().unwrap();
        used.0 -= 1;
        used.1 -= self.bytes;
    }
}
impl Budget {
    fn reserve(self: &Arc<Self>, bytes: usize) -> Result<Reservation, StoreError> {
        if bytes > MAX_BYTES {
            return Err(StoreError::TooLarge);
        }
        let mut used = self.0.lock().map_err(|_| StoreError::Unavailable)?;
        if used.0 >= MAX_JOBS || used.1 + bytes > MAX_JOBS * MAX_BYTES {
            return Err(StoreError::QueueFull);
        }
        used.0 += 1;
        used.1 += bytes;
        Ok(Reservation {
            budget: self.clone(),
            bytes,
        })
    }
}
pub(super) fn reserve(bytes: usize) -> Result<Reservation, StoreError> {
    static BUDGET: OnceLock<Arc<Budget>> = OnceLock::new();
    BUDGET
        .get_or_init(|| Arc::new(Budget::default()))
        .reserve(bytes)
}
// Count encoded intent without allocating an unbounded serialized copy.
pub(super) fn intent_bytes(value: &impl serde::Serialize) -> Result<usize, StoreError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(buf.len())
                .filter(|n| *n <= MAX_BYTES)
                .ok_or_else(|| std::io::Error::other("evidence intent exceeds limit"))?;
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|_| StoreError::TooLarge)?;
    Ok(counter.0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn job_and_byte_bounds_and_release_are_exact() {
        let budget = Arc::new(Budget::default());
        assert!(matches!(
            budget.reserve(MAX_BYTES + 1),
            Err(StoreError::TooLarge)
        ));
        let mut held: Vec<_> = (0..32)
            .map(|_| budget.reserve(MAX_BYTES).unwrap())
            .collect();
        assert_eq!(*budget.0.lock().unwrap(), (32, 32 * MAX_BYTES));
        assert!(matches!(
            budget.clone().reserve(0),
            Err(StoreError::QueueFull)
        ));
        held.pop();
        assert!(budget.reserve(MAX_BYTES).is_ok());
        drop(held);
        assert_eq!(*budget.0.lock().unwrap(), (0, 0));
        assert_eq!(intent_bytes(&"x".repeat(MAX_BYTES - 2)).unwrap(), MAX_BYTES);
        assert!(matches!(
            intent_bytes(&"x".repeat(MAX_BYTES - 1)),
            Err(StoreError::TooLarge)
        ));
    }
}
