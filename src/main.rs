#[cfg(not(target_os = "linux"))]
compile_error!("pi supports only linux");

use crate::{
    query::Ingester,
    view::{
        process::ProcessTab,
        system::{SystemNavigation, SystemTab},
        Component,
    },
};
use eframe::egui;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

mod query;
mod snapshot;
mod view;

const SUBSEC: u64 = 5;
const TICK_DELAY: Duration = Duration::from_micros(1_000_000 / SUBSEC);
const HISTORY: usize = (60 * SUBSEC + 1) as usize;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    eframe::run_native(
        "pi: process information",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(320.0, 240.0)),
            default_theme: eframe::Theme::Light,
            ..Default::default()
        },
        Box::new(|cc| Box::new(setup(cc))),
    )
}
fn setup(cc: &eframe::CreationContext) -> State {
    let ingester = Arc::new(Mutex::new(Ingester::default()));

    thread::Builder::new()
        .name("pi-ingester".to_owned())
        .spawn({
            let ingester = Arc::clone(&ingester);
            let ctx = cc.egui_ctx.clone();
            move || {
                let mut deadline = Instant::now();
                loop {
                    ingester.lock().unwrap().update();
                    ctx.request_repaint();
                    deadline += TICK_DELAY;
                    if let Some(wait) = deadline.checked_duration_since(Instant::now()) {
                        thread::sleep(wait);
                    }
                }
            }
        })
        .unwrap();
    State {
        nav: Navigation {
            tab: NavigationTab::Process,
            process: (),
            system: SystemNavigation::Cpu,
        },
        ingester,
    }
}

struct State {
    nav: Navigation,
    ingester: Arc<Mutex<Ingester>>,
}
struct Navigation {
    tab: NavigationTab,
    process: (),
    system: SystemNavigation,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum NavigationTab {
    Process,
    System,
}
impl eframe::App for State {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let guard = self.ingester.lock().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.nav.tab, NavigationTab::Process, "Processes");
                ui.selectable_value(&mut self.nav.tab, NavigationTab::System, "System");
            });
            egui::ScrollArea::vertical().show(ui, |ui| match self.nav.tab {
                NavigationTab::Process => {
                    ProcessTab::render(ui, &mut self.nav.process, &guard.process_info)
                }
                NavigationTab::System => {
                    SystemTab::render(ui, &mut self.nav.system, &guard.system_info)
                }
            });
        });
    }
}
