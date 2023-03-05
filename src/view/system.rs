use crate::{
    query::system::{SystemInfo, SystemInfoTick},
    view::Component,
};
use eframe::egui::{self, Frame, Label, Sense, Ui};

pub struct SystemTab;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SystemNavigation {
    Cpu,
    Ram,
}
impl Component for SystemTab {
    type Navigation = SystemNavigation;
    type Info = SystemInfo;
    fn render(ui: &mut Ui, nav: &mut SystemNavigation, info: &SystemInfo) {
        ui.heading("System view");
        egui::SidePanel::left("system-left-panel").show_inside(ui, |ui| {
            let tick = info.ticks.back().unwrap();
            left_panel_item(
                ui,
                "CPU",
                &[
                    &format!("{:.2}/{}", tick.avg_cpu.total(), tick.cpus.len()),
                    &format!(
                        "({:.0}%)",
                        100.0 * tick.avg_cpu.total() / (tick.cpus.len() as f32)
                    ),
                ],
                nav,
                SystemNavigation::Cpu,
            );
            left_panel_item(
                ui,
                "RAM",
                &[
                    &format!(
                        "{:.0}/{:.0}GiB",
                        tick.mem_used as f64 / 2.0f64.powi(30),
                        tick.mem_total as f64 / 2.0f64.powi(30),
                    ),
                    &format!(
                        "({:.0}%)",
                        100.0 * tick.mem_used as f32 / tick.mem_total as f32
                    ),
                ],
                nav,
                SystemNavigation::Ram,
            );
        });
        egui::CentralPanel::default().show_inside(ui, |ui| match nav {
            SystemNavigation::Cpu => {
                ui.heading("CPU View");
                let s = format!("{info:#?}");
                ui.label(&s[..s.len().min(10000)]);
            }
            SystemNavigation::Ram => {
                ui.heading("RAM View");
                let s = format!("{info:#?}");
                ui.label(&s[..s.len().min(10000)]);
            }
        });
    }
}

fn left_panel_item(
    ui: &mut Ui,
    label: &'static str,
    info: &[&str],
    nav: &mut SystemNavigation,
    value: SystemNavigation,
) {
    let selected = *nav == value;
    if ui
        .push_id(label, |ui| {
            Frame::none()
                .inner_margin(6.0)
                .fill({
                    let visuals = ui.visuals();
                    match selected {
                        true => visuals.selection.bg_fill,
                        false => visuals.window_fill,
                    }
                })
                .show(ui, |ui| {
                    if selected {
                        let mut v = ui.visuals_mut();
                        v.override_text_color = Some(v.selection.stroke.color);
                    }
                    ui.horizontal(|ui| {
                        let texture = ui.ctx().load_texture(
                            "placeholder",
                            egui::ColorImage::example(),
                            Default::default(),
                        );
                        ui.image(&texture, texture.size_vec2());
                        ui.horizontal_centered(|ui| {
                            ui.vertical(|ui| {
                                ui.add(Label::new(label).wrap(false));
                                for i in info {
                                    ui.add(Label::new(*i).wrap(false));
                                }
                            })
                        });
                        ui.allocate_space(ui.available_size())
                    })
                });
        })
        .response
        .interact(Sense::click())
        .clicked()
    {
        *nav = value;
    }
}
