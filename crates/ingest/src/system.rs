use crate::{
    snapshot::{CpuStat, DiskStats, GpuSnapshot, NetInterfaceSnapshot, OldSnapshot, Snapshot},
    Series, SUBSEC,
};
use std::{collections::BTreeMap, time::Duration};

#[derive(Default, Debug)]
pub struct SystemInfo {
    pub global: GlobalInfo,
    pub by_cpu: Vec<CpuInfo>,
    pub total_cpu: CpuInfo,
    pub by_partition: BTreeMap<String, PartitionInfo>,
    pub total_partition: PartitionInfo,
    pub by_net_interface: BTreeMap<String, NetInterfaceInfo>,
    pub total_net: NetInterfaceInfo,
    pub by_gpu: BTreeMap<String, GpuInfo>,
    pub total_gpu: GpuInfo,
}
#[derive(Default, Debug)]
pub struct GlobalInfo {
    pub mem_total: f64,
    pub swap_total: f64,
    pub mem_reclaimable: Series<f64>,
    pub mem_used: Series<f64>,
    pub swap_used: Series<f64>,
    pub cpu_max_temp: Series<f64>,
    pub uptime: Duration,
    pub uptime_cpu_busy: Duration,
}
#[derive(Clone, Default, Debug)]
pub struct CpuInfo {
    wma_total: WindowMovingAverage,
    wma_user: WindowMovingAverage,
    wma_system: WindowMovingAverage,
    wma_guest: WindowMovingAverage,
    pub total: Series<f64>,
    pub user: Series<f64>,
    pub system: Series<f64>,
    pub guest: Series<f64>,
}
#[derive(Default, Debug)]
pub struct PartitionInfo {
    pub capacity: f64,
    pub used: f64,
    pub read: Series<f64>,
    pub written: Series<f64>,
    pub discarded: Series<f64>,
}
#[derive(Default, Debug)]
pub struct NetInterfaceInfo {
    wma_rx: WindowMovingAverage,
    wma_tx: WindowMovingAverage,
    pub rx: Series<f64>,
    pub tx: Series<f64>,
}
#[derive(Default, Debug)]
pub struct GpuInfo {
    wma_gpu_busy: WindowMovingAverage,
    pub vram_total: f64,
    pub vram_used: Series<f64>,
    pub vram_busy: Series<f64>,
    pub gpu_busy: Series<f64>,
    pub max_temperature: Series<f64>,
}
impl SystemInfo {
    pub(crate) fn update(&mut self, new: &Snapshot, old: &OldSnapshot) {
        self.global.update(new);

        CpuInfo::update_all(&mut self.by_cpu, &new.cpus_stat, &old.cpus_stat);
        self.total_cpu.push_sum_of_others(&self.by_cpu);

        PartitionInfo::update_all(&mut self.by_partition, new, old);
        self.total_partition
            .push_sum_of_others(self.by_partition.values());

        intersect_old_new(
            &mut self.by_net_interface,
            old.by_net_interface.iter(),
            new.by_net_interface.iter(),
            NetInterfaceInfo::update,
        );
        self.total_net
            .push_sum_of_others(self.by_net_interface.values());

        {
            self.by_gpu.retain(|k, _| new.by_gpu.contains_key(k));
            for (name, new) in &new.by_gpu {
                self.by_gpu
                    .entry(name.to_owned())
                    .or_insert_with(GpuInfo::default)
                    .update(new)
            }
        }
        self.total_gpu.push_sum_of_others(self.by_gpu.values());
    }
}
impl GlobalInfo {
    fn update(&mut self, new: &Snapshot) {
        self.mem_total = 1024.0 * new.mem_info.mem_total as f64;
        self.swap_total = 1024.0 * new.mem_info.swap_total as f64;
        self.mem_reclaimable
            .push(1024.0 * (new.mem_info.mem_available - new.mem_info.mem_free) as f64);
        self.mem_used
            .push(1024.0 * (new.mem_info.mem_total - new.mem_info.mem_available) as f64);
        self.swap_used
            .push(1024.0 * (new.mem_info.swap_total - new.mem_info.swap_free) as f64);
        self.cpu_max_temp
            .push(new.cpu_max_temp_millicelsius as f64 / 1e3);
        self.uptime = new.uptime.since_boot;
        self.uptime_cpu_busy =
            new.uptime.since_boot - new.uptime.idle_cpu_since_boot / new.cpus_stat.len() as u32;
    }
}
impl CpuInfo {
    fn update_all(by_cpu: &mut Vec<Self>, new: &[CpuStat], old: &[CpuStat]) {
        by_cpu.resize_with(new.len(), Self::default);
        for i in 0..new.len() {
            by_cpu[i].update(&new[i], old.get(i).unwrap_or(&new[i]));
        }
    }
    fn update(&mut self, new: &CpuStat, old: &CpuStat) {
        let user = (new.user - old.user) as f64;
        let idle = (new.idle - old.idle) as f64;
        let system = (new.system - old.system) as f64;
        let guest = (new.guest - old.guest) as f64;
        let busy = user + system + guest;
        let total = if busy + idle > 0.0 { busy + idle } else { 1.0 };
        self.total.push(self.wma_total.add_sample(busy / total));
        self.user.push(self.wma_user.add_sample(user / total));
        self.system.push(self.wma_system.add_sample(system / total));
        self.guest.push(self.wma_guest.add_sample(guest / total));
    }
    fn push_sum_of_others(&mut self, others: &[Self]) {
        let mut total = 0.0;
        let mut user = 0.0;
        let mut system = 0.0;
        let mut guest = 0.0;
        for other in others {
            total += other.total.latest();
            user += other.user.latest();
            system += other.system.latest();
            guest += other.guest.latest();
        }
        self.total.push(total);
        self.user.push(user);
        self.system.push(system);
        self.guest.push(guest);
    }
}
impl PartitionInfo {
    fn update_all(by_partition: &mut BTreeMap<String, Self>, new: &Snapshot, old: &OldSnapshot) {
        intersect_old_new(
            by_partition,
            old.disk_stats
                .iter()
                .filter(|stats| {
                    let is_partition = stats.minor_device_number != 0;
                    is_partition
                })
                .map(|stats| (&stats.device_name, stats)),
            new.disk_stats
                .iter()
                .map(|stats| (&stats.device_name, stats)),
            |ret, old_stats, new_stats| {
                ret.update(
                    new.partition_to_mountpath
                        .partition_to_mountpath
                        .get(&new_stats.device_name),
                    old_stats,
                    new_stats,
                )
            },
        );
    }
    fn update(&mut self, mountpath: Option<&String>, old: &DiskStats, new: &DiskStats) {
        if let Some(path) = mountpath {
            let statfs = nix::sys::statfs::statfs(path.as_str()).unwrap();
            self.capacity = (statfs.block_size() as u64 * statfs.blocks()) as f64;
            self.used =
                (statfs.block_size() as u64 * (statfs.blocks() - statfs.blocks_available())) as f64;
        }
        self.read
            .push(512.0 * (new.sectors_read - old.sectors_read) as f64);
        self.written
            .push(512.0 * (new.sectors_written - old.sectors_written) as f64);
        self.discarded
            .push(512.0 * (new.sectors_discarded - old.sectors_discarded) as f64);
    }
    fn push_sum_of_others<'a>(&mut self, others: impl Iterator<Item = &'a Self>) {
        self.capacity = 0.0;
        self.used = 0.0;
        let mut read = 0.0;
        let mut written = 0.0;
        let mut discarded = 0.0;
        for other in others {
            self.capacity += other.capacity;
            self.used += other.used;
            read += other.read.latest();
            written += other.written.latest();
            discarded += other.discarded.latest();
        }
        self.read.push(read);
        self.written.push(written);
        self.discarded.push(discarded);
    }
}
impl NetInterfaceInfo {
    fn update(&mut self, old: &NetInterfaceSnapshot, new: &NetInterfaceSnapshot) {
        self.rx
            .push(self.wma_rx.add_sample((new.rx_bytes - old.rx_bytes) as f64));
        self.tx
            .push(self.wma_tx.add_sample((new.tx_bytes - old.tx_bytes) as f64));
    }
    fn push_sum_of_others<'a>(&mut self, others: impl Iterator<Item = &'a Self>) {
        let mut rx = 0.0;
        let mut tx = 0.0;
        for other in others {
            rx += other.rx.latest();
            tx += other.tx.latest();
        }
        self.rx.push(rx);
        self.tx.push(tx);
    }
}
impl GpuInfo {
    fn update(&mut self, new: &GpuSnapshot) {
        self.vram_total = new.mem_info_vram_total as f64;
        self.vram_used.push(new.mem_info_vram_used as f64);
        self.vram_busy.push(new.mem_busy_percent as f64 / 100.0);
        self.gpu_busy.push(
            self.wma_gpu_busy
                .add_sample(new.gpu_busy_percent as f64 / 100.0),
        );
        self.max_temperature.push(new.max_temperature as f64 / 1e3);
    }
    fn push_sum_of_others<'a>(&mut self, others: impl Iterator<Item = &'a Self>) {
        self.vram_total = 0.0;
        let mut vram_used = 0.0;
        let mut vram_busy = 0.0;
        let mut gpu_busy = 0.0;
        let mut max_temperature = 0.0;
        for other in others {
            self.vram_total += other.vram_total;
            vram_used += other.vram_used.latest();
            vram_busy += other.vram_busy.latest();
            gpu_busy += other.gpu_busy.latest();
            max_temperature += other.max_temperature.latest();
        }
        self.vram_used.push(vram_used);
        self.vram_busy.push(vram_busy);
        self.gpu_busy.push(gpu_busy);
        self.max_temperature.push(max_temperature);
    }
}

fn intersect_old_new<'a, T: 'a, U: Default>(
    ret: &mut BTreeMap<String, U>,
    old: impl Iterator<Item = (&'a String, &'a T)>,
    new: impl Iterator<Item = (&'a String, &'a T)>,
    mut f: impl FnMut(&mut U, &T, &T),
) {
    let old: BTreeMap<&String, &T> = old.collect();
    let new: BTreeMap<&String, &T> = new.filter(|(k, _)| old.contains_key(k)).collect();
    ret.retain(|k, _| new.contains_key(k));
    for (k, v) in new {
        let old_v = &old[&k];
        let ret_slot = ret.entry(k.to_owned()).or_insert_with(Default::default);
        f(ret_slot, old_v, v);
    }
}

#[derive(Clone, Debug)]
struct WindowMovingAverage {
    i: usize,
    samples: [f64; Self::SMOOTHING_WINDOW],
}
impl WindowMovingAverage {
    const SMOOTHING_WINDOW: usize = SUBSEC as usize;
    fn add_sample(&mut self, sample: f64) -> f64 {
        self.samples[self.i] = sample;
        self.i = (self.i + 1) % Self::SMOOTHING_WINDOW;
        self.samples.iter().copied().sum::<f64>() / Self::SMOOTHING_WINDOW as f64
    }
}
impl Default for WindowMovingAverage {
    fn default() -> Self {
        Self {
            i: 0,
            samples: [0.0; Self::SMOOTHING_WINDOW],
        }
    }
}
