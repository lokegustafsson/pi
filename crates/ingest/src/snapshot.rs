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
