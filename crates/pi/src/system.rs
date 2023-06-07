use crate::{show::Show, Component};
use eframe::egui::{
    self,
    plot::{Corner, Legend, Line, Plot, PlotPoints},
    Align, Frame, Grid, Label, Layout, Sense, Ui, Vec2,
};
use ingest::{Series, SystemInfo, HISTORY, TICK_DELAY};
use std::{collections::btree_map::Range, ops::RangeInclusive};

const TICK_PER_SEC: f64 = ingest::SUBSEC as f64;

pub struct SystemTab;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SystemNavigation {
    Cpu,
    Ram,
    Disk,
    Net,
    Gpu,
}
impl Component for SystemTab {
    type Navigation = SystemNavigation;
    type Info = SystemInfo;
    fn render(ui: &mut Ui, nav: &mut SystemNavigation, info: &SystemInfo) {
        ui.heading("System view");
        egui::SidePanel::left("system-left-panel").show_inside(ui, |ui| {
            let total_cpu = info.total_cpu.total.latest();
            let num_cpu = info.by_cpu.len();
            let mem_used = info.global.mem_used.latest();
            let size = {
                let mut ret = ui.available_size();
                ret.y /= 5.0;
                ret
            };
            left_panel_item(
                ui,
                size,
                &[
                    &format!("{:.2}/{}", total_cpu, num_cpu),
                    &format!("({:.0}%)", 100.0 * total_cpu / (num_cpu as f64)),
                    &format!("{:.0}C", info.global.cpu_max_temp.latest()),
                ],
                nav,
                SystemNavigation::Cpu,
                &[("", &info.total_cpu.total)],
                TimeSeries {
                    name: "CPU",
                    max_y: num_cpu as f64,
                    kind: TimeSeriesKind::Preview,
                    value_kind: ValueKind::Percent,
                },
            );
            left_panel_item(
                ui,
                size,
                &[
                    &Show::size_fraction(mem_used, info.global.mem_total).to_string(),
                    &format!("({:.0}%)", 100.0 * mem_used / info.global.mem_total),
                ],
                nav,
                SystemNavigation::Ram,
                &[("", &info.global.mem_used)],
                TimeSeries {
                    name: "RAM",
                    max_y: info.global.mem_total,
                    kind: TimeSeriesKind::Preview,
                    value_kind: ValueKind::Bytes,
                },
            );
            left_panel_item(
                ui,
                size,
                &[
                    &Show::size_fraction(info.total_partition.used, info.total_partition.capacity)
                        .to_string(),
                    &Show::rate(TICK_PER_SEC * info.total_partition.wma_read.get(), "Read "),
                    &Show::rate(
                        TICK_PER_SEC * info.total_partition.wma_written.get(),
                        "Write ",
                    ),
                    &Show::rate(
                        TICK_PER_SEC * info.total_partition.wma_discarded.get(),
                        "Discard ",
                    ),
                ],
                nav,
                SystemNavigation::Disk,
                &[
                    ("READ", &info.total_partition.read),
                    ("WRITE", &info.total_partition.written),
                ],
                TimeSeries {
                    name: "DISK",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Preview,
                    value_kind: ValueKind::Bytes,
                },
            );
            left_panel_item(
                ui,
                size,
                &[
                    &Show::rate(TICK_PER_SEC * info.total_net.wma_rx.get(), "RX "),
                    &Show::rate(TICK_PER_SEC * info.total_net.wma_tx.get(), "TX "),
                ],
                nav,
                SystemNavigation::Net,
                &[("RX", &info.total_net.rx), ("TX", &info.total_net.tx)],
                TimeSeries {
                    name: "NET",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Preview,
                    value_kind: ValueKind::Bytes,
                },
            );
            left_panel_item(
                ui,
                size,
                &[
                    &Show::size_fraction(
                        info.total_gpu.vram_used.latest(),
                        info.total_gpu.vram_total,
                    ),
                    &format!("Mem {:.0}%", 100.0 * info.total_gpu.vram_busy.latest()),
                    &format!("{:.0}%", 100.0 * info.total_gpu.gpu_busy.latest()),
                    &format!("{:.0}C", info.total_gpu.max_temperature.latest()),
                ],
                nav,
                SystemNavigation::Gpu,
                &[("", &info.total_gpu.gpu_busy)],
                TimeSeries {
                    name: "GPU",
                    max_y: info.by_gpu.len() as f64,
                    kind: TimeSeriesKind::Preview,
                    value_kind: ValueKind::Percent,
                },
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
                    value_kind: ValueKind::Percent,
                }
                .render(ui, &[("Total CPU", &info.total_cpu.total)]);
                TimeSeries {
                    name: "CPU TEMP",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Temperature,
                }
                .render(ui, &[("Max temperature", &info.global.cpu_max_temp)]);
                Grid::new("cpu-grid").num_columns(long_side).show(ui, |ui| {
                    for i in 0..cpus {
                        let name = format!("CPU{i}");
                        TimeSeries {
                            name: &name,
                            max_y: 1.0,
                            kind: TimeSeriesKind::GridCell {
                                width: grid_cell_width,
                            },
                            value_kind: ValueKind::Percent,
                        }
                        .render(ui, &[(&name, &info.by_cpu[i].total)]);
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
                    value_kind: ValueKind::Bytes,
                }
                .render(
                    ui,
                    &[
                        ("USED", &info.global.mem_used),
                        ("RECLAIMABLE", &info.global.mem_reclaimable),
                    ],
                );
            }
            SystemNavigation::Disk => {
                ui.heading("DISK View");
                TimeSeries {
                    name: "DISK",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Bytes,
                }
                .render(
                    ui,
                    &[
                        ("READ", &info.total_partition.read),
                        ("WRITE", &info.total_partition.written),
                        ("DISCARD", &info.total_partition.discarded),
                    ],
                );
            }
            SystemNavigation::Net => {
                ui.heading("NET View");
                TimeSeries {
                    name: "NET",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Bytes,
                }
                .render(
                    ui,
                    &[("RX", &info.total_net.rx), ("TX", &info.total_net.tx)],
                );
            }
            SystemNavigation::Gpu => {
                ui.heading("GPU View");
                TimeSeries {
                    name: "GPU BUSY",
                    max_y: info.by_gpu.len() as f64,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Percent,
                }
                .render(
                    ui,
                    &[
                        ("GPU BUSY", &info.total_gpu.gpu_busy),
                        ("VRAM BUSY", &info.total_gpu.vram_busy),
                    ],
                );
                TimeSeries {
                    name: "GPU VRAM",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Bytes,
                }
                .render(ui, &[("VRAM", &info.total_gpu.vram_used)]);
                TimeSeries {
                    name: "GPU TEMP",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Temperature,
                }
                .render(ui, &[("TEMP", &info.total_gpu.max_temperature)]);
            }
        });
    }
}

fn left_panel_item(
    ui: &mut Ui,
    size: Vec2,
    sublabels: &[&str],
    nav: &mut SystemNavigation,
    value: SystemNavigation,
    series: &[(&str, &Series<f64>)],
    time_series: TimeSeries,
) {
    let selected = *nav == value;
    let resp = ui
        .push_id(time_series.name, |ui| {
            ui.allocate_ui(size, |ui| {
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
                        ui.allocate_ui_with_layout(
                            ui.available_size(),
                            Layout::right_to_left(Align::Center),
                            |ui| {
                                ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                                    ui.add(Label::new(time_series.name).wrap(false));
                                    for text in sublabels {
                                        ui.add(Label::new(*text).wrap(false));
                                    }
                                });
                                time_series.render(ui, series);
                            },
                        )
                    });
            })
        })
        .response;
    if resp.interact(Sense::click()).clicked() {
        *nav = value;
    }
}

struct TimeSeries<'a> {
    name: &'a str,
    max_y: f64,
    kind: TimeSeriesKind,
    value_kind: ValueKind,
}
#[derive(PartialEq)]
enum TimeSeriesKind {
    Preview,
    Primary,
    GridCell { width: f32 },
}
#[derive(PartialEq)]
enum ValueKind {
    Percent,
    Bytes,
    Temperature,
}
impl<'a> TimeSeries<'a> {
    fn render(&self, ui: &mut Ui, series: &[(&str, &Series<f64>)]) {
        let series_max_y = series
            .iter()
            .map(|(_, series)| series.iter().copied().max_by(f64::total_cmp).unwrap())
            .reduce(f64::max)
            .unwrap();
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
            .allow_drag(false)
            .include_x(-((HISTORY - 1) as f64) * TICK_DELAY.as_secs_f64())
            .include_x(0)
            .include_y(0)
            .include_y(self.max_y.min(1.2 * series_max_y))
            .y_axis_formatter(match self.value_kind {
                ValueKind::Bytes => |val, range: &RangeInclusive<f64>| {
                    let maximum = *range.end();
                    Show::size_at_scale(val, maximum)
                },
                ValueKind::Percent => |val, _: &_| format!("{:.0}%", 100.0 * val),
                ValueKind::Temperature => |val, _: &_| format!("{val}°C"),
            })
            .with_prop(
                match self.kind {
                    TimeSeriesKind::Preview => None,
                    TimeSeriesKind::Primary | TimeSeriesKind::GridCell { .. } => Some(()),
                },
                |plot, ()| plot.legend(Legend::default().position(Corner::LeftTop)),
            )
            .show(ui, |ui| {
                for (name, series) in series {
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
                    ui.line(Line::new(points).name(name));
                }
            })
            .response;
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
