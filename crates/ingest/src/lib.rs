use procinfo::{ProcInfo, ProcIngest};
use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{SysHandles, SysInfo, SysOldSnapshot, SysSnapshot};
use util::{SUBSEC, TICK_DELAY};

struct MetricsProducer {
    sys_handles: SysHandles,
    sys_old_snapshot: SysOldSnapshot,

    proc_ingest: ProcIngest,

    consumer: MetricsConsumer,

    num_sys_ingest: usize,
    num_proc_ingest: usize,
    cumulative_sys_ingest: Duration,
    cumulative_proc_ingest: Duration,
}
pub struct MetricsConsumer {
    pub sys_info: &'static Mutex<SysInfo>,
    pub proc_info: &'static Mutex<ProcInfo>,
    viewing: &'static AtomicU8,
}
#[derive(Debug, PartialEq, Eq)]
pub enum ProducerStatus {
    Running,
    Exiting,
    Exited,
}
impl ProducerStatus {
    pub fn compare_and_set(shared: &Mutex<Self>, expected: Self, set: Self) -> bool {
        let mut guard = shared.lock().unwrap();
        if *guard == expected {
            *guard = set;
            true
        } else {
            false
        }
    }
}
impl MetricsConsumer {
    const VIEWING_PROC: u8 = 0;
    const VIEWING_SYS: u8 = 1;
    pub fn start(ctx: egui::Context, status: &'static Mutex<ProducerStatus>) -> Self {
        let consumer = Self {
            sys_info: Box::leak(Box::new(Mutex::new(SysInfo::default()))),
            proc_info: Box::leak(Box::new(Mutex::new(ProcInfo::new()))),
            viewing: Box::leak(Box::new(AtomicU8::new(Self::VIEWING_SYS))),
        };
        let mut sys_handles = SysHandles::new();
        let producer = MetricsProducer {
            proc_ingest: ProcIngest::new(),
            sys_old_snapshot: SysSnapshot::new(&mut sys_handles).retire(),
            sys_handles,
            consumer: Self {
                sys_info: consumer.sys_info,
                proc_info: consumer.proc_info,
                viewing: consumer.viewing,
            },
            num_sys_ingest: 0,
            num_proc_ingest: 0,
            cumulative_sys_ingest: Duration::ZERO,
            cumulative_proc_ingest: Duration::ZERO,
        };
        thread::spawn(move || producer.run(ctx, status));
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
    fn run(mut self, ctx: egui::Context, status: &'static Mutex<ProducerStatus>) {
        let mut proc_counter = 0;
        loop {
            thread::sleep(TICK_DELAY);
            if ProducerStatus::compare_and_set(
                status,
                ProducerStatus::Exiting,
                ProducerStatus::Exiting,
            ) {
                let _ = self;
                assert!(ProducerStatus::compare_and_set(
                    status,
                    ProducerStatus::Exiting,
                    ProducerStatus::Exited
                ));
                return;
            }

            let now = Instant::now();
            self.update_sys();
            self.cumulative_sys_ingest += now.elapsed();
            self.num_sys_ingest += 1;

            if proc_counter == 0 {
                proc_counter = SUBSEC;

                let now = Instant::now();
                self.update_proc();
                self.cumulative_proc_ingest += now.elapsed();
                self.num_proc_ingest += 1;

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
        let new = SysSnapshot::new(&mut self.sys_handles);
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
impl Drop for MetricsProducer {
    fn drop(&mut self) {
        let avg_sys_ingest_time_ms =
            (self.cumulative_sys_ingest.as_micros() / self.num_sys_ingest as u128) as f64 / 1000.0;
        let avg_proc_ingest_time_ms = (self.cumulative_proc_ingest.as_micros()
            / self.num_proc_ingest as u128) as f64
            / 1000.0;
        tracing::info!("avg sys ingest time = {}ms", avg_sys_ingest_time_ms);
        tracing::info!("avg proc ingest time = {}ms", avg_proc_ingest_time_ms);
    }
}
