use crate::Component;
use eframe::egui::{self, containers::popup, style::TextStyle, Id, Sense, Ui};
use procinfo::{ProcInfo, ProcStat};

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
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &Self::Info) {
        ui.horizontal(|ui| {
            ui.selectable_value(nav, ProcessNavigation::LoginSessions, "Login sessions");
            ui.selectable_value(nav, ProcessNavigation::Sessions, "Sessions");
            ui.selectable_value(nav, ProcessNavigation::Processes, "Processes");
            ui.selectable_value(nav, ProcessNavigation::Threads, "Threads");
        });

        match nav {
            ProcessNavigation::LoginSessions => {
                table(
                    ui,
                    &info.login_sessions,
                    &slice_concat(&["lsid"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.lsid));
                        stat_labels(ui, &row.stat);
                    },
                );
            }
            ProcessNavigation::Sessions => {
                table(
                    ui,
                    &info.sessions,
                    &slice_concat(&["sid", "cmdline"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.sid));
                        let cmdline = &info
                            .processes
                            .iter()
                            .find(|p| p.parent_sid == row.sid)
                            .unwrap()
                            .cmdline;
                        let resp_cmdline =
                            ui.monospace(format!("{}", cmdline_to_displayable(cmdline)));
                        ui.interact(resp_cmdline.rect, Id::new("cmdline"), Sense::hover())
                            .on_hover_ui_at_pointer(|ui| {
                                ui.label(format!("{}", &cmdline));
                            });
                        stat_labels(ui, &row.stat);
                    },
                );
            }
            ProcessNavigation::Processes => {
                table(
                    ui,
                    &info.processes,
                    &slice_concat(&["pid", "cmdline"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.pid));
                        let resp_cmdline =
                            ui.monospace(format!("{}", cmdline_to_displayable(&row.cmdline)));
                        ui.interact(resp_cmdline.rect, Id::new("cmdline"), Sense::hover())
                            .on_hover_ui_at_pointer(|ui| {
                                ui.label(format!("{}", &row.cmdline));
                            });
                        stat_labels(ui, &row.stat);
                    },
                );
            }
            ProcessNavigation::Threads => {
                table(
                    ui,
                    &info.threads,
                    &slice_concat(&["tid"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.tid));
                        stat_labels(ui, &row.stat);
                    },
                );
            }
        }
    }
}
fn table<T>(ui: &mut Ui, rows: &[T], header: &[&str], mut f: impl FnMut(&mut Ui, &T)) {
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
                        for title in header {
                            ui.label(*title);
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
const STAT_HEADER: &[&str] = &["user cpu%", "sys cpu%", "disk read", "disk write", "mem"];
fn stat_labels(ui: &mut Ui, stat: &ProcStat) {
    ui.label(format!("{:?}", stat.user_time_millis));
    ui.label(format!("{:?}", stat.system_time_millis));
    ui.label(format!("{:?}", stat.disk_read_bytes_per_second));
    ui.label(format!("{:?}", stat.disk_write_bytes_per_second));
    ui.label(format!("{:?}", stat.mem_bytes));
}
fn slice_concat(a: &[&'static str], b: &[&'static str]) -> Vec<&'static str> {
    let mut ret = Vec::new();
    ret.extend_from_slice(a);
    ret.extend_from_slice(b);
    ret
}
fn cmdline_to_displayable(s: &str) -> &str {
    if s.chars().next() == Some('[') {
        return &s[..s.len().min(20)];
    }
    let first_whitespace = s
        .find(|ch: char| ch.is_ascii_whitespace())
        .unwrap_or(s.len());
    let last_relevant_slash = s[..first_whitespace].rfind('/');
    match last_relevant_slash {
        None => s,
        Some(i) => &s[(i + 1)..s.len().min(i + 1 + 20)],
    }
}
