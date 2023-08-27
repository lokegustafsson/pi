use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    ffi::OsString,
    fs::{self, DirEntry, File},
    hash::{Hash, Hasher},
    path::Path,
    process::Command,
};

/// Additional relevant syscalls
///
/// - statfs64
/// - uname
///
/// Also
/// - `/sys/class/hwmon/*/name`
pub struct Handles {
    /// `/proc/diskstats`
    pub diskstats: File,
    /// `/proc/meminfo`
    pub meminfo: File,
    /// `/proc/mounts`
    pub mounts: File,
    /// `/proc/stat`
    pub stat: File,
    /// `/proc/uptime`
    pub uptime: File,

    /// `/sys/class/hwmon/{num}/temp*_input
    pub cpu_temperatures: Vec<File>,

    pub by_net_interface: BTreeMap<String, NetInterfaceHandles>,
    pub by_gpu: BTreeMap<String, GpuHandles>,

    /// Used to regenerate `Handles` if the environment has changed (e.g. new disk)
    environment_hash: u64,
}
pub struct NetInterfaceHandles {
    /// `/sys/class/net/{interface}/statistics/rx_bytes`
    pub rx_bytes: File,
    /// `/sys/class/net/{interface}/statistics/tx_bytes`
    pub tx_bytes: File,
}
pub struct GpuHandles {
    /// `/sys/class/drm/{gpu}/device/mem_info_vram_used`
    pub mem_info_vram_used: File,
    /// `/sys/class/drm/{gpu}/device/mem_info_vram_total`
    pub mem_info_vram_total: File,
    /// `/sys/class/drm/{gpu}/device/mem_busy_percent`
    pub mem_busy_percent: File,
    /// `/sys/class/drm/{gpu}/device/gpu_busy_percent`
    pub gpu_busy_percent: File,
    /// `/sys/class/hwmon/{num}/temp*_input
    pub temperatures: Vec<File>,
}
impl Handles {
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

        Self {
            environment_hash: Self::hash_environment(),

            diskstats: open("/proc/diskstats"),
            meminfo: open("/proc/meminfo"),
            mounts: open("/proc/mounts"),
            stat: open("/proc/stat"),
            uptime: open("/proc/uptime"),

            cpu_temperatures: {
                let mut ret = Vec::new();
                for hwmon in read_dir("/sys/class/hwmon") {
                    let path = hwmon.path();
                    if ["k10temp\n", "coretemp\n"]
                        .contains(&fs::read_to_string(path.join("name")).unwrap().as_ref())
                    {
                        ret.extend(hwmon_get_temps(&path));
                    }
                }
                ret
            },

            by_net_interface: read_dir("/sys/class/net")
                .map(|interface| {
                    let interface_name = interface.file_name().into_string().unwrap();
                    (
                        interface_name,
                        NetInterfaceHandles {
                            rx_bytes: open(interface.path().join("statistics/rx_bytes")),
                            tx_bytes: open(interface.path().join("statistics/tx_bytes")),
                        },
                    )
                })
                .collect(),
            by_gpu: read_dir("/sys/class/drm")
                .filter(|drm| {
                    let drm_name = drm.file_name().into_string().unwrap();
                    if drm_name == "version" {
                        return false;
                    }

                    let has_similar_name_children = read_dir(drm.path()).any(|entry| {
                        entry
                            .file_name()
                            .into_string()
                            .unwrap()
                            .starts_with(&drm_name)
                    });
                    has_similar_name_children
                })
                .filter_map(|drm| {
                    let device = drm.path().join("device");
                    Some((
                        drm.file_name().into_string().unwrap(),
                        GpuHandles {
                            mem_info_vram_used: try_open(device.join("mem_info_vram_used"))?,
                            mem_info_vram_total: open(device.join("mem_info_vram_total")),
                            mem_busy_percent: open(device.join("mem_busy_percent")),
                            gpu_busy_percent: open(device.join("gpu_busy_percent")),
                            temperatures: hwmon_get_temps(&device.join("hwmon/hwmon0")),
                        },
                    ))
                })
                .collect(),
        }
    }
    pub fn update(&mut self) {
        if Self::hash_environment() != self.environment_hash {
            *self = Self::new();
        }
    }
    fn hash_environment() -> u64 {
        let mut hasher = DefaultHasher::new();
        for path in [
            "/sys/class/net",
            "/sys/class/block",
            "/sys/block",
            "/sys/class/drm",
            "/sys/class/hwmon",
        ] {
            let mut v: Vec<OsString> = read_dir(path).map(|entry| entry.file_name()).collect();
            v.sort_unstable();
            v.hash(&mut hasher);
        }
        hasher.finish()
    }
}

fn read_dir(path: impl AsRef<Path>) -> impl Iterator<Item = DirEntry> {
    fs::read_dir(path).unwrap().map(|entry| entry.unwrap())
}
fn hwmon_get_temps(path: &Path) -> Vec<File> {
    read_dir(path)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().unwrap();
            (name.starts_with("temp") && name.ends_with("_input")).then(|| open(entry.path()))
        })
        .collect()
}
fn open(path: impl AsRef<Path>) -> File {
    try_open(path.as_ref()).unwrap()
}
fn try_open(path: impl AsRef<Path>) -> Option<File> {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => {
            tracing::info!(?path, "opening");
            Some(file)
        }
        Err(err) => {
            tracing::warn!(?path, ?err, "error opening");
            None
        }
    }
}
