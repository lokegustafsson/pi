use procinfo::{ProcInfo, ProcIngest};
use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex,
    },
    thread,
};
use sysinfo::{SysHandles, SysInfo, SysOldSnapshot, SysSnapshot};
use util::{SUBSEC, TICK_DELAY};

struct MetricsProducer {
    scratch_buf: String,
    sys_handles: SysHandles,
    sys_old_snapshot: SysOldSnapshot,

    proc_ingest: ProcIngest,

    consumer: MetricsConsumer,
}
pub struct MetricsConsumer {
    pub sys_info: &'static Mutex<SysInfo>,
    pub proc_info: &'static Mutex<ProcInfo>,
    viewing: &'static AtomicU8,
}
impl MetricsConsumer {
    const VIEWING_PROC: u8 = 0;
    const VIEWING_SYS: u8 = 1;
    pub fn start(ctx: egui::Context) -> Self {
        let consumer = Self {
            sys_info: Box::leak(Box::new(Mutex::new(SysInfo::default()))),
            proc_info: Box::leak(Box::new(Mutex::new(ProcInfo::new()))),
            viewing: Box::leak(Box::new(AtomicU8::new(Self::VIEWING_SYS))),
        };
        let mut scratch_buf = String::new();
        let mut sys_handles = SysHandles::new();
        let producer = MetricsProducer {
            proc_ingest: ProcIngest::new(),
            sys_old_snapshot: SysSnapshot::new(&mut scratch_buf, &mut sys_handles).retire(),
            scratch_buf,
            sys_handles,
            consumer: Self {
                sys_info: consumer.sys_info,
                proc_info: consumer.proc_info,
                viewing: consumer.viewing,
            },
        };
        thread::spawn(move || producer.run(ctx));
        consumer
    }
    pub fn set_viewing_proc(&self) {
        self.viewing.store(Self::VIEWING_PROC, Ordering::Relaxed);
    }
    pub fn set_viewing_sys(&self) {
        self.viewing.store(Self::VIEWING_SYS, Ordering::Relaxed);
    }
}
impl MetricsProducer {
    fn run(mut self, ctx: egui::Context) {
        let mut proc_counter = 0;
        loop {
            thread::sleep(TICK_DELAY);
            self.update_sys();
            if proc_counter == 0 {
                proc_counter = SUBSEC;
                self.update_proc();
                ctx.request_repaint();
            } else {
                match self.consumer.viewing.load(Ordering::Relaxed) {
                    MetricsConsumer::VIEWING_PROC => {}
                    MetricsConsumer::VIEWING_SYS => ctx.request_repaint(),
                    _ => unreachable!(),
                }
            }
            proc_counter -= 1;
        }
    }
    fn update_sys(&mut self) {
        self.sys_handles.update();
        let new = SysSnapshot::new(&mut self.scratch_buf, &mut self.sys_handles);
        self.consumer
            .sys_info
            .lock()
            .unwrap()
            .update(&new, &self.sys_old_snapshot);
        self.sys_old_snapshot = new.retire();
    }
    fn update_proc(&mut self) {
        self.proc_ingest.update();
        self.consumer
            .proc_info
            .lock()
            .unwrap()
            .update(&self.proc_ingest);
    }
}
