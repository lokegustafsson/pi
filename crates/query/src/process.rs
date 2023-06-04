use crate::snapshot::Snapshot;

#[derive(Default, Clone, Debug)]
pub struct ProcessInfo {}
impl ProcessInfo {
    pub fn update(&mut self, new: &Snapshot) {}
}
