use crate::Component;
use eframe::egui::{
    self,
    plot::{Corner, Legend, Line, Plot, PlotPoints},
    Frame, Grid, Label, Response, Sense, Ui,
};
use ingest::{SystemInfo, SystemInfoTick, HISTORY, TICK_DELAY};

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
        egui::SidePanel::left("system-left-panel")
            .max_width(1.0)
            .show_inside(ui, |ui| {
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
                    info,
                    &(|tick| tick.avg_cpu.total() as f64),
                    tick.cpus.len() as f64,
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
                    info,
                    &(|tick| tick.mem_used as f64),
                    tick.mem_total as f64,
                );
            });
        egui::ScrollArea::vertical().show(ui, |ui| match nav {
            SystemNavigation::Cpu => {
                let cpus = info.ticks.back().unwrap().cpus.len();
                let long_side = (cpus as f64).sqrt().ceil() as usize;
                let grid_cell_width = ui.available_width() / (long_side as f32);

                ui.heading("CPU View");
                TimeSeries {
                    name: "Total CPU",
                    extract: &(|tick| tick.avg_cpu.total() as f64),
                    max_y: cpus as f64,
                    kind: TimeSeriesKind::Primary,
                }
                .render(ui, info);
                Grid::new("cpu-grid").num_columns(long_side).show(ui, |ui| {
                    for i in 0..cpus {
                        TimeSeries {
                            name: &format!("CPU{i}"),
                            extract: &(|tick| tick.cpus[i].total() as f64),
                            max_y: 1.0,
                            kind: TimeSeriesKind::GridCell {
                                width: grid_cell_width,
                            },
                        }
                        .render(ui, info);
                        if (i + 1) % long_side == 0 {
                            ui.end_row();
                        }
                    }
                });
            }
            SystemNavigation::Ram => {
                ui.heading("RAM View");
                TimeSeries {
                    name: "RAM",
                    extract: &(|tick| tick.mem_used as f64),
                    max_y: info.ticks.back().unwrap().mem_total as f64,
                    kind: TimeSeriesKind::Primary,
                }
                .render(ui, info);
            }
        });
    }
}

fn left_panel_item(
    ui: &mut Ui,
    label: &'static str,
    sublabels: &[&str],
    nav: &mut SystemNavigation,
    value: SystemNavigation,
    info: &SystemInfo,
    extract: &dyn Fn(&SystemInfoTick) -> f64,
    max_y: f64,
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
                        TimeSeries {
                            name: label,
                            extract,
                            max_y,
                            kind: TimeSeriesKind::Preview,
                        }
                        .render(ui, info);
                        ui.horizontal_centered(|ui| {
                            ui.vertical(|ui| {
                                ui.add(Label::new(label).wrap(false));
                                for text in sublabels {
                                    ui.add(Label::new(*text).wrap(false));
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

struct TimeSeries<'a> {
    name: &'a str,
    extract: &'a dyn Fn(&SystemInfoTick) -> f64,
    max_y: f64,
    kind: TimeSeriesKind,
}
#[derive(PartialEq)]
enum TimeSeriesKind {
    Preview,
    Primary,
    GridCell { width: f32 },
}
impl<'a> TimeSeries<'a> {
    fn render(&self, ui: &mut Ui, info: &SystemInfo) -> Response {
        let points: PlotPoints = info
            .ticks
            .iter()
            .enumerate()
            .map(|(i, tick)| {
                [
                    (i as f64 - (info.ticks.len() - 1) as f64) * TICK_DELAY.as_secs_f64(),
                    (self.extract)(tick),
                ]
            })
            .collect();
        let line = Line::new(points).name(self.name);
        Plot::new(self.name)
            .view_aspect(match self.kind {
                TimeSeriesKind::Preview | TimeSeriesKind::Primary => 1.6,
                TimeSeriesKind::GridCell { .. } => 1.0,
            })
            .with_prop(
                match self.kind {
                    TimeSeriesKind::GridCell { width } => Some(width),
                    _ => None,
                },
                |plot, width| plot.width(width),
            )
            .show_x(false)
            .show_y(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .include_x(-((HISTORY - 1) as f64) * TICK_DELAY.as_secs_f64())
            .include_x(0)
            .include_y(0)
            .include_y(self.max_y)
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => None,
                    TimeSeriesKind::Primary | TimeSeriesKind::GridCell { .. } => Some(()),
                },
                |plot, ()| plot.legend(Legend::default().position(Corner::LeftTop)),
            )
            .show(ui, |ui| ui.line(line))
            .response
    }
}

trait BuilderOptional: Sized {
    fn with_prop<T>(self, prop: Option<T>, f: impl Fn(Self, T) -> Self) -> Self {
        match prop {
            Some(t) => f(self, t),
            None => self,
        }
    }
}
impl<T> BuilderOptional for T {}
