use crate::Handles;
use std::{
    convert::Infallible,
    io::{Read, Seek},
    str::FromStr,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub mem_info: MemInfo,
    pub cpus_stat: Vec<CpuStat>,
}
impl Snapshot {
    pub(crate) fn new(scratch_buf: &mut String, handles: &mut Handles) -> Self {
        handles.meminfo.read_to_string(scratch_buf).unwrap();
        let mem_info = scratch_buf.parse().unwrap();
        scratch_buf.clear();
        handles.meminfo.rewind().unwrap();

        handles.stat.read_to_string(scratch_buf).unwrap();
        let cpus_stat = scratch_buf
            .lines()
            .take_while(|line| line.starts_with("cpu"))
            .map(|line| line.parse::<CpuStat>().unwrap())
            .collect();
        scratch_buf.clear();
        handles.stat.rewind().unwrap();

        Snapshot {
            mem_info,
            cpus_stat,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemInfo {
    pub mem_total: u64,
    pub mem_free: u64,
    pub mem_available: u64,
    pub cached: u64,
    pub swap_cached: u64,
    pub active: u64,
    pub inactive: u64,
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
            cached: 0,
            swap_cached: 0,
            active: 0,
            inactive: 0,
            swap_total: 0,
            swap_free: 0,
        };
        let l = s.lines();
        let l = extract(l, &mut ret.mem_total, "MemTotal:");
        let l = extract(l, &mut ret.mem_free, "MemFree:");
        let l = extract(l, &mut ret.mem_available, "MemAvailable:");
        let l = extract(l, &mut ret.cached, "Cached:");
        let l = extract(l, &mut ret.swap_cached, "SwapCached:");
        let l = extract(l, &mut ret.active, "Active:");
        let l = extract(l, &mut ret.inactive, "Inactive:");
        let l = extract(l, &mut ret.swap_total, "SwapTotal:");
        let _ = extract(l, &mut ret.swap_free, "SwapFree:");
        Ok(ret)
    }
}
#[derive(Clone, Debug, PartialEq)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mem_info() {
        let raw = r#"MemTotal:       131860364 kB
MemFree:        122033496 kB
MemAvailable:   126333412 kB
Buffers:            2680 kB
Cached:          5209628 kB
SwapCached:            0 kB
Active:          6133200 kB
Inactive:        2724488 kB
Active(anon):    3652696 kB
Inactive(anon):    37860 kB
Active(file):    2480504 kB
Inactive(file):  2686628 kB
Unevictable:          32 kB
Mlocked:              32 kB
SwapTotal:             0 kB
SwapFree:              0 kB
Zswap:                 0 kB
Zswapped:              0 kB
Dirty:               116 kB
Writeback:             0 kB
AnonPages:       3642604 kB
Mapped:           751492 kB
Shmem:             45176 kB
KReclaimable:     363824 kB
Slab:             539976 kB
SReclaimable:     363824 kB
SUnreclaim:       176152 kB
KernelStack:       19472 kB
PageTables:        35480 kB
SecPageTables:         0 kB
NFS_Unstable:          0 kB
Bounce:                0 kB
WritebackTmp:          0 kB
CommitLimit:    65930180 kB
Committed_AS:   10106716 kB
VmallocTotal:   34359738367 kB
VmallocUsed:       73284 kB
VmallocChunk:          0 kB
Percpu:            25344 kB
AnonHugePages:     10240 kB
ShmemHugePages:        0 kB
ShmemPmdMapped:        0 kB
FileHugePages:         0 kB
FilePmdMapped:         0 kB
CmaTotal:              0 kB
CmaFree:               0 kB
HugePages_Total:       0
HugePages_Free:        0
HugePages_Rsvd:        0
HugePages_Surp:        0
Hugepagesize:       2048 kB
Hugetlb:               0 kB
DirectMap4k:      620060 kB
DirectMap2M:    12953600 kB
DirectMap1G:    120586240 kB
"#;
        let parsed = MemInfo {
            mem_total: 131860364,
            mem_free: 122033496,
            mem_available: 126333412,
            cached: 5209628,
            swap_cached: 0,
            active: 6133200,
            inactive: 2724488,
            swap_total: 0,
            swap_free: 0,
        };
        assert_eq!(Ok::<MemInfo, Infallible>(parsed), raw.parse());
    }

    #[test]
    fn cpu_stat() {
        let raw = "cpu3 3417 151 2626 706482 159 0 8 0 0 0";
        let parsed = CpuStat {
            user: 3417 + 151,
            system: 2626 + 8 + 0 + 0,
            idle: 706482,
            guest: 0 + 0,
        };
        assert_eq!(Ok::<CpuStat, Infallible>(parsed), raw.parse());
    }
}
