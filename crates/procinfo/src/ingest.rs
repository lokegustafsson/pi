use crate::procfs;
use either::Either;
use std::{collections::BTreeMap, process::Command};

pub struct ProcIngest {
    pub by_pid: BTreeMap<u32, ProcessIngest>,
}
pub struct ProcessIngest {
    pub kernel: bool,
    pub name: String,
    pub cmdline: Option<String>,
    pub by_tid: BTreeMap<u32, ThreadIngest>,

    pub status: procfs::PidStatus,
    pub uid: u16,
    pub gid: u16,
    pub vm_rss_kb: u64,
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
    pub fn new() -> Self {
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
        let fd_limit = 8196;
        nix::sys::resource::setrlimit(
            nix::sys::resource::Resource::RLIMIT_NOFILE,
            fd_limit,
            fd_limit,
        )
        .unwrap();

        let mut ret = Self {
            by_pid: BTreeMap::new(),
        };
        ret.update();
        ret
    }
    pub fn update(&mut self) {
        self.by_pid = procfs::get_live_pids()
            .filter_map(|pid| {
                Some((
                    pid,
                    ProcessIngest::new_from_old(pid, self.by_pid.remove(&pid))?,
                ))
            })
            .collect();
    }
}
impl ProcessIngest {
    fn new_from_old(pid: u32, old: Option<Self>) -> Option<Self> {
        let mut old = old.or_else(|| {
            let (kernel, name, cmdline) = procfs::get_is_kernel_name_cmdline(pid)?;
            Some(Self {
                kernel,
                name,
                cmdline,
                by_tid: BTreeMap::new(),
                status: procfs::PidStatus::new(pid, kernel)?,
                uid: 0,
                gid: 0,
                vm_rss_kb: 0,
            })
        })?;
        let (uid, gid, vm_rss_kb, threads) = old.status.get_uid_gid_vm_rss_kb_threads()?;
        Some(ProcessIngest {
            kernel: old.kernel,
            name: old.name,
            cmdline: old.cmdline,
            by_tid: ThreadIngest::new_by_tid(pid, old.by_tid, threads == 1)?,
            status: old.status,
            uid: uid as u16,
            gid: gid as u16,
            vm_rss_kb,
        })
    }
}
impl ThreadIngest {
    fn new_by_tid(
        pid: u32,
        mut old: BTreeMap<u32, ThreadIngest>,
        single_threaded: bool,
    ) -> Option<BTreeMap<u32, ThreadIngest>> {
        let mut ret = BTreeMap::new();
        for tid in match single_threaded {
            true => Either::Left([pid].into_iter()),
            false => Either::Right(procfs::get_live_tids(pid)),
        } {
            let mut old = old.remove(&tid).or_else(|| {
                Some(ThreadIngest {
                    io: procfs::TidIo::new(pid, tid),
                    cumulative_read_bytes: 0,
                    cumulative_write_bytes: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    stat: procfs::TidStat::new(pid, tid)?,
                    sid: 0,
                    cumulative_user_time_ms: 0,
                    cumulative_system_time_ms: 0,
                    cumulative_guest_time_ms: 0,
                    user_time_ms: 0,
                    system_time_ms: 0,
                    guest_time_ms: 0,
                })
            })?;
            let (cumulative_read_bytes, cumulative_write_bytes) = match old.io.as_mut() {
                Some(io) => io.get_cumulative_read_write_bytes()?,
                None => (0, 0),
            };

            let (sid, cumulative_user_time_ms, cumulative_system_time_ms, cumulative_guest_time_ms) =
                old.stat.get_sid_cumulative_user_system_guest_time()?;
            ret.insert(
                tid,
                ThreadIngest {
                    io: old.io,
                    cumulative_read_bytes,
                    cumulative_write_bytes,
                    read_bytes: cumulative_read_bytes.saturating_sub(old.cumulative_read_bytes),
                    write_bytes: cumulative_write_bytes.saturating_sub(old.cumulative_write_bytes),
                    stat: old.stat,
                    sid,
                    cumulative_user_time_ms,
                    cumulative_system_time_ms,
                    cumulative_guest_time_ms,
                    user_time_ms: cumulative_user_time_ms
                        .saturating_sub(old.cumulative_user_time_ms)
                        as u32,
                    system_time_ms: cumulative_system_time_ms
                        .saturating_sub(old.cumulative_system_time_ms)
                        as u32,
                    guest_time_ms: cumulative_guest_time_ms
                        .saturating_sub(old.cumulative_guest_time_ms)
                        as u32,
                },
            );
        }
        Some(ret)
    }
}
