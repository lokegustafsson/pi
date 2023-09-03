use std::{
    io,
    collections::BTreeMap,
    fs::{self, DirEntry, File},
    path::Path,
    process::Command,
};
use util::TextualKeyValue;

pub struct ProcIngest {
    pub by_pid: BTreeMap<u32, ProcessIngest>,
}
pub struct ProcessIngest {
    pub kernel: bool,
    pub cmdline: String,
    pub by_tid: BTreeMap<u32, ThreadIngest>,

    pub status: File,
    pub uid: u16,
    pub gid: u16,
    pub vm_rss_bytes: u64,
}
pub struct ThreadIngest {
    /// Sometimes requires `PTRACE_MODE_READ_FSCREDS`.
    pub io: Option<File>,
    cumulative_read_bytes: u64,
    cumulative_write_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,

    pub stat: File,
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

        let mut ret = Self {
            by_pid: BTreeMap::new(),
        };
        ret.update();
        ret
    }
    pub fn update(&mut self) {
        self.by_pid = read_dir("/proc")
            .filter_map(direntry_as_u32)
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
            let (kernel, cmdline) = {
                let cmdline = read_to_string(format!("/proc/{pid}/cmdline"))?;
                if cmdline.is_empty() {
                    let status = read_to_string(format!("/proc/{pid}/status"))?;
                    let name = status
                        .lines()
                        .next()
                        .unwrap()
                        .strip_prefix("Name:\t")
                        .unwrap()
                        .trim();
                    (true, format!("[{name}]"))
                } else {
                    (false, cmdline)
                }
            };
            Some(Self {
                kernel,
                cmdline,
                by_tid: BTreeMap::new(),
                status: File::open(format!("/proc/{pid}/status")).unwrap(),
                uid: 0,
                gid: 0,
                vm_rss_bytes: 0,
            })
        })?;
        let mut uid = 0;
        let mut gid = 0;
        let mut vm_rss_bytes = 0;
        TextualKeyValue::extract_from(
            &mut [
                Some(TextualKeyValue {
                    key: "Uid",
                    value: &mut uid,
                }),
                Some(TextualKeyValue {
                    key: "Gid",
                    value: &mut gid,
                }),
                (!old.kernel).then_some(TextualKeyValue {
                    key: "VmRSS",
                    value: &mut vm_rss_bytes,
                }),
            ],
            &read_file_to_string(&mut old.status)?,
        );
        Some(ProcessIngest {
            kernel: old.kernel,
            cmdline: old.cmdline,
            by_tid: ThreadIngest::new_by_tid(pid, old.by_tid)?,
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
    ) -> Option<BTreeMap<u32, ThreadIngest>> {
        let mut ret = BTreeMap::new();
        for tid in
            read_dir(format!("/proc/{pid}/task")).map(|entry| direntry_as_u32(entry).unwrap())
        {
            let mut old = old.remove(&tid).unwrap_or_else(|| ThreadIngest {
                io: File::open(format!("/proc/{pid}/task/{tid}/io"))
                    .ok()
                    .and_then(|mut file| can_read_from(&mut file).then_some(file)),
                cumulative_read_bytes: 0,
                cumulative_write_bytes: 0,
                read_bytes: 0,
                write_bytes: 0,
                stat: File::open(format!("/proc/{pid}/task/{tid}/stat")).unwrap(),
                sid: 0,
                cumulative_user_time_ms: 0,
                cumulative_system_time_ms: 0,
                cumulative_guest_time_ms: 0,
                user_time_ms: 0,
                system_time_ms: 0,
                guest_time_ms: 0,
            });
            let mut cumulative_read_bytes = 0;
            let mut cumulative_write_bytes = 0;
            if let Some(io) = old.io.as_mut() {
                TextualKeyValue::extract_from(
                    &mut [
                        Some(TextualKeyValue {
                            key: "read_bytes",
                            value: &mut cumulative_read_bytes,
                        }),
                        Some(TextualKeyValue {
                            key: "write_bytes",
                            value: &mut cumulative_write_bytes,
                        }),
                    ],
                    &read_file_to_string(io)?,
                );
            }

            let stat_data = read_file_to_string(&mut old.stat)?;
            let mut stat_entries = stat_data.split(' ');
            let sid = stat_entries.nth(5).unwrap().parse().unwrap();
            let cumulative_user_time_ms = stat_entries.nth(7).unwrap().parse::<u64>().unwrap() * 10;
            let cumulative_system_time_ms =
                stat_entries.nth(0).unwrap().parse::<u64>().unwrap() * 10;
            let cumulative_guest_time_ms =
                stat_entries.nth(28).unwrap().parse::<u64>().unwrap() * 10;
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

fn direntry_as_u32(entry: DirEntry) -> Option<u32> {
    entry.file_name().to_str().unwrap().parse::<u32>().ok()
}
fn read_dir(path: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    fs::read_dir(path).unwrap().map(|entry| entry.unwrap())
}
fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => panic!("{}", err),
    }
}
fn read_file_to_string(file: &mut File) -> Option<String> {
    use std::io::{Read, Seek};
    let mut ret = String::new();
    const ESRCH_NO_SUCH_PROCESS: i32 = 3;
    match file.read_to_string(&mut ret) {
        Ok(_) => {}
        Err(err) if err.raw_os_error() == Some(ESRCH_NO_SUCH_PROCESS) => return None,
        Err(err) => panic!("{}", err),
    }
    file.rewind().unwrap();
    Some(ret)
}
fn can_read_from(file: &mut File) -> bool {
    use std::io::{Read, Seek};
    let mut buf: &mut [u8] = &mut [0u8];
    let ret = file.read(&mut buf).is_ok();
    file.rewind().unwrap();
    ret
}
