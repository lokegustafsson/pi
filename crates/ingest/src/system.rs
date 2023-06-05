use crate::{
    snapshot::{CpuStat, Snapshot},
    HISTORY,
};
use std::collections::VecDeque;

#[derive(Default, Debug)]
pub struct SystemInfo {
    pub ticks: VecDeque<SystemInfoTick>,
    old_cpus_stat: Vec<CpuStat>,
}
#[derive(Debug)]
pub struct SystemInfoTick {
    pub mem_total: u64,
    pub mem_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub avg_cpu: SystemInfoTickCpu,
    pub cpus: Vec<SystemInfoTickCpu>,
}
#[derive(Debug)]
pub struct SystemInfoTickCpu {
    user: f32,
    system: f32,
    guest: f32,
}
impl SystemInfoTickCpu {
    pub fn total(&self) -> f32 {
        self.user + self.system + self.guest
    }
}
impl SystemInfo {
    pub fn update(&mut self, new: &Snapshot) {
        while self.ticks.len() >= HISTORY {
            self.ticks.pop_front();
        }
        if self.old_cpus_stat.is_empty() {
            assert!(self.ticks.is_empty());
        } else {
            self.ticks
                .push_back(SystemInfoTick::new(&self.old_cpus_stat, new));
        }
        self.old_cpus_stat = new.cpus_stat.clone();
    }
}
impl SystemInfoTick {
    fn new(old_cpus_stat: &[CpuStat], new: &Snapshot) -> Self {
        let cpu = |old: &CpuStat, new: &CpuStat, multiple: usize| {
            let user = (new.user - old.user) as f32;
            let idle = (new.idle - old.idle) as f32;
            let system = (new.system - old.system) as f32;
            let guest = (new.guest - old.guest) as f32;
            let total = user + idle + system + guest;
            let total = if total > 0.0 { total } else { 1.0 };
            let multiple = multiple as f32;
            SystemInfoTickCpu {
                user: user / total * multiple,
                system: system / total * multiple,
                guest: guest / total * multiple,
            }
        };
        let num_cpu = new.cpus_stat.len() - 1;
        Self {
            mem_total: 1024 * new.mem_info.mem_total,
            mem_used: 1024 * (new.mem_info.mem_total - new.mem_info.mem_available),
            swap_total: 1024 * new.mem_info.swap_total,
            swap_used: 1024 * (new.mem_info.swap_total - new.mem_info.swap_free),
            avg_cpu: cpu(&old_cpus_stat[0], &new.cpus_stat[0], num_cpu),
            cpus: Iterator::zip(old_cpus_stat.iter(), new.cpus_stat.iter())
                .skip(1)
                .map(|(old, new)| cpu(old, new, 1))
                .collect(),
        }
    }
}
