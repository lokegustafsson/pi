#[cfg(not(target_os = "linux"))]
compile_error!("pi supports only linux");

use crate::{
    process::{ProcessNavigation, ProcessTab},
    system::{SystemNavigation, SystemTab},
};
use clap::{Parser, Subcommand};
use eframe::egui::{self, Ui};
use ingest::Ingester;
use tracing_subscriber::Layer;

mod process;
mod show;
mod system;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    focus: Option<Focus>,
}
#[derive(Subcommand)]
enum Focus {
    Cpu,
    Ram,
    Disk,
    Net,
    Gpu,
}

fn main() -> Result<(), eframe::Error> {
    let cli = Cli::parse();

    tracing::subscriber::set_global_default(
        tracing_subscriber::filter::targets::Targets::new()
            .with_target("eframe::native::run", tracing::Level::INFO)
            .with_target("egui_glow", tracing::Level::INFO)
            .with_default(tracing::Level::TRACE)
            .with_subscriber(
                tracing_subscriber::FmtSubscriber::builder()
                    .with_max_level(tracing::Level::TRACE)
                    .finish(),
            ),
    )
    .expect("enabling global logger");

    eframe::run_native(
        "pi: process information",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(320.0, 240.0)),
            default_theme: eframe::Theme::Light,
            ..Default::default()
        },
        Box::new(move |_| {
            Box::new(State {
                nav: Navigation {
                    tab: if cli.focus.is_some() {
                        NavigationTab::System
                    } else {
                        NavigationTab::Process
                    },
                    process: ProcessNavigation::LoginSessions,
                    system: match cli.focus {
                        Some(Focus::Cpu) | None => SystemNavigation::Cpu,
                        Some(Focus::Ram) => SystemNavigation::Ram,
                        Some(Focus::Disk) => SystemNavigation::Disk,
                        Some(Focus::Net) => SystemNavigation::Net,
                        Some(Focus::Gpu) => SystemNavigation::Gpu,
                    },
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
    process: ProcessNavigation,
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
            match self.nav.tab {
                NavigationTab::Process => {
                    ProcessTab::render(ui, &mut self.nav.process, self.ingester.process_info())
                }
                NavigationTab::System => {
                    SystemTab::render(ui, &mut self.nav.system, self.ingester.system_info())
                }
            }
        });
        ctx.request_repaint_after(self.ingester.safe_sleep_duration())
    }
}

pub trait Component {
    type Navigation;
    type Info;
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &Self::Info);
}
