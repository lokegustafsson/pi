use nix::errno::Errno;
use std::{
    fs::{self, DirEntry, File},
    io,
    path::Path,
};
use util::TextualKeyValue;

pub fn get_is_kernel_name_cmdline(pid: u32) -> Option<(bool, String, Option<String>)> {
    let cmdline = read_to_string(format!("/proc/{pid}/cmdline"))?;
    let status = read_to_string(format!("/proc/{pid}/status"))?;
    let name = status
        .lines()
        .next()
        .unwrap()
        .strip_prefix("Name:\t")
        .unwrap()
        .trim();
    if cmdline.is_empty() {
        Some((true, format!("[{name}]"), None))
    } else {
        Some((false, name.to_owned(), Some(cmdline.replace('\0', " "))))
    }
}

pub fn get_live_tids(pid: u32) -> impl Iterator<Item = u32> {
    read_dir(format!("/proc/{pid}/task")).map(|entry| direntry_as_u32(entry).unwrap())
}
pub fn get_live_pids() -> impl Iterator<Item = u32> {
    read_dir("/proc").filter_map(direntry_as_u32)
}

pub struct PidStatus {
    file: File,
    is_kernel: bool,
}
impl PidStatus {
    pub fn new(pid: u32, is_kernel: bool) -> Option<Self> {
        Some(Self {
            file: File::open(format!("/proc/{pid}/status"))
                .map_err(check_io_err)
                .ok()?,
            is_kernel,
        })
    }
    pub fn get_uid_gid_vm_rss_kb_threads(&mut self) -> Option<(u16, u16, u64, u32)> {
        let mut uid = 0;
        let mut gid = 0;
        let mut vm_rss_kb = 0;
        let mut threads = 0;
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
                (!self.is_kernel).then_some(TextualKeyValue {
                    key: "VmRSS",
                    value: &mut vm_rss_kb,
                }),
                Some(TextualKeyValue {
                    key: "Threads",
                    value: &mut threads,
                }),
            ],
            &read_file_to_string(&mut self.file, &mut [0u8; 4096])?,
        )?;
        Some((uid as u16, gid as u16, vm_rss_kb, threads as u32))
    }
}

pub struct TidIo {
    file: File,
}
impl TidIo {
    pub fn new(pid: u32, tid: u32) -> Option<Self> {
        let mut file = match File::open(format!("/proc/{pid}/task/{tid}/io")) {
            Ok(file) => file,
            Err(err) => {
                check_io_err(err);
                return None;
            }
        };
        can_read_from(&mut file).then_some(Self { file })
    }
    pub fn get_cumulative_read_write_bytes(&mut self) -> Option<(u64, u64)> {
        let mut cumulative_read_bytes = 0;
        let mut cumulative_write_bytes = 0;
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
            &read_file_to_string(&mut self.file, &mut [0u8; 4096])?,
        )?;
        Some((cumulative_read_bytes, cumulative_write_bytes))
    }
}

pub struct TidStat {
    file: File,
}
impl TidStat {
    pub fn new(pid: u32, tid: u32) -> Option<Self> {
        Some(Self {
            file: match File::open(format!("/proc/{pid}/task/{tid}/stat")) {
                Ok(file) => file,
                Err(err) => {
                    check_io_err(err);
                    return None;
                }
            },
        })
    }
    pub fn get_sid_cumulative_user_system_guest_time(&mut self) -> Option<(u32, u64, u64, u64)> {
        let buf = &mut [0u8; 4096];
        let stat_data = read_file_to_string(&mut self.file, buf)?;
        let mut stat_entries = stat_data.split(' ');
        let sid = stat_entries.nth(5).unwrap().parse().unwrap();
        let cumulative_user_time_ms = stat_entries.nth(7).unwrap().parse::<u64>().unwrap() * 10;
        let cumulative_system_time_ms = stat_entries.nth(0).unwrap().parse::<u64>().unwrap() * 10;
        let cumulative_guest_time_ms = stat_entries.nth(28).unwrap().parse::<u64>().unwrap() * 10;
        Some((
            sid,
            cumulative_user_time_ms,
            cumulative_system_time_ms,
            cumulative_guest_time_ms,
        ))
    }
}

fn direntry_as_u32(entry: DirEntry) -> Option<u32> {
    entry.file_name().to_str().unwrap().parse::<u32>().ok()
}
fn read_dir(path: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    fs::read_dir(path).unwrap().map(|entry| entry.unwrap())
}
fn read_to_string(path: impl AsRef<Path>) -> Option<String> {
    let mut file = File::open(path).ok()?;
    Some(read_file_to_string(&mut file, &mut [0u8; 4096])?.to_owned())
}
fn read_file_to_string<'a>(file: &mut File, buf: &'a mut [u8; 4096]) -> Option<&'a str> {
    // For performance reasons, we assume that read returns the entire (tiny, <4K) file.
    match nix::sys::uio::pread(file, buf, 0) {
        Ok(len) => Some(std::str::from_utf8(&buf[..len]).unwrap()),
        Err(Errno::ENOENT | Errno::ESRCH | Errno::EACCES) => None,
        Err(other) => panic!("{other}"),
    }
}
fn can_read_from(file: &mut File) -> bool {
    read_file_to_string(file, &mut [0u8; 4096]).is_some()
}

#[track_caller]
fn check_io_err(err: io::Error) {
    if err.kind() == io::ErrorKind::NotFound || err.kind() == io::ErrorKind::PermissionDenied {
        return;
    }
    panic!("{err}");
}
