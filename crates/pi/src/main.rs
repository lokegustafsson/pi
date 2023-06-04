#[cfg(not(target_os = "linux"))]
compile_error!("pi supports only linux");

use crate::{
    process::ProcessTab,
    system::{SystemNavigation, SystemTab},
};
use eframe::egui::{self, Ui};
use ingest::Ingester;

mod process;
mod system;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    eframe::run_native(
        "pi: process information",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(320.0, 240.0)),
            default_theme: eframe::Theme::Light,
            ..Default::default()
        },
        Box::new(|_| {
            Box::new(State {
                nav: Navigation {
                    tab: NavigationTab::Process,
                    process: (),
                    system: SystemNavigation::Cpu,
                },
                ingester: Ingester::new(),
            })
        }),
    )
}

struct State {
    nav: Navigation,
    ingester: Ingester,
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
        self.ingester.poll_update();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.nav.tab, NavigationTab::Process, "Processes");
                ui.selectable_value(&mut self.nav.tab, NavigationTab::System, "System");
            });
            egui::ScrollArea::vertical().show(ui, |ui| match self.nav.tab {
                NavigationTab::Process => {
                    ProcessTab::render(ui, &mut self.nav.process, self.ingester.process_info())
                }
                NavigationTab::System => {
                    SystemTab::render(ui, &mut self.nav.system, self.ingester.system_info())
                }
            });
        });
        ctx.request_repaint_after(self.ingester.safe_sleep_duration())
    }
}

pub trait Component {
    type Navigation;
    type Info;
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &Self::Info);
}
