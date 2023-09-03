use crate::Component;
use eframe::egui::{self, Ui};
use procinfo::ProcInfo;

pub struct ProcessTab;
impl Component for ProcessTab {
    type Navigation = ();
    type Info = ProcInfo;
    fn render(ui: &mut Ui, _: &mut Self::Navigation, info: &Self::Info) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Process table");
            let s = format!("{info:#?}");
            ui.label(&s);
        });
    }
}
