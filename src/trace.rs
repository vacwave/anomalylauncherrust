//! Optional timing log for diagnosing slow start-ups. Built only with
//! `--features trace`; writes AnomalyLauncher_trace.log next to the exe.

#[cfg(feature = "trace")]
mod imp {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();
    static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

    pub fn init() {
        START.get_or_init(Instant::now);
        let path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("AnomalyLauncher_trace.log")))
            .unwrap_or_else(|| "AnomalyLauncher_trace.log".into());
        *FILE.lock().unwrap() = std::fs::File::create(path).ok();
    }

    pub fn log(msg: &str) {
        let t = START.get_or_init(Instant::now).elapsed();
        if let Some(f) = FILE.lock().unwrap().as_mut() {
            let _ = writeln!(f, "[{:>9.3} ms] {}", t.as_secs_f64() * 1000.0, msg);
        }
    }
}

#[cfg(feature = "trace")]
pub use imp::{init, log};

#[cfg(not(feature = "trace"))]
pub fn init() {}

#[cfg(not(feature = "trace"))]
#[inline(always)]
pub fn log(_msg: &str) {}
