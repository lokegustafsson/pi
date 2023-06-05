use crate::{handles::Handles, snapshot::Snapshot};
use std::time::{Duration, Instant};

mod handles;
mod process;
mod snapshot;
mod system;

pub use process::ProcessInfo;
pub use system::{SystemInfo, SystemInfoTick};

const SUBSEC: u64 = 60;
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
            handles: Handles::new(),
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
        // Possibly refresh handles (added/removed interfaces/disk/etc)
        self.handles.update();

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
