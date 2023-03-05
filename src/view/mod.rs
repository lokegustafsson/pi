use eframe::egui::Ui;

pub mod process;
pub mod system;

pub trait Component {
    type Navigation;
    type Info;
    fn render(ui: &mut Ui, nav: &mut Self::Navigation, info: &Self::Info);
}
