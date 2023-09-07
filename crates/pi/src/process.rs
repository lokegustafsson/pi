use crate::{show::Show, Component};
use eframe::egui::{self, style::TextStyle, Id, Sense, Ui};
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
        ui.horizontal(|ui| {
            ui.selectable_value(nav, ProcessNavigation::LoginSessions, "Login sessions");
            ui.selectable_value(nav, ProcessNavigation::Sessions, "Sessions");
            ui.selectable_value(nav, ProcessNavigation::Processes, "Processes");
            ui.selectable_value(nav, ProcessNavigation::Threads, "Threads");
        });

        let mut sort_by = None;
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
                    &mut sort_by,
                );
            }
            ProcessNavigation::Sessions => {
                table(
                    ui,
                    &info.sessions,
                    &slice_concat(&["sid", "name"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.sid));
                        let resp_name = ui.monospace(&*row.name);
                        ui.interact(resp_name.rect, Id::new("cmdline"), Sense::hover())
                            .on_hover_ui_at_pointer(|ui| {
                                for entry in &row.entries_cmdline {
                                    ui.monospace(&**entry);
                                }
                            });
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
            ProcessNavigation::Processes => {
                table(
                    ui,
                    &info.processes,
                    &slice_concat(&["pid", "name"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.pid));
                        let resp_cmdline = ui.monospace(&*row.name);
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
                    &slice_concat(&["tid", "name"], STAT_HEADER),
                    |ui, row| {
                        ui.label(format!("{:?}", row.tid));
                        ui.monospace(&*row.name);
                        stat_labels(ui, &row.stat);
                    },
                    &mut sort_by,
                );
            }
        }
        if let Some(sort_by) = sort_by {
            info.sort(sort_by);
        }
    }
}
fn table<T>(
    ui: &mut Ui,
    rows: &[T],
    header: &[&str],
    mut f: impl FnMut(&mut Ui, &T),
    sort_by: &mut Option<ProcSortBy>,
) {
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
                            if ui.button(*title).clicked() {
                                *sort_by = Some(match i {
                                    0 => ProcSortBy::Id,
                                    _ if i == header.len() - 5 || i == header.len() - 4 => {
                                        ProcSortBy::Cpu
                                    }
                                    _ if i == header.len() - 3 => ProcSortBy::DiskRead,
                                    _ if i == header.len() - 2 => ProcSortBy::DiskWrite,
                                    _ if i == header.len() - 1 => ProcSortBy::Memory,
                                    _ => ProcSortBy::Name,
                                })
                            }
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
    ui.label(millis_to_percent(stat.user_time_millis));
    ui.label(millis_to_percent(stat.system_time_millis));
    ui.label(Show::rate(stat.disk_read_bytes_per_second as f64, ""));
    ui.label(Show::rate(stat.disk_write_bytes_per_second as f64, ""));
    ui.label(Show::size(stat.mem_bytes as f64));
}
fn millis_to_percent(v: u32) -> String {
    format!("{}.{}%", v / 10, v % 10)
}
fn slice_concat(a: &[&'static str], b: &[&'static str]) -> Vec<&'static str> {
    let mut ret = Vec::new();
    ret.extend_from_slice(a);
    ret.extend_from_slice(b);
    ret
}
