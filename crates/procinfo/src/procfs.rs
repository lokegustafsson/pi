use std::{
    fs::{self, DirEntry, File},
    io,
    path::Path,
};
use util::TextualKeyValue;

pub fn get_is_kernel_and_cmdline(pid: u32) -> Option<(bool, String)> {
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
        Some((true, format!("[{name}]")))
    } else {
        Some((false, cmdline))
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
    pub fn new(pid: u32, is_kernel: bool) -> Self {
        Self {
            file: File::open(format!("/proc/{pid}/status")).unwrap(),
            is_kernel,
        }
    }
    pub fn get_uid_gid_vm_rss_bytes(&mut self, scratch: &mut String) -> Option<(u16, u16, u64)> {
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
                (!self.is_kernel).then_some(TextualKeyValue {
                    key: "VmRSS",
                    value: &mut vm_rss_bytes,
                }),
            ],
            &read_file_to_string(&mut self.file, scratch)?,
        )?;
        Some((uid as u16, gid as u16, vm_rss_bytes))
    }
}

pub struct TidIo {
    file: File,
}
impl TidIo {
    pub fn new(pid: u32, tid: u32) -> Option<Self> {
        let mut file = match File::open(format!("/proc/{pid}/task/{tid}/io")) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => return None,
            Err(err) => panic!("{}", err),
        };
        can_read_from(&mut file).then_some(Self { file })
    }
    pub fn get_cumulative_read_write_bytes(&mut self, scratch: &mut String) -> Option<(u64, u64)> {
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
            &read_file_to_string(&mut self.file, scratch)?,
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
            file: File::open(format!("/proc/{pid}/task/{tid}/stat")).unwrap(),
        })
    }
    pub fn get_sid_cumulative_user_system_guest_time(
        &mut self,
        scratch: &mut String,
    ) -> Option<(u32, u64, u64, u64)> {
        let stat_data = read_file_to_string(&mut self.file, scratch)?;
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
    match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => panic!("{}", err),
    }
}
fn read_file_to_string<'a>(file: &mut File, scratch: &'a mut String) -> Option<&'a str> {
    use std::io::{Read, Seek};
    const ESRCH_NO_SUCH_PROCESS: i32 = 3;

    scratch.clear();
    match file.read_to_string(scratch) {
        Ok(_) => {
            file.rewind().unwrap();
            Some(&*scratch)
        }
        Err(err) if err.raw_os_error() == Some(ESRCH_NO_SUCH_PROCESS) => None,
        Err(err) => panic!("{}", err),
    }
}
fn can_read_from(file: &mut File) -> bool {
    use std::io::{Read, Seek};
    let mut buf: &mut [u8] = &mut [0u8];
    let ret = file.read(&mut buf).is_ok();
    file.rewind().unwrap();
    ret
}
