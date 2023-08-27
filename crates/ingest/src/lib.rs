use crate::{
    handles::Handles,
    snapshot::{OldSnapshot, Snapshot},
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

mod handles;
mod process;
mod snapshot;
mod system;

pub use process::ProcessInfo;
pub use system::SystemInfo;

pub const SUBSEC: u64 = 30;
pub const TICK_DELAY: Duration = Duration::from_micros(1_000_000 / SUBSEC);
pub const HISTORY: usize = (60 * SUBSEC + 1) as usize;

pub struct Ingester {
    handles: Handles,
    next_update_instant: Instant,
    scratch_buf: String,
    old_snapshot: OldSnapshot,
    process_info: ProcessInfo,
    system_info: SystemInfo,
}
impl Ingester {
    pub fn new() -> Self {
        let mut handles = Handles::new();
        let mut scratch_buf = String::new();
        let old_snapshot = Snapshot::new(&mut scratch_buf, &mut handles).retire();
        Self {
            handles,
            next_update_instant: Instant::now(),
            scratch_buf,
            old_snapshot,
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
        self.process_info.update(&new, &self.old_snapshot);
        self.system_info.update(&new, &self.old_snapshot);
        self.old_snapshot = new.retire();
    }
    pub fn process_info(&self) -> &ProcessInfo {
        &self.process_info
    }
    pub fn system_info(&self) -> &SystemInfo {
        &self.system_info
    }
}

#[derive(Clone, Default, Debug)]
pub struct Series<T: Copy + Default> {
    inner: VecDeque<T>,
}
impl<T: Copy + Default> Series<T> {
    fn push(&mut self, item: T) {
        while self.inner.len() >= HISTORY {
            self.inner.pop_front();
        }
        self.inner.push_back(item);
    }
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn latest(&self) -> T {
        *self.inner.back().unwrap()
    }
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.inner.iter()
    }
}
