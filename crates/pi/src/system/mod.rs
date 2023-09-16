use crate::{
    show::Show,
    system::time_series::{TimeSeries, TimeSeriesKind, ValueKind},
    Component,
};
use eframe::egui::{
    self, Align, Frame, Grid, Id, Key, KeyboardShortcut, Label, Layout, Modifiers, Sense, Stroke,
    TextStyle, Ui, Vec2,
};
use sysinfo::{Series, SysInfo};

mod time_series;

const TICK_PER_SEC: f64 = util::SUBSEC as f64;
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
    type Info = SysInfo;
    fn render(ui: &mut Ui, nav: &mut SystemNavigation, info: &mut SysInfo) {
        ui.ctx().input_mut(|i| {
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::C)) {
                *nav = SystemNavigation::Cpu;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::M)) {
                *nav = SystemNavigation::Ram;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::D)) {
                *nav = SystemNavigation::Disk;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::N)) {
                *nav = SystemNavigation::Net;
            } else if !info.by_gpu.is_empty()
                && i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::G))
            {
                *nav = SystemNavigation::Gpu;
            }
        });

        ui.heading("System view");
        egui::SidePanel::left("system-left-panel").show_inside(ui, |ui| {
            side_panel_items(ui, nav, info);
        });

        match nav {
            SystemNavigation::Cpu => Page::CPU.render(ui, info, info.by_cpu.len()),
            SystemNavigation::Ram => Page::RAM.render(ui, info, 0),
            SystemNavigation::Disk => Page::DISK.render(ui, info, info.by_partition.len()),
            SystemNavigation::Net => Page::NET.render(ui, info, info.by_net_interface.len()),
            SystemNavigation::Gpu => Page::GPU.render(ui, info, 2),
        }
    }
}

fn side_panel_items(ui: &mut Ui, nav: &mut SystemNavigation, info: &SysInfo) {
    let total_cpu = info.total_cpu.slow_total.latest();
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
            name: "CPU (c)",
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
            name: "RAM (m)",
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
            name: "DISK (d)",
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
            name: "NET (n)",
            max_y: f64::INFINITY,
            kind: TimeSeriesKind::Preview,
            value_kind: ValueKind::Bytes,
        },
    );
    if !info.by_gpu.is_empty() {
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
                name: "GPU (g)",
                max_y: info.by_gpu.len() as f64,
                kind: TimeSeriesKind::Preview,
                value_kind: ValueKind::Percent,
            },
        );
    }
}

fn left_panel_item(
    ui: &mut Ui,
    size: Vec2,
    sublabels: &[&str],
    nav: &mut SystemNavigation,
    value: SystemNavigation,
    series: &[(&str, &Series<f64>)],
    time_series: TimeSeries<'_>,
) {
    let selected = *nav == value;
    let focused = ui.memory(|m| m.has_focus(Id::new(time_series.name).with("interact")));
    let rect = ui
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
                    .stroke({
                        let visuals = ui.visuals();
                        match focused {
                            true => visuals.selection.stroke,
                            false => Stroke::default(),
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
        .response
        .rect;
    let resp = ui.interact(
        rect,
        Id::new(time_series.name).with("interact"),
        Sense {
            click: true,
            drag: false,
            focusable: true,
        },
    );
    if resp.interact(Sense::click()).clicked() {
        *nav = value;
    }
}

struct Page {
    heading: &'static str,
    main_series: &'static [fn(&mut Ui, &SysInfo)],
    grid_name: &'static str,
    grid_series: fn(&mut Ui, &SysInfo, f32, usize),
}
impl Page {
    fn render(&self, ui: &mut Ui, info: &SysInfo, num_grid_items: usize) {
        let long_side = (num_grid_items as f64).sqrt().ceil() as usize;
        let margin = ui.available_width() * 0.03;
        let grid_cell_width =
            (ui.available_width() - 2.0 * margin) / (long_side as f32) - MARGIN_PIXELS;

        egui::ScrollArea::vertical().show(ui, |ui| {
            crate::vim_like_scroll(
                ui,
                4.0 * ui.text_style_height(&TextStyle::Body),
                40.0 * ui.text_style_height(&TextStyle::Body),
            );
            Frame::none().inner_margin(MARGIN_PIXELS).show(ui, |ui| {
                ui.heading(self.heading);
                for series in self.main_series {
                    Frame::none()
                        .inner_margin(margin)
                        .show(ui, |ui| series(ui, info));
                }
                Frame::none().inner_margin(margin).show(ui, |ui| {
                    Grid::new(self.grid_name)
                        .num_columns(long_side)
                        .show(ui, |ui| {
                            for i in 0..num_grid_items {
                                (self.grid_series)(ui, info, grid_cell_width, i);
                                if (i + 1) % long_side == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            })
        });
    }
    const CPU: Self = Self {
        heading: "CPU View",
        main_series: &[
            |ui, info| {
                TimeSeries {
                    name: "Total CPU",
                    max_y: info.by_cpu.len() as f64,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Percent,
                }
                .render(ui, &[("Total CPU", &info.total_cpu.total)])
            },
            |ui, info| {
                TimeSeries {
                    name: "CPU TEMP",
                    max_y: f64::INFINITY,
                    kind: TimeSeriesKind::Primary,
                    value_kind: ValueKind::Temperature,
                }
                .render(ui, &[("Max temperature", &info.global.cpu_max_temp)]);
            },
        ],
        grid_name: "cpu-grid",
        grid_series: |ui, info, grid_cell_width, i| {
            let name = format!("CPU{i}");
            TimeSeries {
                name: &name,
                max_y: 1.0,
                kind: TimeSeriesKind::GridCell {
                    width: grid_cell_width,
                },
                value_kind: ValueKind::Percent,
            }
            .render(ui, &[(&name, &info.by_cpu[i].total)])
        },
    };
    const RAM: Self = Self {
        heading: "RAM View",
        main_series: &[|ui, info| {
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
        }],
        grid_name: "",
        grid_series: |_, _, _, _| (),
    };
    const DISK: Self = Self {
        heading: "DISK View",
        main_series: &[|ui, info| {
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
        }],
        grid_name: "partition-grid",
        grid_series: |ui, info, grid_cell_width, i| {
            let (partition, part_info) = info.by_partition.iter().nth(i).unwrap();
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
        },
    };

    const NET: Self = Self {
        heading: "NET View",
        main_series: &[|ui, info| {
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
        }],
        grid_name: "net-grid",
        grid_series: |ui, info, grid_cell_width, i| {
            let (interface, interface_info) = info.by_net_interface.iter().nth(i).unwrap();
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
        },
    };
    const GPU: Self = Self {
        heading: "GPU View",
        main_series: &[|ui, info| {
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
        }],
        grid_name: "gpu-grid",
        grid_series: |ui, info, grid_cell_width, i| match i {
            0 => TimeSeries {
                name: "GPU VRAM",
                max_y: f64::INFINITY,
                kind: TimeSeriesKind::GridCell {
                    width: grid_cell_width,
                },
                value_kind: ValueKind::Bytes,
            }
            .render(ui, &[("VRAM usage", &info.total_gpu.vram_used)]),

            1 => TimeSeries {
                name: "GPU TEMP",
                max_y: f64::INFINITY,
                kind: TimeSeriesKind::GridCell {
                    width: grid_cell_width,
                },
                value_kind: ValueKind::Temperature,
            }
            .render(ui, &[("Max temperature", &info.total_gpu.max_temperature)]),
            other => unreachable!("{}", other),
        },
    };
}
