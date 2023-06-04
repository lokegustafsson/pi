use crate::snapshot::Snapshot;
use std::{
    fs::File,
    process::Command,
    time::{Duration, Instant},
};

mod process;
mod snapshot;
mod system;

pub use process::ProcessInfo;
pub use system::{SystemInfo, SystemInfoTick};

const SUBSEC: u64 = 5;
pub const TICK_DELAY: Duration = Duration::from_micros(1_000_000 / SUBSEC);
pub const HISTORY: usize = (60 * SUBSEC + 1) as usize;

pub struct Ingester {
    handles: Handles,
    next_update_instant: Instant,
    scratch_buf: String,
    process_info: ProcessInfo,
    system_info: SystemInfo,
}
impl Ingester {
    pub fn new() -> Self {
        Self {
            handles: Handles::default(),
            next_update_instant: Instant::now(),
            scratch_buf: String::new(),
            process_info: ProcessInfo::default(),
            system_info: SystemInfo::default(),
        }
    }
    pub fn poll_update(&mut self) {
        while Instant::now() >= self.next_update_instant {
            self.next_update_instant += TICK_DELAY;
            self.tick_update();
        }
    }
    pub fn safe_sleep_duration(&self) -> Duration {
        self.next_update_instant
            .saturating_duration_since(Instant::now())
    }
    fn tick_update(&mut self) {
        // All data for a given tick is read as a `Snapshot`.
        let new = Snapshot::new(&mut self.scratch_buf, &mut self.handles);

        // We then update our persistent state using the `Snapshot`.
        self.process_info.update(&new);
        self.system_info.update(&new);
    }
    pub fn process_info(&self) -> &ProcessInfo {
        &self.process_info
    }
    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }
}

struct Handles {
    meminfo: File,
    swaps: File,
    stat: File,
}
impl Default for Handles {
    fn default() -> Self {
        let user_hz: u32 = {
            let output = Command::new("getconf").arg("CLK_TCK").output().unwrap();
            assert!(output.status.success());
            std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .parse()
                .unwrap()
        };
        assert_eq!(user_hz, 100);

        Self {
            meminfo: File::open("/proc/meminfo").unwrap(),
            swaps: File::open("/proc/swaps").unwrap(),
            stat: File::open("/proc/stat").unwrap(),
        }
    }
}
