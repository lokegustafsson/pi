use crate::Handles;
use std::{
    collections::BTreeMap,
    convert::Infallible,
    fs::File,
    io::{Read, Seek},
    path::Path,
    str::FromStr,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
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
pub struct OldSnapshot {
    pub disk_stats: Vec<DiskStats>,
    pub cpus_stat: Vec<CpuStat>,
    pub by_net_interface: BTreeMap<String, NetInterfaceSnapshot>,
}
impl Snapshot {
    pub fn retire(self) -> OldSnapshot {
        OldSnapshot {
            disk_stats: self.disk_stats,
            cpus_stat: self.cpus_stat,
            by_net_interface: self.by_net_interface,
        }
    }
    pub(crate) fn new(scratch_buf: &mut String, handles: &mut Handles) -> Self {
        fn parse<F: FromStr>(file: &mut File, scratch_buf: &mut String) -> F
        where
            F::Err: std::fmt::Debug,
        {
            file.read_to_string(scratch_buf).unwrap();
            file.rewind().unwrap();
            let ret = scratch_buf.trim().parse().unwrap();
            scratch_buf.clear();
            ret
        }

        handles.diskstats.read_to_string(scratch_buf).unwrap();
        let disk_stats = scratch_buf
            .lines()
            .map(|line| line.parse().unwrap())
            .collect();
        scratch_buf.clear();
        handles.diskstats.rewind().unwrap();

        handles.stat.read_to_string(scratch_buf).unwrap();
        let cpus_stat = scratch_buf
            .lines()
            .take_while(|line| line.starts_with("cpu"))
            .skip_while(|line| line.starts_with("cpu "))
            .map(|line| line.parse().unwrap())
            .collect();
        scratch_buf.clear();
        handles.stat.rewind().unwrap();

        Snapshot {
            disk_stats,
            mem_info: parse(&mut handles.meminfo, scratch_buf),
            partition_to_mountpath: parse(&mut handles.mounts, scratch_buf),
            cpus_stat,
            uptime: parse(&mut handles.uptime, scratch_buf),
            cpu_max_temp_millicelsius: {
                let mut ret = 0;
                for temp in &mut handles.cpu_temperatures {
                    ret = ret.max(parse(temp, scratch_buf));
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
                            rx_bytes: parse(&mut interface.rx_bytes, scratch_buf),
                            tx_bytes: parse(&mut interface.tx_bytes, scratch_buf),
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
                            mem_info_vram_used: parse(&mut gpu.mem_info_vram_used, scratch_buf),
                            mem_info_vram_total: parse(&mut gpu.mem_info_vram_total, scratch_buf),
                            mem_busy_percent: parse(&mut gpu.mem_busy_percent, scratch_buf),
                            gpu_busy_percent: parse(&mut gpu.gpu_busy_percent, scratch_buf),
                            max_temperature: {
                                let mut ret = 0;
                                for temp in &mut gpu.temperatures {
                                    ret = ret.max(parse(temp, scratch_buf));
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
