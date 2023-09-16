use crate::{show::Show, Component};
use eframe::egui::{
    self, style::TextStyle, Color32, Frame, Id, Key, KeyboardShortcut, Modifiers, Sense, Ui,
};
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
                Table {
                    id_header: "Lsid",
                    sort_by: &mut sort_by,
                    rows: info.login_sessions.iter().map(|ls| Row {
                        id: format!("{:?}", ls.lsid),
                        name: &ls.name,
                        hover_name: None,
                        stat: ls.stat,
                    }),
                }
                .render(ui);
            }
            ProcessNavigation::Sessions => {
                Table {
                    id_header: "Sid",
                    sort_by: &mut sort_by,
                    rows: info.sessions.iter().map(|s| Row {
                        id: format!("{:?}", s.sid),
                        name: info.strings.get(s.name),
                        hover_name: (!s.entries_cmdline.is_empty()).then_some(&s.entries_cmdline),
                        stat: s.stat,
                    }),
                }
                .render(ui);
            }
            ProcessNavigation::Processes => {
                Table {
                    id_header: "Pid",
                    sort_by: &mut sort_by,
                    rows: info.processes.iter().map(|p| Row {
                        id: format!("{:?}", p.pid),
                        name: info.strings.get(p.name),
                        hover_name: p.cmdline.as_deref(),
                        stat: p.stat,
                    }),
                }
                .render(ui);
            }
            ProcessNavigation::Threads => {
                Table {
                    id_header: "Tid",
                    sort_by: &mut sort_by,
                    rows: info.threads.iter().map(|t| Row {
                        id: format!("{:?}", t.tid),
                        name: info.strings.get(t.name),
                        hover_name: None,
                        stat: t.stat,
                    }),
                }
                .render(ui);
            }
        }
        info.sort(sort_by);
    }
}
struct Table<'a, I: Iterator<Item = Row<'a>>> {
    id_header: &'a str,
    sort_by: &'a mut ProcSortBy,
    rows: I,
}
struct Row<'a> {
    id: String,
    name: &'a str,
    hover_name: Option<&'a str>,
    stat: ProcStat,
}
impl<'a, I: Iterator<Item = Row<'a>>> Table<'a, I> {
    fn render(mut self, ui: &mut Ui) {
        let row_height = ui.text_style_height(&TextStyle::Body);
        let spacing = ui.style().spacing.item_spacing;
        let total_col_spacing = 6.0 * spacing.x;
        let col_width = (ui.available_width() - total_col_spacing) / 7.0;
        egui::Frame::none()
            .fill(ui.style().visuals.widgets.hovered.bg_fill)
            .show(ui, |ui| {
                egui::Grid::new("table-header")
                    .min_col_width(col_width)
                    .max_col_width(col_width)
                    .spacing(spacing)
                    .show(ui, |ui| self.header(ui));
            });

        egui::ScrollArea::vertical().show_rows(ui, row_height, 7, |ui, row_range| {
            crate::vim_like_scroll(
                ui,
                2.0 * row_height,
                4.0 * row_height * (row_range.end - row_range.start) as f32,
            );
            egui::Grid::new("table-body")
                .min_col_width(col_width)
                .max_col_width(col_width)
                .spacing(spacing)
                .striped(true)
                .start_row(row_range.start)
                .show(ui, |ui| self.rows.for_each(|row| row.render(ui)));
        });
    }
    fn header(&mut self, ui: &mut Ui) {
        for (i, title) in [
            &format!("{} (i)", self.id_header),
            "Name (n)",
            "User cpu% (c)",
            "Sys cpu% (c)",
            "Disk read (r)",
            "Disk write (w)",
            "Mem (m)",
        ]
        .iter()
        .enumerate()
        {
            ui.add_sized(ui.available_size(), |ui: &mut Ui| {
                ui.selectable_value(
                    self.sort_by,
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
            });
        }
    }
}
impl<'a> Row<'a> {
    fn render(self, ui: &mut Ui) {
        ui.label(self.id);
        let resp_name = ui.monospace(self.name);
        if let Some(hover_name) = self.hover_name {
            ui.interact(resp_name.rect, Id::new("name-hover"), Sense::hover())
                .on_hover_ui_at_pointer(|ui| {
                    ui.monospace(hover_name);
                });
        }
        metric_cell(
            ui,
            self.stat.user_time_millis > 0,
            millis_to_percent(self.stat.user_time_millis),
        );
        metric_cell(
            ui,
            self.stat.system_time_millis > 0,
            millis_to_percent(self.stat.system_time_millis),
        );
        metric_cell(
            ui,
            self.stat.disk_read_bytes_per_second > 0,
            Show::rate(self.stat.disk_read_bytes_per_second as f64, ""),
        );
        metric_cell(
            ui,
            self.stat.disk_write_bytes_per_second > 0,
            Show::rate(self.stat.disk_write_bytes_per_second as f64, ""),
        );
        metric_cell(ui, false, Show::size(self.stat.mem_bytes as f64));
        ui.end_row();

        fn metric_cell(ui: &mut Ui, highlight: bool, text: String) {
            const HIGHLIGHT: Color32 = Color32::from_rgb(245, 196, 97);
            if highlight {
                ui.add_sized(ui.available_size(), |ui: &mut Ui| {
                    Frame::none()
                        .fill(HIGHLIGHT)
                        .show(ui, |ui| ui.label(text))
                        .response
                });
            } else {
                ui.add_sized(ui.available_size(), |ui: &mut Ui| ui.label(text));
            }
        }

        fn millis_to_percent(v: u32) -> String {
            assert!(v % 10 == 0);
            format!("{}%", v / 10)
        }
    }
}
