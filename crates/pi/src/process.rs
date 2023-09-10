use crate::{show::Show, Component};
use eframe::egui::{self, style::TextStyle, Id, Key, KeyboardShortcut, Modifiers, Sense, Ui};
use procinfo::{ProcInfo, ProcSortBy, ProcStat};

pub struct ProcessTab;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessNavigation {
    LoginSessions,
    Sessions,
    Processes,
    Threads,
}
impl Component for ProcessTab {
    type Navigation = ProcessNavigation;
    type Info = ProcInfo;
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &mut Self::Info) {
        ui.ctx().input_mut(|i| {
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::L)) {
                *nav = ProcessNavigation::LoginSessions;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::S)) {
                *nav = ProcessNavigation::Sessions;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::P)) {
                *nav = ProcessNavigation::Processes;
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::T)) {
                *nav = ProcessNavigation::Threads;
            }
            if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::I)) {
                info.sort(ProcSortBy::Id);
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::N)) {
                info.sort(ProcSortBy::Name);
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::C)) {
                info.sort(ProcSortBy::Cpu);
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::R)) {
                info.sort(ProcSortBy::DiskRead);
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::W)) {
                info.sort(ProcSortBy::DiskWrite);
            } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::M)) {
                info.sort(ProcSortBy::Memory);
            }
        });

        ui.horizontal(|ui| {
            ui.selectable_value(nav, ProcessNavigation::LoginSessions, "Login sessions (l)");
            ui.selectable_value(nav, ProcessNavigation::Sessions, "Sessions (s)");
            ui.selectable_value(nav, ProcessNavigation::Processes, "Processes (p)");
            ui.selectable_value(nav, ProcessNavigation::Threads, "Threads (t)");
        });

        let mut sort_by = info.get_sort_by();
        match nav {
            ProcessNavigation::LoginSessions => {
                table(
                    ui,
                    &info.login_sessions,
                    "Lsid",
                    |ui, row| {
                        ui.label(format!("{:?}", row.lsid));
                        ui.monospace(&row.name);
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
            ProcessNavigation::Sessions => {
                table(
                    ui,
                    &info.sessions,
                    "Sid",
                    |ui, row| {
                        ui.label(format!("{:?}", row.sid));
                        let resp_name = ui.monospace(info.strings.get(row.name));
                        if !row.entries_cmdline.is_empty() {
                            ui.interact(resp_name.rect, Id::new("cmdline"), Sense::hover())
                                .on_hover_ui_at_pointer(|ui| {
                                    ui.monospace(&row.entries_cmdline);
                                });
                        }
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
            ProcessNavigation::Processes => {
                table(
                    ui,
                    &info.processes,
                    "Pid",
                    |ui, row| {
                        ui.label(format!("{:?}", row.pid));
                        let resp_cmdline = ui.monospace(info.strings.get(row.name));
                        if let Some(cmdline) = &row.cmdline {
                            ui.interact(resp_cmdline.rect, Id::new("cmdline"), Sense::hover())
                                .on_hover_ui_at_pointer(|ui| {
                                    ui.monospace(&**cmdline);
                                });
                        }
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
            ProcessNavigation::Threads => {
                table(
                    ui,
                    &info.threads,
                    "Tid",
                    |ui, row| {
                        ui.label(format!("{:?}", row.tid));
                        ui.monospace(info.strings.get(row.name));
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
        }
        info.sort(sort_by);
    }
}
fn table<T>(
    ui: &mut Ui,
    rows: &[T],
    id_header: &str,
    mut f: impl FnMut(&mut Ui, &T),
    sort_by: &mut ProcSortBy,
) {
    let header: [&str; 7] = [
        &format!("{id_header} (i)"),
        "Name (n)",
        "User cpu% (c)",
        "Sys cpu% (c)",
        "Disk read (r)",
        "Disk write (w)",
        "Mem (m)",
    ];

    egui::Frame::none().outer_margin(20.0).show(ui, |ui| {
        let row_height = ui.text_style_height(&TextStyle::Body);
        let spacing = ui.style().spacing.item_spacing;
        let total_col_spacing = (header.len() - 1) as f32 * spacing.x;
        let col_width = (ui.available_width() - total_col_spacing) / header.len() as f32;
        egui::Frame::none()
            .fill(ui.style().visuals.widgets.hovered.bg_fill)
            .show(ui, |ui| {
                egui::Grid::new("table-header")
                    .min_col_width(col_width)
                    .max_col_width(col_width)
                    .spacing(spacing)
                    .show(ui, |ui| {
                        for (i, title) in header.iter().enumerate() {
                            if ui
                                .selectable_value(
                                    sort_by,
                                    match i {
                                        0 => ProcSortBy::Id,
                                        1 => ProcSortBy::Name,
                                        2 | 3 => ProcSortBy::Cpu,
                                        4 => ProcSortBy::DiskRead,
                                        5 => ProcSortBy::DiskWrite,
                                        6 => ProcSortBy::Memory,
                                        _ => unreachable!(),
                                    },
                                    *title,
                                )
                                .clicked()
                            {}
                        }
                    });
            });

        egui::ScrollArea::vertical().show_rows(ui, row_height, rows.len(), |ui, row_range| {
            egui::Grid::new("table-body")
                .min_col_width(col_width)
                .max_col_width(col_width)
                .spacing(spacing)
                .striped(true)
                .start_row(row_range.start)
                .show(ui, |ui| {
                    for row in &rows[row_range] {
                        f(ui, row);
                        ui.end_row();
                    }
                });
        });
    });
}
fn stat_labels(ui: &mut Ui, stat: &ProcStat) {
    ui.label(millis_to_percent(stat.user_time_millis));
    ui.label(millis_to_percent(stat.system_time_millis));
    ui.label(Show::rate(stat.disk_read_bytes_per_second as f64, ""));
    ui.label(Show::rate(stat.disk_write_bytes_per_second as f64, ""));
    ui.label(Show::size(stat.mem_bytes as f64));
}
fn millis_to_percent(v: u32) -> String {
    format!("{}.{}%", v / 10, v % 10)
}
