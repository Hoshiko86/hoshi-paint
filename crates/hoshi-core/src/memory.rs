//! Shared resident-pixel and scratch budget, including all open documents/history.
use crate::Error;
use std::sync::atomic::{AtomicU64, Ordering};
const MIB: u64 = 1024 * 1024;
static LIMIT: AtomicU64 = AtomicU64::new(1024 * MIB);
static USED: AtomicU64 = AtomicU64::new(0);
/// Configure before creating documents. Lowering below live usage is rejected.
pub fn configure_memory_budget(bytes: u64) -> Result<(), Error> {
    if bytes < USED.load(Ordering::Relaxed) || bytes == 0 {
        return Err(Error::MemoryLimit);
    }
    LIMIT.store(bytes, Ordering::Relaxed);
    Ok(())
}
pub fn memory_usage() -> u64 {
    USED.load(Ordering::Relaxed)
}
pub fn memory_budget() -> u64 {
    LIMIT.load(Ordering::Relaxed)
}
#[derive(Debug)]
pub struct MemoryReservation {
    bytes: u64,
}
impl MemoryReservation {
    pub fn new(bytes: u64) -> Result<Self, Error> {
        USED.try_update(Ordering::AcqRel, Ordering::Relaxed, |used| {
            used.checked_add(bytes)
                .filter(|total| *total <= LIMIT.load(Ordering::Relaxed))
        })
        .map_err(|_| Error::MemoryLimit)?;
        Ok(Self { bytes })
    }
}
impl Drop for MemoryReservation {
    fn drop(&mut self) {
        USED.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
