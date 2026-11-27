use std::time::Duration;

#[cfg(target_os = "linux")]
pub fn boot_time() -> Option<Duration> {
    use rustix::time::{clock_gettime_dynamic, DynamicClockId};

    let time = clock_gettime_dynamic(DynamicClockId::Boottime).ok()?;
    let seconds = u64::try_from(time.tv_sec).ok()?;
    let nanos = u32::try_from(time.tv_nsec).ok()?;
    Some(Duration::new(seconds, nanos))
}

#[cfg(not(target_os = "linux"))]
pub const fn boot_time() -> Option<Duration> {
    None
}

// Windows is a development/test target; replica activation is Linux-only.
pub fn within_boot_budget(start: Option<Duration>, budget: Duration) -> bool {
    if cfg!(target_os = "linux") {
        return boot_time()
            .zip(start)
            .and_then(|(now, start)| now.checked_sub(start))
            .is_some_and(|elapsed| elapsed < budget);
    }
    true
}
