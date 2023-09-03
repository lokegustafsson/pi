use procinfo::{ProcInfo, ProcIngest};
use std::time::{Duration, Instant};
use sysinfo::{SysHandles, SysInfo, SysOldSnapshot, SysSnapshot};
use util::{TICK_DELAY};

pub struct Ingester {
    next_update_instant: Instant,
    scratch_buf: String,

    sys_handles: SysHandles,
    sys_old_snapshot: SysOldSnapshot,
    sys_info: SysInfo,

    proc_ingest: ProcIngest,
    proc_info: ProcInfo,
}
impl Ingester {
    pub fn new() -> Self {
        let mut scratch_buf = String::new();

        let mut sys_handles = SysHandles::new();
        let sys_old_snapshot = SysSnapshot::new(&mut scratch_buf, &mut sys_handles).retire();

        Self {
            next_update_instant: Instant::now(),
            scratch_buf,
            sys_handles,
            sys_old_snapshot,
            sys_info: SysInfo::default(),
            proc_ingest: ProcIngest::new(),
            proc_info: ProcInfo::new(),
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
        self.sys_handles.update();

        // All data for a given tick is read as a `Snapshot`.
        let new = SysSnapshot::new(&mut self.scratch_buf, &mut self.sys_handles);

        // We then update our persistent state using the `Snapshot`.
        self.sys_info.update(&new, &self.sys_old_snapshot);
        self.sys_old_snapshot = new.retire();

        //self.proc_ingest.update();
        //self.proc_info.update(&self.proc_ingest);
    }
    pub fn process_info(&self) -> &ProcInfo {
        &self.proc_info
    }
    pub fn system_info(&self) -> &SysInfo {
        &self.sys_info
    }
}
