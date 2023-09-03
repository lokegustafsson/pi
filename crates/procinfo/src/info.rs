use crate::ingest::ProcIngest;
use std::{
    collections::HashMap,
    fs,
    ops::{Add, AddAssign},
};

#[derive(Debug)]
pub struct ProcInfo {
    update_hz: u8,
    pub uid_to_user: HashMap<u16, UserInfo>,
    pub gid_to_group: HashMap<u16, GroupInfo>,
    pub login_sessions: Vec<LoginSessionInfo>,
    pub sessions: Vec<SessionInfo>,
    pub processes: Vec<ProcessInfo>,
    pub threads: Vec<ThreadInfo>,
}
#[derive(Debug)]
pub struct LoginSessionInfo {
    pub lsid: Lsid,
    pub stat: ProcStat,
}
#[derive(Debug)]
pub enum Lsid {
    Kernel,
    SystemdServices,
    SystemdSession(u16),
}
#[derive(Debug)]
pub struct SessionInfo {
    pub parent_lsid: Lsid,
    pub sid: u32,
    pub stat: ProcStat,
}
#[derive(Debug)]
pub struct ProcessInfo {
    pub parent_sid: u32,
    pub pid: u32,
    pub uid: u16,
    pub gid: u16,
    pub cmdline: String,
    pub stat: ProcStat,
}
#[derive(Debug)]
pub struct ThreadInfo {
    pub parent_pid: u32,
    pub tid: u32,
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
            uid_to_user,
            gid_to_group,

            login_sessions: Vec::new(),
            sessions: Vec::new(),
            processes: Vec::new(),
            threads: Vec::new(),
        }
    }
    pub fn update(&mut self, src: &ProcIngest) {
        self.login_sessions = Vec::new();
        self.sessions = Vec::new();
        self.processes = Vec::new();
        self.threads = Vec::new();
        for (&pid, process) in &src.by_pid {
            let thread_start_idx = self.threads.len();
            for (&tid, thread) in &process.by_tid {
                self.threads.push(ThreadInfo {
                    parent_pid: pid,
                    tid,
                    stat: ProcStat {
                        guest_time_millis: thread.guest_time_ms,
                        user_time_millis: thread.user_time_ms,
                        system_time_millis: thread.system_time_ms,
                        disk_read_bytes_per_second: thread.read_bytes / self.update_hz as u64,
                        disk_write_bytes_per_second: thread.write_bytes / self.update_hz as u64,
                        mem_bytes: process.vm_rss_bytes,
                    },
                });
            }
            self.processes.push(ProcessInfo {
                parent_sid: process.by_tid.first_key_value().unwrap().1.sid,
                pid,
                uid: process.uid,
                gid: process.gid,
                cmdline: process.cmdline.clone(),
                stat: {
                    let mut stat = self.threads[thread_start_idx..]
                        .iter()
                        .map(|thread| thread.stat)
                        .fold(ProcStat::ZERO, ProcStat::add);
                    stat.mem_bytes = process.vm_rss_bytes;
                    stat
                },
            });
        }
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
                    stat: ProcStat::ZERO,
                });
            }
            self.sessions.last_mut().unwrap().stat += p.stat;
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
