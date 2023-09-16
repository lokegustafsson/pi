#[cfg(not(target_os = "linux"))]
compile_error!("pi supports only linux");

use crate::{
    process::{ProcessNavigation, ProcessTab},
    system::{SystemNavigation, SystemTab},
};
use clap::{Parser, Subcommand};
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, Ui};
use ingest::{MetricsConsumer, ProducerStatus};
use std::{sync::Mutex, thread, time::Duration};
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

    let status = Box::leak(Box::new(Mutex::new(ProducerStatus::Running)));
    let ret = eframe::run_native(
        "pi: process information",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(320.0, 240.0)),
            default_theme: eframe::Theme::Light,
            run_and_return: true,
            ..Default::default()
        },
        Box::new({
            let status = &*status;
            move |cc| {
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
                    metrics: MetricsConsumer::start(cc.egui_ctx.clone(), status),
                })
            }
        }),
    );
    ProducerStatus::compare_and_set(status, ProducerStatus::Running, ProducerStatus::Exiting);
    while !ProducerStatus::compare_and_set(status, ProducerStatus::Exited, ProducerStatus::Exited) {
        thread::sleep(Duration::from_millis(50));
    }
    ret
}

struct State {
    nav: Navigation,
    metrics: MetricsConsumer,
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
        ctx.input_mut(|i| {
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::P)) {
                self.nav.tab = NavigationTab::Process;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::S)) {
                self.nav.tab = NavigationTab::System;
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.nav.tab, NavigationTab::Process, "Processes (P)");
                ui.selectable_value(&mut self.nav.tab, NavigationTab::System, "System (S)");
            });
            match self.nav.tab {
                NavigationTab::Process => ProcessTab::render(
                    ui,
                    &mut self.nav.process,
                    &mut self.metrics.proc_info.lock().unwrap(),
                ),
                NavigationTab::System => SystemTab::render(
                    ui,
                    &mut self.nav.system,
                    &mut self.metrics.sys_info.lock().unwrap(),
                ),
            }
        });
        match self.nav.tab {
            NavigationTab::Process => self.metrics.set_viewing_proc(),
            NavigationTab::System => self.metrics.set_viewing_sys(),
        }
    }
}

pub trait Component {
    type Navigation;
    type Info;
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &mut Self::Info);
}

fn vim_like_scroll(ui: &mut Ui, small_jump: f32, large_jump: f32) {
    let mut scroll_up = 0.0;
    ui.ctx().input_mut(|i| {
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::K)) {
            scroll_up += small_jump;
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::J)) {
            scroll_up -= small_jump;
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::U)) {
            scroll_up += large_jump;
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::D)) {
            scroll_up -= large_jump;
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::G)) {
            scroll_up += f32::INFINITY;
        }
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::SHIFT, Key::G)) {
            scroll_up -= f32::INFINITY;
        }
    });
    ui.scroll_with_delta(egui::Vec2::DOWN * scroll_up);
}
