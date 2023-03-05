use crate::{
    query::{process::ProcessInfo, system::SystemInfo},
    snapshot::Snapshot,
};
use std::{fs::File, process::Command};

pub mod process;
pub mod system;

#[derive(Default)]
pub struct Ingester {
    config: Config,
    buf: String,
    pub process_info: ProcessInfo,
    pub system_info: SystemInfo,
}
struct Config {
    user_hz: u32,
    meminfo: File,
    swaps: File,
    stat: File,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            user_hz: {
                let output = Command::new("getconf").arg("CLK_TCK").output().unwrap();
                assert!(output.status.success());
                std::str::from_utf8(&output.stdout)
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap()
            },
            meminfo: File::open("/proc/meminfo").unwrap(),
            swaps: File::open("/proc/swaps").unwrap(),
            stat: File::open("/proc/stat").unwrap(),
        }
    }
}
impl Ingester {
    pub fn update(&mut self) {
        let new = Snapshot::new(
            &mut self.buf,
            &mut self.config.meminfo,
            &mut self.config.stat,
        );
        self.process_info.update(&new);
        self.system_info.update(&new);
    }
}
