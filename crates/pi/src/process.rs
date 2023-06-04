use crate::Component;
use eframe::egui::Ui;
use ingest::ProcessInfo;

pub struct ProcessTab;
impl Component for ProcessTab {
    type Navigation = ();
    type Info = ProcessInfo;
    fn render(ui: &mut Ui, _: &mut Self::Navigation, info: &Self::Info) {
        ui.heading("Process table");
        let s = format!("{info:#?}");
        ui.label(&s[..s.len().min(10000)]);
    }
}
