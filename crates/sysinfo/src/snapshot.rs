use crate::SysHandles;
use std::{
    collections::BTreeMap, convert::Infallible, fs::File, path::Path, str::FromStr, time::Duration,
};

#[derive(Clone, Debug)]
pub struct SysSnapshot {
    pub disk_stats: Vec<DiskStats>,
    pub mem_info: MemInfo,
    pub partition_to_mountpath: PartitionToMountpath,
    pub cpus_stat: Vec<CpuStat>,
    pub uptime: Uptime,
    pub cpu_max_temp_millicelsius: u32,
    pub by_net_interface: BTreeMap<String, NetInterfaceSnapshot>,
    pub by_gpu: BTreeMap<String, GpuSnapshot>,
}
#[derive(Clone, Debug)]
pub struct NetInterfaceSnapshot {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}
#[derive(Clone, Debug)]
pub struct GpuSnapshot {
    pub mem_info_vram_used: u64,
    pub mem_info_vram_total: u64,
    pub mem_busy_percent: u16,
    pub gpu_busy_percent: u16,
    pub max_temperature: u32,
}
#[derive(Clone, Debug)]
pub struct SysOldSnapshot {
    pub disk_stats: Vec<DiskStats>,
    pub cpus_stat: Vec<CpuStat>,
    pub by_net_interface: BTreeMap<String, NetInterfaceSnapshot>,
}
impl SysSnapshot {
    pub fn retire(self) -> SysOldSnapshot {
        SysOldSnapshot {
            disk_stats: self.disk_stats,
            cpus_stat: self.cpus_stat,
            by_net_interface: self.by_net_interface,
        }
    }
    pub fn new(handles: &mut SysHandles) -> Self {
        fn parse<F: FromStr>(file: &mut File) -> F
        where
            F::Err: std::fmt::Debug,
        {
            let buf = &mut [0u8; 8196];
            let s = read_file_to_string(file, buf);
            let ret = s.trim().parse().unwrap();
            ret
        }

        let buf = &mut [0u8; 8196];
        let data = read_file_to_string(&mut handles.diskstats, buf);
        let disk_stats = data.lines().map(|line| line.parse().unwrap()).collect();

        let data = read_file_to_string(&mut handles.stat, buf);
        let cpus_stat = data
            .lines()
            .take_while(|line| line.starts_with("cpu"))
            .skip_while(|line| line.starts_with("cpu "))
            .map(|line| line.parse().unwrap())
            .collect();

        SysSnapshot {
            disk_stats,
            mem_info: parse(&mut handles.meminfo),
            partition_to_mountpath: parse(&mut handles.mounts),
            cpus_stat,
            uptime: parse(&mut handles.uptime),
            cpu_max_temp_millicelsius: {
                let mut ret = 0;
                for temp in &mut handles.cpu_temperatures {
                    ret = ret.max(parse(temp));
                }
                ret
            },
            by_net_interface: handles
                .by_net_interface
                .iter_mut()
                .map(|(name, interface)| {
                    (
                        name.to_owned(),
                        NetInterfaceSnapshot {
                            rx_bytes: parse(&mut interface.rx_bytes),
                            tx_bytes: parse(&mut interface.tx_bytes),
                        },
                    )
                })
                .collect(),
            by_gpu: handles
                .by_gpu
                .iter_mut()
                .map(|(name, gpu)| {
                    (
                        name.to_owned(),
                        GpuSnapshot {
                            mem_info_vram_used: parse(&mut gpu.mem_info_vram_used),
                            mem_info_vram_total: parse(&mut gpu.mem_info_vram_total),
                            mem_busy_percent: parse(&mut gpu.mem_busy_percent),
                            gpu_busy_percent: parse(&mut gpu.gpu_busy_percent),
                            max_temperature: {
                                let mut ret = 0;
                                for temp in &mut gpu.temperatures {
                                    ret = ret.max(parse(temp));
                                }
                                ret
                            },
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct DiskStats {
    major_device_number: u16,
    pub minor_device_number: u16,
    pub device_name: String,
    reads_completed: u64,
    reads_merged: u64,
    pub sectors_read: u64,
    time_spent_reading: Duration,
    writes_completed: u64,
    writes_merged: u64,
    pub sectors_written: u64,
    time_spent_writing: Duration,
    io_currently_in_progress: u32,
    time_spent_io: Duration,
    weighted_time_spent_io: Duration,
    discards_completed: u64,
    discards_merged: u64,
    pub sectors_discarded: u64,
    time_spent_discarding: Duration,
    flush_requests_completed: u64,
    time_spent_flushing: Duration,
}
impl FromStr for DiskStats {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = s.split_ascii_whitespace();
        Ok(Self {
            major_device_number: words.next().unwrap().parse().unwrap(),
            minor_device_number: words.next().unwrap().parse().unwrap(),
            device_name: words.next().unwrap().to_owned(),
            reads_completed: words.next().unwrap().parse().unwrap(),
            reads_merged: words.next().unwrap().parse().unwrap(),
            sectors_read: words.next().unwrap().parse().unwrap(),
            time_spent_reading: Duration::from_millis(words.next().unwrap().parse().unwrap()),
            writes_completed: words.next().unwrap().parse().unwrap(),
            writes_merged: words.next().unwrap().parse().unwrap(),
            sectors_written: words.next().unwrap().parse().unwrap(),
            time_spent_writing: Duration::from_millis(words.next().unwrap().parse().unwrap()),
            io_currently_in_progress: words.next().unwrap().parse().unwrap(),
            time_spent_io: Duration::from_millis(words.next().unwrap().parse().unwrap()),
            weighted_time_spent_io: Duration::from_millis(words.next().unwrap().parse().unwrap()),
            discards_completed: words.next().unwrap().parse().unwrap(),
            discards_merged: words.next().unwrap().parse().unwrap(),
            sectors_discarded: words.next().unwrap().parse().unwrap(),
            time_spent_discarding: Duration::from_millis(words.next().unwrap().parse().unwrap()),
            flush_requests_completed: words.next().unwrap().parse().unwrap(),
            time_spent_flushing: Duration::from_millis(words.next().unwrap().parse().unwrap()),
        })
    }
}

#[derive(Clone, Debug)]
pub struct MemInfo {
    pub mem_total: u64,
    pub mem_free: u64,
    pub mem_available: u64,
    pub swap_total: u64,
    pub swap_free: u64,
}
impl FromStr for MemInfo {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn extract<'a, I: Iterator<Item = &'a str>>(
            lines: I,
            field: &mut u64,
            crib: &'static str,
        ) -> impl Iterator<Item = &'a str> {
            let mut lines = lines.skip_while(move |line| !line.starts_with(crib));
            let line = lines
                .next()
                .unwrap_or_else(move || panic!("cannot find {crib}"));
            let kb: u64 = line[crib.len()..(line.len() - 2)].trim().parse().unwrap();
            *field = kb;
            lines
        }
        let mut ret = MemInfo {
            mem_total: 0,
            mem_free: 0,
            mem_available: 0,
            swap_total: 0,
            swap_free: 0,
        };
        let l = s.lines();
        let l = extract(l, &mut ret.mem_total, "MemTotal:");
        let l = extract(l, &mut ret.mem_free, "MemFree:");
        let l = extract(l, &mut ret.mem_available, "MemAvailable:");
        let l = extract(l, &mut ret.swap_total, "SwapTotal:");
        let l = extract(l, &mut ret.swap_free, "SwapFree:");
        let _ = l;
        Ok(ret)
    }
}

#[derive(Clone, Debug)]
pub struct PartitionToMountpath {
    pub partition_to_mountpath: BTreeMap<String, String>,
}
impl FromStr for PartitionToMountpath {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut partition_to_mountpath = BTreeMap::new();
        for line in s.lines() {
            let mut words = line.split_ascii_whitespace();
            let device = words.next().unwrap();
            let mountpath = words.next().unwrap();
            if device.starts_with("/dev") {
                let device = Path::new(device)
                    .canonicalize()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                partition_to_mountpath.insert(device, mountpath.to_owned());
            }
        }
        Ok(Self {
            partition_to_mountpath,
        })
    }
}

#[derive(Clone, Debug)]
pub struct CpuStat {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub guest: u64,
}
impl FromStr for CpuStat {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ret = Self {
            user: 0,
            system: 0,
            idle: 0,
            guest: 0,
        };
        let mut words = s
            .split(' ')
            .skip_while(|w| w.is_empty() || w.starts_with("cpu"));
        ret.user += words.next().unwrap().parse::<u64>().unwrap();
        ret.user += words.next().unwrap().parse::<u64>().unwrap();
        ret.system += words.next().unwrap().parse::<u64>().unwrap();
        ret.idle += words.next().unwrap().parse::<u64>().unwrap();
        words.next();
        ret.system += words.next().unwrap().parse::<u64>().unwrap();
        ret.system += words.next().unwrap().parse::<u64>().unwrap();
        ret.system += words.next().unwrap().parse::<u64>().unwrap();
        ret.guest += words.next().unwrap().parse::<u64>().unwrap();
        ret.guest += words.next().unwrap().parse::<u64>().unwrap();
        Ok(ret)
    }
}

#[derive(Clone, Debug)]
pub struct Uptime {
    pub since_boot: Duration,
    pub idle_cpu_since_boot: Duration,
}
impl FromStr for Uptime {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut words = s.split_ascii_whitespace();
        Ok(Self {
            since_boot: Duration::from_secs_f64(words.next().unwrap().parse().unwrap()),
            idle_cpu_since_boot: Duration::from_secs_f64(words.next().unwrap().parse().unwrap()),
        })
    }
}

fn read_file_to_string<'a>(file: &mut File, buf: &'a mut [u8; 8196]) -> &'a str {
    // For performance reasons, we assume that read returns the entire (tiny, <8K) file.
    let len = nix::sys::uio::pread(file, buf, 0).unwrap();
    assert!(len < 8196);
    std::str::from_utf8(&buf[..len]).unwrap()
}
