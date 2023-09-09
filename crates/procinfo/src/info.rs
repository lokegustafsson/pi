use crate::ingest::ProcIngest;
use std::{
    collections::HashMap,
    fs,
    ops::{Add, AddAssign},
};

#[derive(Debug)]
pub struct ProcInfo {
    update_hz: u8,
    sort_by: ProcSortBy,
    pub uid_to_user: HashMap<u16, UserInfo>,
    pub gid_to_group: HashMap<u16, GroupInfo>,
    pub strings: StringArena,
    pub login_sessions: Vec<LoginSessionInfo>,
    pub sessions: Vec<SessionInfo>,
    pub processes: Vec<ProcessInfo>,
    pub threads: Vec<ThreadInfo>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcSortBy {
    Id,
    Name,
    Cpu,
    DiskRead,
    DiskWrite,
    Memory,
}
#[derive(Debug)]
pub struct LoginSessionInfo {
    pub lsid: Lsid,
    pub stat: ProcStat,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lsid {
    Kernel,
    SystemdServices,
    SystemdSession(u16),
}
#[derive(Debug)]
pub struct SessionInfo {
    pub parent_lsid: Lsid,
    pub sid: u32,
    pub name: StringArenaHandle,
    pub entries_cmdline: String,
    pub stat: ProcStat,
}
#[derive(Debug)]
pub struct ProcessInfo {
    pub parent_sid: u32,
    pub pid: u32,
    pub uid: u16,
    pub gid: u16,
    pub name: StringArenaHandle,
    pub cmdline: Option<String>,
    pub stat: ProcStat,
}
#[derive(Debug)]
pub struct ThreadInfo {
    pub parent_pid: u32,
    pub tid: u32,
    pub name: StringArenaHandle,
    pub stat: ProcStat,
}
#[derive(Clone, Copy, Debug)]
pub struct ProcStat {
    pub guest_time_millis: u32,
    pub user_time_millis: u32,
    pub system_time_millis: u32,
    pub disk_read_bytes_per_second: u64,
    pub disk_write_bytes_per_second: u64,
    pub mem_bytes: u64,
}

impl ProcInfo {
    pub fn new() -> Self {
        let uid_to_user = fs::read_to_string("/etc/passwd")
            .unwrap()
            .lines()
            .map(|line| {
                let user = UserInfo::new(line);
                (user.uid, user)
            })
            .collect();
        let gid_to_group = fs::read_to_string("/etc/group")
            .unwrap()
            .lines()
            .map(|line| {
                let group = GroupInfo::new(line);
                (group.gid, group)
            })
            .collect();
        Self {
            update_hz: 1,
            sort_by: ProcSortBy::Id,
            uid_to_user,
            gid_to_group,
            strings: StringArena::default(),

            login_sessions: Vec::new(),
            sessions: Vec::new(),
            processes: Vec::new(),
            threads: Vec::new(),
        }
    }
    pub fn update(&mut self, src: &ProcIngest) {
        self.strings = StringArena::default();
        self.login_sessions = Vec::new();
        self.sessions = Vec::new();
        self.processes = Vec::new();
        self.threads = Vec::new();
        for (&pid, process) in &src.by_pid {
            let thread_start_idx = self.threads.len();
            let name = self.strings.push(process.name.clone());
            for (&tid, thread) in &process.by_tid {
                self.threads.push(ThreadInfo {
                    parent_pid: pid,
                    tid,
                    name,
                    stat: ProcStat {
                        guest_time_millis: thread.guest_time_ms,
                        user_time_millis: thread.user_time_ms,
                        system_time_millis: thread.system_time_ms,
                        disk_read_bytes_per_second: thread.read_bytes / self.update_hz as u64,
                        disk_write_bytes_per_second: thread.write_bytes / self.update_hz as u64,
                        mem_bytes: process.vm_rss_kb * 1024,
                    },
                });
            }
            self.processes.push(ProcessInfo {
                parent_sid: process.by_tid.first_key_value().unwrap().1.sid,
                pid,
                uid: process.uid,
                gid: process.gid,
                name,
                cmdline: process.cmdline.clone(),
                stat: {
                    let mut stat = self.threads[thread_start_idx..]
                        .iter()
                        .map(|thread| thread.stat)
                        .fold(ProcStat::ZERO, ProcStat::add);
                    stat.mem_bytes = process.vm_rss_kb * 1024;
                    stat
                },
            });
        }
        self.processes.sort_by_key(|p| (p.parent_sid, p.pid));
        for p in &self.processes {
            if self
                .sessions
                .last()
                .map_or(true, |session| session.sid != p.parent_sid)
            {
                self.sessions.push(SessionInfo {
                    parent_lsid: if p.parent_sid == 0 {
                        Lsid::Kernel
                    } else {
                        Lsid::SystemdServices
                    },
                    sid: p.parent_sid,
                    name: p.name,
                    entries_cmdline: String::new(),
                    stat: ProcStat::ZERO,
                });
            }
            let sess = self.sessions.last_mut().unwrap();
            if let Some(cmdline) = p.cmdline.as_ref() {
                if !sess.entries_cmdline.is_empty() {
                    sess.entries_cmdline.push_str("\n");
                }
                sess.entries_cmdline.push_str(cmdline);
            }
            sess.stat += p.stat;
        }
        self.login_sessions = vec![
            LoginSessionInfo {
                lsid: Lsid::Kernel,
                stat: ProcStat::ZERO,
            },
            LoginSessionInfo {
                lsid: Lsid::SystemdServices,
                stat: ProcStat::ZERO,
            },
        ];
        for s in &self.sessions {
            let idx = match s.parent_lsid {
                Lsid::Kernel => 0,
                Lsid::SystemdServices => 1,
                Lsid::SystemdSession(_) => unreachable!(),
            };
            self.login_sessions[idx].stat += s.stat;
        }
        self.sort_self();
    }
    pub fn get_sort_by(&self) -> ProcSortBy {
        self.sort_by
    }
    pub fn sort(&mut self, sort_by: ProcSortBy) {
        if self.sort_by == sort_by {
            return;
        }
        self.sort_by = sort_by;
        self.sort_self();
    }
    fn sort_self(&mut self) {
        match self.sort_by {
            ProcSortBy::Id => {
                self.login_sessions.sort_by_key(|ls| ls.lsid);
                self.sessions.sort_by_key(|s| s.sid);
                self.processes.sort_by_key(|p| p.pid);
                self.threads.sort_by_key(|t| t.tid);
            }
            ProcSortBy::Name => {
                self.login_sessions.sort_by_key(|ls| ls.lsid);
                self.sessions.sort_by(|s1, s2| {
                    Ord::cmp(
                        &(&self.strings.get(s1.name), s1.sid),
                        &(&self.strings.get(s2.name), s2.sid),
                    )
                });
                self.processes.sort_by(|p1, p2| {
                    Ord::cmp(
                        &(&self.strings.get(p1.name), p1.pid),
                        &(&self.strings.get(p2.name), p2.pid),
                    )
                });
                self.threads.sort_by(|t1, t2| {
                    Ord::cmp(
                        &(&self.strings.get(t1.name), t1.tid),
                        &(&self.strings.get(t2.name), t2.tid),
                    )
                });
            }
            ProcSortBy::Cpu => {
                self.login_sessions.sort_by_key(|ls| {
                    (
                        u32::MAX - ls.stat.user_time_millis - ls.stat.system_time_millis,
                        ls.lsid,
                    )
                });
                self.sessions.sort_by_key(|s| {
                    (
                        u32::MAX - s.stat.user_time_millis - s.stat.system_time_millis,
                        s.sid,
                    )
                });
                self.processes.sort_by_key(|p| {
                    (
                        u32::MAX - p.stat.user_time_millis - p.stat.system_time_millis,
                        p.pid,
                    )
                });
                self.threads.sort_by_key(|t| {
                    (
                        u32::MAX - t.stat.user_time_millis - t.stat.system_time_millis,
                        t.tid,
                    )
                });
            }
            ProcSortBy::DiskRead => {
                self.login_sessions
                    .sort_by_key(|ls| (u64::MAX - ls.stat.disk_read_bytes_per_second, ls.lsid));
                self.sessions
                    .sort_by_key(|s| (u64::MAX - s.stat.disk_read_bytes_per_second, s.sid));
                self.processes
                    .sort_by_key(|p| (u64::MAX - p.stat.disk_read_bytes_per_second, p.pid));
                self.threads
                    .sort_by_key(|t| (u64::MAX - t.stat.disk_read_bytes_per_second, t.tid));
            }
            ProcSortBy::DiskWrite => {
                self.login_sessions
                    .sort_by_key(|ls| (u64::MAX - ls.stat.disk_write_bytes_per_second, ls.lsid));
                self.sessions
                    .sort_by_key(|s| (u64::MAX - s.stat.disk_write_bytes_per_second, s.sid));
                self.processes
                    .sort_by_key(|p| (u64::MAX - p.stat.disk_write_bytes_per_second, p.pid));
                self.threads
                    .sort_by_key(|t| (u64::MAX - t.stat.disk_write_bytes_per_second, t.tid));
            }
            ProcSortBy::Memory => {
                self.login_sessions
                    .sort_by_key(|ls| (u64::MAX - ls.stat.mem_bytes, ls.lsid));
                self.sessions
                    .sort_by_key(|s| (u64::MAX - s.stat.mem_bytes, s.sid));
                self.processes
                    .sort_by_key(|p| (u64::MAX - p.stat.mem_bytes, p.pid));
                self.threads
                    .sort_by_key(|t| (u64::MAX - t.stat.mem_bytes, t.tid));
            }
        }
    }
}
impl ProcStat {
    const ZERO: Self = Self {
        guest_time_millis: 0,
        user_time_millis: 0,
        system_time_millis: 0,
        disk_read_bytes_per_second: 0,
        disk_write_bytes_per_second: 0,
        mem_bytes: 0,
    };
}
impl Add for ProcStat {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            guest_time_millis: self.guest_time_millis + rhs.guest_time_millis,
            user_time_millis: self.user_time_millis + rhs.user_time_millis,
            system_time_millis: self.system_time_millis + rhs.system_time_millis,
            disk_read_bytes_per_second: self.disk_read_bytes_per_second
                + rhs.disk_read_bytes_per_second,
            disk_write_bytes_per_second: self.disk_write_bytes_per_second
                + rhs.disk_write_bytes_per_second,
            mem_bytes: self.mem_bytes + rhs.mem_bytes,
        }
    }
}
impl AddAssign for ProcStat {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[derive(Debug)]
pub struct UserInfo {
    pub name: String,
    pub uid: u16,
    pub gid: u16,
    pub description: String,
    pub kind: UserKind,
}
#[derive(Clone, Debug)]
pub enum UserKind {
    Root,
    User,
    Nologin,
}
impl UserInfo {
    fn new(line: &str) -> Self {
        let mut words = line.trim().split(":");
        let name = words.next().unwrap().to_owned();
        let _x = words.next().unwrap();
        let uid = words.next().unwrap().parse().unwrap();
        let gid = words.next().unwrap().parse().unwrap();
        let description = words.next().unwrap().to_owned();
        let _home = words.next().unwrap();
        let nologin = words.next().unwrap().ends_with("nologin");
        Self {
            name,
            uid,
            gid,
            description,
            kind: if uid == 0 {
                UserKind::Root
            } else if nologin {
                UserKind::Nologin
            } else {
                UserKind::User
            },
        }
    }
}
#[derive(Debug)]
#[allow(unused)]
pub struct GroupInfo {
    pub name: String,
    pub gid: u16,
    pub users: Vec<String>,
}
impl GroupInfo {
    fn new(line: &str) -> Self {
        let mut words = line.trim().split(":");
        let name = words.next().unwrap().to_owned();
        let _x = words.next().unwrap();
        let gid = words.next().unwrap().parse().unwrap();
        let users = words
            .next()
            .unwrap()
            .split(",")
            .map(str::to_owned)
            .collect();
        Self { name, gid, users }
    }
}

#[derive(Default, Debug)]
pub struct StringArena {
    arena: Vec<String>,
}
#[derive(Debug, Clone, Copy)]
pub struct StringArenaHandle {
    idx: usize,
}
impl StringArena {
    fn push(&mut self, s: String) -> StringArenaHandle {
        self.arena.push(s);
        StringArenaHandle {
            idx: self.arena.len() - 1,
        }
    }
    pub fn get(&self, h: StringArenaHandle) -> &str {
        &self.arena[h.idx]
    }
}
