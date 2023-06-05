use crate::snapshot::Snapshot;

#[derive(Default, Clone, Debug)]
pub struct ProcessInfo {}
impl ProcessInfo {
    pub(crate) fn update(&mut self, new: &Snapshot) {
        let _ = new;
    }
}
