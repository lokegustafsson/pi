use crate::{
    handles::Handles,
    snapshot::{OldSnapshot, Snapshot},
};
use std::time::{Duration, Instant};

mod handles;
mod process;
mod snapshot;
mod system;

pub use process::ProcessInfo;
pub use system::SystemInfo;

pub const SUBSEC: u64 = 60;
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

#[derive(Clone, Debug)]
pub struct Series<T: Copy + Default> {
    inner: Box<[T; HISTORY]>,
    last: usize,
}
impl<T: Copy + Default> Default for Series<T> {
    fn default() -> Self {
        Self {
            inner: Box::new([T::default(); HISTORY]),
            last: HISTORY - 1,
        }
    }
}
impl<T: Copy + Default> Series<T> {
    fn push(&mut self, item: T) {
        self.last += 1;
        if self.last == HISTORY {
            self.last = 0;
        }
        self.inner[self.last] = item;
    }
    pub fn capacity() -> usize {
        HISTORY
    }
    pub fn latest(&self) -> T {
        self.inner[self.last]
    }
    pub fn iter<'a>(&'a self) -> impl 'a + Iterator<Item = T> {
        Iterator::chain(
            self.inner[(self.last + 1)..].iter().copied(),
            self.inner[..(self.last + 1)].iter().copied(),
        )
    }
    pub fn chunks<'a>(
        &'a self,
        chunk_size: usize,
    ) -> (&'a [T], impl Iterator<Item = &'a [T]>, &'a [T]) {
        let tail = self.inner[(self.last + 1)..].rchunks_exact(chunk_size);
        let head = self.inner[..(self.last + 1)].chunks_exact(chunk_size);
        let first_chunk = tail.remainder();
        let last_chunk = head.remainder();
        let iterator = tail.rev().chain(head);
        (first_chunk, iterator, last_chunk)
    }
}
