use crate::snapshot::{OldSnapshot, Snapshot};

#[derive(Default, Clone, Debug)]
pub struct ProcessInfo {}
impl ProcessInfo {
    pub(crate) fn update(&mut self, new: &Snapshot, old: &OldSnapshot) {
        let _ = new;
    }
}
