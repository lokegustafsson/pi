use crate::procfs;
use std::{collections::BTreeMap, process::Command};

pub struct ProcIngest {
    pub by_pid: BTreeMap<u32, ProcessIngest>,
}
pub struct ProcessIngest {
    pub kernel: bool,
    pub cmdline: String,
    pub by_tid: BTreeMap<u32, ThreadIngest>,

    pub status: procfs::PidStatus,
    pub uid: u16,
    pub gid: u16,
    pub vm_rss_bytes: u64,
}
pub struct ThreadIngest {
    /// Sometimes requires `PTRACE_MODE_READ_FSCREDS`.
    pub io: Option<procfs::TidIo>,
    cumulative_read_bytes: u64,
    cumulative_write_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,

    pub stat: procfs::TidStat,
    pub sid: u32,
    cumulative_user_time_ms: u64,
    cumulative_system_time_ms: u64,
    cumulative_guest_time_ms: u64,
    pub user_time_ms: u32,
    pub system_time_ms: u32,
    pub guest_time_ms: u32,
}
impl ProcIngest {
    pub fn new(scratch: &mut String) -> Self {
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

        let mut ret = Self {
            by_pid: BTreeMap::new(),
        };
        ret.update(scratch);
        ret
    }
    pub fn update(&mut self, scratch: &mut String) {
        self.by_pid = procfs::get_live_pids()
            .filter_map(|pid| {
                Some((
                    pid,
                    ProcessIngest::new_from_old(pid, self.by_pid.remove(&pid), scratch)?,
                ))
            })
            .collect();
        scratch.clear();
    }
}
impl ProcessIngest {
    fn new_from_old(pid: u32, old: Option<Self>, scratch: &mut String) -> Option<Self> {
        let mut old = old.or_else(|| {
            let (kernel, cmdline) = procfs::get_is_kernel_and_cmdline(pid)?;
            Some(Self {
                kernel,
                cmdline,
                by_tid: BTreeMap::new(),
                status: procfs::PidStatus::new(pid, kernel),
                uid: 0,
                gid: 0,
                vm_rss_bytes: 0,
            })
        })?;
        let (uid, gid, vm_rss_bytes) = old.status.get_uid_gid_vm_rss_bytes(scratch)?;
        Some(ProcessIngest {
            kernel: old.kernel,
            cmdline: old.cmdline,
            by_tid: ThreadIngest::new_by_tid(pid, old.by_tid, scratch)?,
            status: old.status,
            uid: uid as u16,
            gid: gid as u16,
            vm_rss_bytes,
        })
    }
}
impl ThreadIngest {
    fn new_by_tid(
        pid: u32,
        mut old: BTreeMap<u32, ThreadIngest>,
        scratch: &mut String,
    ) -> Option<BTreeMap<u32, ThreadIngest>> {
        let mut ret = BTreeMap::new();
        for tid in procfs::get_live_tids(pid) {
            let mut old = old.remove(&tid).unwrap_or_else(|| ThreadIngest {
                io: procfs::TidIo::new(pid, tid),
                cumulative_read_bytes: 0,
                cumulative_write_bytes: 0,
                read_bytes: 0,
                write_bytes: 0,
                stat: procfs::TidStat::new(pid, tid).unwrap(),
                sid: 0,
                cumulative_user_time_ms: 0,
                cumulative_system_time_ms: 0,
                cumulative_guest_time_ms: 0,
                user_time_ms: 0,
                system_time_ms: 0,
                guest_time_ms: 0,
            });
            let (cumulative_read_bytes, cumulative_write_bytes) = match old.io.as_mut() {
                Some(io) => io.get_cumulative_read_write_bytes(scratch)?,
                None => (0, 0),
            };

            let (sid, cumulative_user_time_ms, cumulative_system_time_ms, cumulative_guest_time_ms) =
                old.stat
                    .get_sid_cumulative_user_system_guest_time(scratch)?;
            ret.insert(
                tid,
                ThreadIngest {
                    io: old.io,
                    cumulative_read_bytes,
                    cumulative_write_bytes,
                    read_bytes: cumulative_read_bytes - old.cumulative_read_bytes,
                    write_bytes: cumulative_write_bytes - old.cumulative_write_bytes,
                    stat: old.stat,
                    sid,
                    cumulative_user_time_ms,
                    cumulative_system_time_ms,
                    cumulative_guest_time_ms,
                    user_time_ms: (cumulative_user_time_ms - old.cumulative_user_time_ms) as u32,
                    system_time_ms: (cumulative_system_time_ms - old.cumulative_system_time_ms)
                        as u32,
                    guest_time_ms: (cumulative_guest_time_ms - old.cumulative_guest_time_ms) as u32,
                },
            );
        }
        Some(ret)
    }
}
