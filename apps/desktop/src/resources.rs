//! Desktop policy; the core accepts an explicit shared budget on every platform.
use std::sync::Once;
pub fn initialize() {
    static INITIALIZE: Once = Once::new();
    INITIALIZE.call_once(|| {
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        let mib = 1024 * 1024;
        let automatic = (system.available_memory() / 4).max(64 * mib);
        let budget = std::env::var("HOSHI_MEMORY_LIMIT_MB")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| *v > 0)
            .and_then(|v| v.checked_mul(mib))
            .unwrap_or(automatic);
        let _ = hoshi_core::configure_memory_budget(budget);
    });
}
