use crate::{
    show::Show,
    system::time_series::{TimeSeries, TimeSeriesKind, ValueKind},
    Component,
};
use eframe::egui::{self, Align, Frame, Grid, Label, Layout, Sense, Ui, Vec2};
use ingest::{Series, SystemInfo};

mod time_series;

const TICK_PER_SEC: f64 = ingest::SUBSEC as f64;
const MARGIN_PIXELS: f32 = 6.0;

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
            side_panel_items(ui, nav, info);
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            Frame::none()
                .inner_margin(MARGIN_PIXELS)
                .show(ui, |ui| match nav {
                    SystemNavigation::Cpu => Page::cpu(ui, info),
                    SystemNavigation::Ram => Page::ram(ui, info),
                    SystemNavigation::Disk => Page::disk(ui, info),
                    SystemNavigation::Net => Page::net(ui, info),
                    SystemNavigation::Gpu => Page::gpu(ui, info),
                })
        });
    }
}

fn side_panel_items(ui: &mut Ui, nav: &mut SystemNavigation, info: &SystemInfo) {
    let total_cpu = info.total_cpu.total.latest();
    let num_cpu = info.by_cpu.len();
    let mem_used = info.global.mem_used.latest();
    let size = {
        let mut ret = ui.available_size();
        ret.y -= 2.0 * MARGIN_PIXELS;
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
            ("Read", &info.total_partition.read),
            ("Write", &info.total_partition.written),
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
            &Show::rate(TICK_PER_SEC * info.total_net.wma_rx.get(), "Receive "),
            &Show::rate(TICK_PER_SEC * info.total_net.wma_tx.get(), "Transmit "),
        ],
        nav,
        SystemNavigation::Net,
        &[
            ("Receive", &info.total_net.rx),
            ("Transmit", &info.total_net.tx),
        ],
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
            &Show::size_fraction(info.total_gpu.vram_used.latest(), info.total_gpu.vram_total),
            &format!(
                "VRAM busy {:.0}%",
                100.0 * info.total_gpu.vram_busy.latest()
            ),
            &format!("GPU {:.0}%", 100.0 * info.total_gpu.gpu_busy.latest()),
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

struct Page;
impl Page {
    fn cpu(ui: &mut Ui, info: &SystemInfo) {
        let cpus = info.by_cpu.len();
        let long_side = (cpus as f64).sqrt().ceil() as usize;
        let grid_cell_width = ui.available_width() / (long_side as f32) - MARGIN_PIXELS;

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

    fn ram(ui: &mut Ui, info: &SystemInfo) {
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
                ("Used", &info.global.mem_used),
                ("Including reclaimable", &info.global.mem_inc_reclaimable),
            ],
        );
    }
    fn disk(ui: &mut Ui, info: &SystemInfo) {
        let parts = info.by_partition.len();
        let long_side = (parts as f64).sqrt().ceil() as usize;
        let grid_cell_width = ui.available_width() / (long_side as f32) - MARGIN_PIXELS;

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
                ("Read", &info.total_partition.read),
                ("Write", &info.total_partition.written),
                ("Discard", &info.total_partition.discarded),
            ],
        );
        Grid::new("partition-grid")
            .num_columns(long_side)
            .show(ui, |ui| {
                for (i, (partition, part_info)) in info.by_partition.iter().enumerate() {
                    TimeSeries {
                        name: partition,
                        max_y: f64::INFINITY,
                        kind: TimeSeriesKind::GridCell {
                            width: grid_cell_width,
                        },
                        value_kind: ValueKind::Bytes,
                    }
                    .render(
                        ui,
                        &[
                            (&format!("{partition} read"), &part_info.read),
                            (&format!("{partition} write"), &part_info.written),
                            (&format!("{partition} discard"), &part_info.discarded),
                        ],
                    );
                    if (i + 1) % long_side == 0 {
                        ui.end_row();
                    }
                }
            });
    }
    fn net(ui: &mut Ui, info: &SystemInfo) {
        let interfaces = info.by_partition.len();
        let long_side = (interfaces as f64).sqrt().ceil() as usize;
        let grid_cell_width = ui.available_width() / (long_side as f32) - MARGIN_PIXELS;

        ui.heading("NET View");
        TimeSeries {
            name: "NET",
            max_y: f64::INFINITY,
            kind: TimeSeriesKind::Primary,
            value_kind: ValueKind::Bytes,
        }
        .render(
            ui,
            &[
                ("Receive", &info.total_net.rx),
                ("Transmit", &info.total_net.tx),
            ],
        );
        Grid::new("net-grid").num_columns(long_side).show(ui, |ui| {
            for (i, (interface, interface_info)) in info.by_net_interface.iter().enumerate() {
                TimeSeries {
                    name: interface,
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::GridCell {
                        width: grid_cell_width,
                    },
                    value_kind: ValueKind::Bytes,
                }
                .render(
                    ui,
                    &[
                        (&format!("{interface} receive"), &interface_info.rx),
                        (&format!("{interface} transmit"), &interface_info.tx),
                    ],
                );
                if (i + 1) % long_side == 0 {
                    ui.end_row();
                }
            }
        });
    }
    fn gpu(ui: &mut Ui, info: &SystemInfo) {
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
                ("GPU busy", &info.total_gpu.gpu_busy),
                ("VRAM busy", &info.total_gpu.vram_busy),
            ],
        );
        TimeSeries {
            name: "GPU VRAM",
            max_y: f64::INFINITY,
            kind: TimeSeriesKind::Primary,
            value_kind: ValueKind::Bytes,
        }
        .render(ui, &[("VRAM usage", &info.total_gpu.vram_used)]);
        TimeSeries {
            name: "GPU TEMP",
            max_y: f64::INFINITY,
            kind: TimeSeriesKind::Primary,
            value_kind: ValueKind::Temperature,
        }
        .render(ui, &[("Max temperature", &info.total_gpu.max_temperature)]);
    }
}
