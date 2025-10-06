use std::time::Duration;

use libc::{RUSAGE_SELF, getrusage, rusage};
pub fn get_cpu_time() -> Duration {
    unsafe {
        let mut usage: rusage = std::mem::zeroed();
        // prefer RUSAGE_THREAD if available for per-thread CPU usage; fallback to RUSAGE_SELF
        #[cfg(target_os = "linux")]
        let who = libc::RUSAGE_THREAD;
        #[cfg(not(target_os = "linux"))]
        let who = RUSAGE_SELF;
        if getrusage(who, &mut usage) != 0 {
            return Duration::ZERO;
        }
        let user = Duration::new(
            usage.ru_utime.tv_sec as u64,
            (usage.ru_utime.tv_usec * 1000) as u32,
        );
        let sys = Duration::new(
            usage.ru_stime.tv_sec as u64,
            (usage.ru_stime.tv_usec * 1000) as u32,
        );
        user + sys
    }
}
