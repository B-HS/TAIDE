use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use taide_model::{error::AppResult, ids::TabId};
use url::Url;

use crate::{
    preview::{Failure, MAX_TEXTURE_BYTES, invalid},
    preview_web::{Prepared, Request},
};

struct Entry {
    ready: Option<Prepared>,
    bytes: usize,
    failed: Option<Failure>,
    loading: Option<u64>,
    has_source: bool,
    needs_read: bool,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            ready: None,
            bytes: 0,
            failed: None,
            loading: None,
            has_source: false,
            needs_read: true,
        }
    }
}

impl Entry {
    fn invalidate(&mut self) {
        self.ready = None;
        self.bytes = 0;
        self.failed = None;
        self.loading = None;
        self.has_source = false;
        self.needs_read = true;
    }
}

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
    active: Option<Request>,
    token: u64,
}

impl Cache {
    pub fn active_request(&self) -> Option<&Request> {
        self.active.as_ref()
    }

    pub fn reconcile(&mut self, paths: &HashMap<TabId, String>) {
        let keep = paths.values().collect::<HashSet<_>>();
        self.entries.retain(|path, _| keep.contains(path));
        if self
            .active
            .as_ref()
            .is_some_and(|request| !self.entries.contains_key(&request.path))
        {
            self.active = None;
        }
    }

    pub fn invalidate(&mut self, path: &str) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.invalidate();
        }
        if self
            .active
            .as_ref()
            .is_some_and(|request| request.path == path)
        {
            self.active = None;
        }
    }

    pub fn invalidate_all(&mut self) {
        for entry in self.entries.values_mut() {
            entry.invalidate();
        }
        self.active = None;
    }

    pub fn invalidate_media(&mut self) {
        for (path, entry) in &mut self.entries {
            if matches!(
                crate::open_with::preview_kind(path),
                Some(crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video)
            ) {
                entry.invalidate();
            }
        }
        if self.active.as_ref().is_some_and(|request| {
            matches!(
                crate::open_with::preview_kind(&request.path),
                Some(crate::open_with::PreviewKind::Audio | crate::open_with::PreviewKind::Video)
            )
        }) {
            self.active = None;
        }
    }

    pub fn invalidate_root(&mut self, root: &str) {
        for (path, entry) in &mut self.entries {
            if Path::new(path).starts_with(root) {
                entry.invalidate();
            }
        }
        if self
            .active
            .as_ref()
            .is_some_and(|request| Path::new(&request.path).starts_with(root))
        {
            self.active = None;
        }
    }

    pub fn begin(&mut self, path: &str) -> Option<Request> {
        if self.active.is_some() {
            return None;
        }
        let entry = self.entries.entry(path.into()).or_default();
        if !entry.needs_read || entry.loading.is_some() {
            return None;
        }
        self.token = self.token.checked_add(1)?;
        let request = Request {
            path: path.into(),
            token: self.token,
        };
        entry.loading = Some(request.token);
        entry.needs_read = false;
        entry.has_source = false;
        self.active = Some(request.clone());
        Some(request)
    }

    pub fn source_ready(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        if let Some(entry) = self.entries.get_mut(&request.path)
            && entry.loading == Some(request.token)
        {
            entry.has_source = true;
            if matches!(entry.failed, Some(Failure::Read(_))) {
                entry.failed = None;
            }
        }
    }

    pub fn cancelled(&mut self, request: &Request) {
        if self.active.as_ref() != Some(request) {
            return;
        }
        self.active = None;
        if let Some(entry) = self.entries.get_mut(&request.path)
            && entry.loading == Some(request.token)
        {
            entry.loading = None;
            entry.has_source = false;
            entry.needs_read = true;
        }
    }

    pub fn retained_bytes(&self) -> usize {
        self.entries
            .values()
            .fold(0usize, |bytes, entry| bytes.saturating_add(entry.bytes))
    }

    pub fn accept(&mut self, request: Request, result: AppResult<Prepared>, other_bytes: usize) {
        if self.active.as_ref() != Some(&request) {
            return;
        }
        self.active = None;
        let used = self.retained_bytes();
        let Some(entry) = self.entries.get_mut(&request.path) else {
            return;
        };
        if entry.loading != Some(request.token) {
            return;
        }
        entry.loading = None;
        let result = result.and_then(|prepared| {
            let bytes = prepared
                .html
                .capacity()
                .checked_add(prepared.canonical.capacity())
                .and_then(|bytes| bytes.checked_add(prepared.source.as_str().len()))
                .and_then(|bytes| bytes.checked_add(size_of::<Prepared>() + size_of::<String>()));
            let Some(bytes) = bytes.filter(|bytes| {
                used.saturating_sub(entry.bytes)
                    .saturating_add(other_bytes)
                    .saturating_add(*bytes)
                    <= MAX_TEXTURE_BYTES
            }) else {
                return Err(invalid("preview document exceeds the shared cache budget"));
            };
            Ok((prepared, bytes))
        });
        match result {
            Ok((prepared, bytes)) => {
                entry.ready = Some(prepared);
                entry.bytes = bytes;
                entry.failed = None;
                entry.has_source = true;
            }
            Err(error) => {
                entry.ready = None;
                entry.bytes = 0;
                entry.failed = Some(if entry.has_source {
                    Failure::Decode(error)
                } else {
                    Failure::Read(error)
                });
            }
        }
    }

    pub fn source(&self, path: &str) -> Option<&Url> {
        self.entries
            .get(path)?
            .ready
            .as_ref()
            .map(|prepared| &prepared.source)
    }

    pub fn error(&self, path: &str) -> Option<&Failure> {
        self.entries.get(path)?.failed.as_ref()
    }

    pub fn renderer_failed(&mut self, path: &str) {
        if let Some(entry) = self.entries.get_mut(path) {
            entry.ready = None;
            entry.bytes = 0;
            entry.failed = Some(Failure::Decode(invalid(
                "isolated preview renderer is unavailable",
            )));
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};
    use taide_model::{ids::ProjectId, paths::AppPaths, project::Project};
    use taide_runtime::AppState;

    struct Fixture(PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn prepared(state: &AppState, source: &Path) -> Prepared {
        let approved = crate::preview_web::approval(state, source).unwrap();
        let (owner, _) = crate::preview_web_resource::Owner::admit(
            state.clone(),
            source.into(),
            approved.clone(),
        )
        .unwrap();
        Prepared {
            canonical: approved.canonical.clone(),
            source: crate::preview_web_document::source_url(&approved.canonical).unwrap(),
            html: Arc::new("synthetic".into()),
            owner,
            ticket: None,
        }
    }

    #[test]
    fn web_cache는_세대_취소_공유_view_상한과_폐기시_owner를_검사한다() {
        let fixture =
            Fixture(std::env::temp_dir().join(format!("taide-web-cache-{}", ProjectId::new())));
        let root = fixture.0.join("root");
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("synthetic.html");
        std::fs::write(&source, b"synthetic").unwrap();
        let path = source.to_str().unwrap();
        let state = AppState::new(AppPaths::new(fixture.0.join("data")));
        let project = ProjectId::new();
        state.projects.write().insert(
            project.clone(),
            Project {
                id: project,
                root: root.to_str().unwrap().into(),
                name: "synthetic cache".into(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: Default::default(),
            },
        );
        let mut cache = Cache::default();
        let old = cache.begin(path).unwrap();
        assert!(cache.begin(path).is_none());
        cache.invalidate(path);
        let current = cache.begin(path).unwrap();
        assert_ne!(old.token, current.token);
        let stale = prepared(&state, &source);
        let retired = stale.owner.scope();
        cache.source_ready(&old);
        cache.cancelled(&old);
        cache.accept(old, Ok(stale), 0);
        assert!(retired.is_closed());
        assert!(cache.source(path).is_none());
        cache.source_ready(&current);
        let document = prepared(&state, &source);
        let owner = document.owner.scope();
        cache.accept(current, Ok(document), 0);
        assert!(cache.source(path).is_some());
        assert!(cache.retained_bytes() > 0);
        let first = TabId::new();
        let second = TabId::new();
        cache.reconcile(&HashMap::from([
            (first, path.into()),
            (second.clone(), path.into()),
        ]));
        cache.reconcile(&HashMap::from([(second, path.into())]));
        assert!(!owner.is_closed());
        cache.invalidate_root(root.to_str().unwrap());
        assert!(owner.is_closed());
        assert_eq!(cache.retained_bytes(), 0);
        let request = cache.begin(path).unwrap();
        cache.source_ready(&request);
        let document = prepared(&state, &source);
        let owner = document.owner.scope();
        cache.accept(request, Ok(document), MAX_TEXTURE_BYTES);
        assert!(matches!(cache.error(path), Some(Failure::Decode(_))));
        assert!(owner.is_closed());
        cache.invalidate_all();
        let request = cache.begin(path).unwrap();
        cache.cancelled(&request);
        let request = cache.begin(path).unwrap();
        cache.accept(request, Err(invalid("synthetic read failure")), 0);
        assert!(matches!(cache.error(path), Some(Failure::Read(_))));
        cache.reconcile(&HashMap::new());
        assert!(cache.source(path).is_none());
    }
}
