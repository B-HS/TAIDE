use std::collections::HashMap;

use parking_lot::Mutex;

use crate::ids::ProjectId;

struct AuxiliaryWindowRecord {
    project_id: ProjectId,
    window_slot: u32,
}

#[derive(Default)]
pub struct WindowRegistry(Mutex<HashMap<String, AuxiliaryWindowRecord>>);

impl WindowRegistry {
    pub fn register(&self, label: String, project_id: ProjectId, window_slot: u32) {
        self.0.lock().insert(label, AuxiliaryWindowRecord { project_id, window_slot });
    }

    pub fn forget(&self, label: &str) -> Option<(ProjectId, u32)> {
        self.0.lock().remove(label).map(|record| (record.project_id, record.window_slot))
    }

    pub fn label_for(&self, project_id: &ProjectId, window_slot: u32) -> Option<String> {
        self.0
            .lock()
            .iter()
            .find(|(_, record)| &record.project_id == project_id && record.window_slot == window_slot)
            .map(|(label, _)| label.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_id(name: &str) -> ProjectId {
        ProjectId::from(format!("prj-{name}"))
    }

    #[test]
    fn 등록한_라벨로_프로젝트와_슬롯을_되찾는다() {
        let registry = WindowRegistry::default();
        registry.register("editor-1".to_string(), project_id("a"), 3);

        assert_eq!(registry.label_for(&project_id("a"), 3), Some("editor-1".to_string()));
        assert_eq!(registry.forget("editor-1"), Some((project_id("a"), 3)));
    }

    #[test]
    fn forget_은_멱등이라_두_번째_호출은_아무것도_돌려주지_않는다() {
        let registry = WindowRegistry::default();
        registry.register("editor-1".to_string(), project_id("a"), 1);

        assert!(registry.forget("editor-1").is_some());
        assert_eq!(registry.forget("editor-1"), None);
    }

    #[test]
    fn 등록되지_않은_라벨은_조회도_해제도_비어_있다() {
        let registry = WindowRegistry::default();

        assert_eq!(registry.forget("editor-9"), None);
        assert_eq!(registry.label_for(&project_id("a"), 1), None);
    }

    #[test]
    fn 같은_프로젝트의_다른_슬롯은_서로_다른_창으로_구분된다() {
        let registry = WindowRegistry::default();
        registry.register("editor-1".to_string(), project_id("a"), 1);
        registry.register("editor-2".to_string(), project_id("a"), 2);

        assert_eq!(registry.label_for(&project_id("a"), 1), Some("editor-1".to_string()));
        assert_eq!(registry.label_for(&project_id("a"), 2), Some("editor-2".to_string()));
        assert_eq!(registry.label_for(&project_id("a"), 3), None);
    }

    #[test]
    fn 슬롯_번호가_같아도_프로젝트가_다르면_다른_창이다() {
        let registry = WindowRegistry::default();
        registry.register("editor-1".to_string(), project_id("a"), 1);
        registry.register("editor-2".to_string(), project_id("b"), 1);

        assert_eq!(registry.label_for(&project_id("b"), 1), Some("editor-2".to_string()));
    }

    #[test]
    fn 해제된_창은_역방향_조회에서도_사라진다() {
        let registry = WindowRegistry::default();
        registry.register("editor-1".to_string(), project_id("a"), 1);
        registry.forget("editor-1");

        assert_eq!(registry.label_for(&project_id("a"), 1), None);
    }
}
