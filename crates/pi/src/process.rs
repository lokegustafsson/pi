use crate::Component;
use eframe::egui::Ui;
use procinfo::ProcInfo;

pub struct ProcessTab;
impl Component for ProcessTab {
    type Navigation = ();
    type Info = ProcInfo;
    fn render(ui: &mut Ui, _: &mut Self::Navigation, info: &Self::Info) {
        ui.heading("Process table");
        let s = format!("{info:#?}");
        ui.label(&s[..s.len().min(10000)]);
    }
}
