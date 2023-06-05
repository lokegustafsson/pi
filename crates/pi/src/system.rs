use crate::Component;
use eframe::egui::{
    self,
    plot::{Corner, Legend, Line, Plot, PlotPoints},
    Frame, Grid, Label, Response, Sense, Ui,
};
use ingest::{Series, SystemInfo, HISTORY, TICK_DELAY};

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
                let total_cpu = info.total_cpu.total.latest();
                let num_cpu = info.by_cpu.len();
                let mem_used = info.global.mem_used.latest() as f64;
                let mem_total = info.global.mem_total as f64;
                left_panel_item(
                    ui,
                    "CPU",
                    &[
                        &format!("{:.2}/{}", total_cpu, num_cpu),
                        &format!("({:.0}%)", 100.0 * total_cpu / (num_cpu as f64)),
                    ],
                    nav,
                    SystemNavigation::Cpu,
                    &info.total_cpu.total,
                    num_cpu as f64,
                );
                left_panel_item(
                    ui,
                    "RAM",
                    &[
                        &format!(
                            "{:.0}/{:.0}GiB",
                            mem_used / 2.0f64.powi(30),
                            mem_total / 2.0f64.powi(30),
                        ),
                        &format!("({:.0}%)", 100.0 * mem_used / mem_total),
                    ],
                    nav,
                    SystemNavigation::Ram,
                    &info.global.mem_used,
                    mem_total,
                );
            });
        egui::ScrollArea::vertical().show(ui, |ui| match nav {
            SystemNavigation::Cpu => {
                let cpus = info.by_cpu.len();
                let long_side = (cpus as f64).sqrt().ceil() as usize;
                let grid_cell_width = ui.available_width() / (long_side as f32);

                ui.heading("CPU View");
                TimeSeries {
                    name: "Total CPU",
                    max_y: cpus as f64,
                    kind: TimeSeriesKind::Primary,
                }
                .render(ui, &info.total_cpu.total);
                Grid::new("cpu-grid").num_columns(long_side).show(ui, |ui| {
                    for i in 0..cpus {
                        TimeSeries {
                            name: &format!("CPU{i}"),
                            max_y: 1.0,
                            kind: TimeSeriesKind::GridCell {
                                width: grid_cell_width,
                            },
                        }
                        .render(ui, &info.by_cpu[i].total);
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
                    max_y: info.global.mem_total as f64,
                    kind: TimeSeriesKind::Primary,
                }
                .render(ui, &info.global.mem_used);
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
    series: &Series<f64>,
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
                            max_y,
                            kind: TimeSeriesKind::Preview,
                        }
                        .render(ui, series);
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
    fn render(&self, ui: &mut Ui, series: &Series<f64>) -> Response {
        let points: PlotPoints = series
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                [
                    (i as f64 - (series.len() - 1) as f64) * TICK_DELAY.as_secs_f64(),
                    y,
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
            .include_y(
                self.max_y
                    .min(1.2 * series.iter().copied().max_by(f64::total_cmp).unwrap()),
            )
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
