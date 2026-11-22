/// Wall-clock milliseconds since the Unix epoch, for user-visible timestamps (Backstage's "last
/// changed", toasts). It can jump when the system clock is adjusted, so it must not drive
/// deadlines or pacing; use [`monotonic_ms`] for those.
pub fn now_ms() -> f64 {
    localsheets_engine::core::date::unix_now_ms().unwrap_or(0.0)
}

/// Monotonic milliseconds since the first call, for screenshot deadlines and AutoSave pacing.
///
/// Deliberately separate from [`now_ms`]: a wall-clock adjustment must not extend (or expire) a
/// screenshot deadline or save a workbook twice.
#[allow(clippy::disallowed_methods)]
pub fn monotonic_ms() -> f64 {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
}
