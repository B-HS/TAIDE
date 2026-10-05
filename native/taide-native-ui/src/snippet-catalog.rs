use std::sync::{Arc, Weak};
use std::time::Duration;

use taide_model::{
    error::{AppError, AppResult},
    snippet::SnippetFile,
};
use web_time::Instant;

use crate::snippet_edit::{Kind, Outcome, Reply as EditorReply, Request as EditorRequest};

pub const STALE_TIME: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Request {
    lifetime: Weak<()>,
    operation: Arc<()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{settings_owner::Owner, snippet_completion};
    use std::collections::BTreeMap;
    use taide_model::{
        ids::{PaneId, ProjectId, TabId},
        snippet::{SnippetEntry, SnippetStringOrList},
    };

    const OUTLIVES_INACTIVE_GC: Duration = Duration::from_secs(601);

    fn observer(lifetime: &Arc<()>, owner: &Owner) -> EditorRequest {
        EditorRequest::new(owner.clone(), lifetime, Kind::List)
    }

    fn owner() -> Owner {
        Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        }
    }

    fn file(body: &str) -> SnippetFile {
        SnippetFile {
            file_name: "rust.json".into(),
            snippets: BTreeMap::from([(
                "Synthetic".into(),
                SnippetEntry {
                    prefix: SnippetStringOrList::Single("s".into()),
                    body: SnippetStringOrList::Single(body.into()),
                    description: None,
                    scope: None,
                },
            )]),
        }
    }

    #[test]
    fn 앱_전체_snippet_catalog는_관찰자_중복_신선도_실패_변경_세대와_단절을_보존한다() {
        let now = Instant::now();
        let mut catalog = Catalog::default();
        let first = catalog.next_read().unwrap();
        assert!(catalog.next_read().is_none());
        let lifetime = Arc::new(());
        let first_owner = owner();
        let second_owner = owner();
        catalog.observe(observer(&lifetime, &first_owner), now);
        catalog.observe(observer(&lifetime, &second_owner), now);
        assert!(catalog.next_read().is_none());
        assert!(catalog.accept(
            Reply {
                request: first.clone(),
                result: Ok(vec![file("old")])
            },
            now
        ));
        assert_eq!(catalog.take_finished().len(), 2);
        assert!(!catalog.accept(
            Reply {
                request: first,
                result: Ok(Vec::new())
            },
            now
        ));
        let old = catalog.snapshot().unwrap();
        catalog.observe(observer(&lifetime, &first_owner), now);
        assert_eq!(catalog.take_finished().len(), 1);
        assert!(catalog.next_read().is_none());
        let stale = now + STALE_TIME;
        assert!(!catalog.is_stale(stale - Duration::from_millis(1)));
        assert!(catalog.is_stale(stale));
        catalog.observe(observer(&lifetime, &first_owner), stale);
        assert_eq!(catalog.take_finished().len(), 1);
        let failed = catalog.next_read().unwrap();
        assert!(catalog.accept(
            failed.failed(AppError::Io("Synthetic read refusal".into())),
            stale
        ));
        assert!(catalog.error().is_some());
        assert!(catalog.next_read().is_none());
        assert!(Arc::ptr_eq(&catalog.snapshot().unwrap(), &old));
        catalog.observe(observer(&lifetime, &second_owner), stale);
        catalog.take_finished();
        let retry = catalog.next_read().unwrap();
        assert!(catalog.accept(
            Reply {
                request: retry,
                result: Ok(vec![file("old")])
            },
            stale
        ));
        assert!(catalog.error().is_none());
        assert!(Arc::ptr_eq(&catalog.snapshot().unwrap(), &old));
        let unmounted = stale + OUTLIVES_INACTIVE_GC;
        assert!(catalog.is_stale(unmounted));
        assert!(catalog.next_read().is_none());
        assert_eq!(catalog.files(), old.as_ref());
        catalog.observe(observer(&lifetime, &first_owner), unmounted);
        catalog.take_finished();
        let previous_generation = catalog.next_read().unwrap();
        catalog.invalidate();
        let current = catalog.next_read().unwrap();
        assert!(!current.same_request(&previous_generation));
        assert!(!catalog.accept(
            Reply {
                request: previous_generation,
                result: Ok(vec![file("stale")])
            },
            unmounted
        ));
        assert!(catalog.accept(
            Reply {
                request: current,
                result: Ok(vec![file("new")])
            },
            unmounted
        ));
        assert_eq!(
            snippet_completion::collect(catalog.files(), "rust")[0].body,
            "new"
        );
        assert_eq!(snippet_completion::collect(&old, "rust")[0].body, "old");
        catalog.invalidate();
        let deleted = catalog.next_read().unwrap();
        assert!(catalog.accept(
            Reply {
                request: deleted,
                result: Ok(Vec::new())
            },
            unmounted
        ));
        assert!(snippet_completion::collect(catalog.files(), "rust").is_empty());
        catalog.invalidate();
        let disconnected = catalog.next_read().unwrap();
        catalog.disconnected();
        assert!(catalog.next_read().is_none());
        assert!(!catalog.accept(
            Reply {
                request: disconnected,
                result: Ok(vec![file("late")])
            },
            unmounted
        ));
        catalog.reconnect(unmounted);
        let recovery = catalog.next_read().unwrap();
        drop(catalog);
        assert!(!recovery.is_active());
    }
}

pub struct Reply {
    pub request: Request,
    pub result: AppResult<Vec<SnippetFile>>,
}

impl Request {
    pub fn is_active(&self) -> bool {
        self.lifetime.upgrade().is_some()
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    pub fn failed(self, error: AppError) -> Reply {
        Reply {
            request: self,
            result: Err(error),
        }
    }

    #[cfg(feature = "native-host")]
    pub async fn execute(self, services: &taide_runtime::AppServices) -> Reply {
        let request = self.clone();
        let state = services.state.clone();
        let result = services
            .tasks
            .run_blocking_result("native-snippet-catalog", move || {
                if !request.is_active() || state.is_shutting_down() {
                    return Err(AppError::Forbidden(
                        "native snippet catalog is no longer active".into(),
                    ));
                }
                taide_runtime::snippet_actions::snippet_list(&state)
            })
            .await;
        Reply {
            request: self,
            result,
        }
    }
}

pub struct Catalog {
    lifetime: Arc<()>,
    files: Option<Arc<[SnippetFile]>>,
    updated_at: Option<Instant>,
    error: Option<AppError>,
    invalidated: bool,
    needs_read: bool,
    pending: Option<Request>,
    waiting: Vec<EditorRequest>,
    finished: Vec<EditorReply>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self {
            lifetime: Arc::new(()),
            files: None,
            updated_at: None,
            error: None,
            invalidated: true,
            needs_read: true,
            pending: None,
            waiting: Vec::new(),
            finished: Vec::new(),
        }
    }
}

impl Catalog {
    pub fn files(&self) -> &[SnippetFile] {
        self.files.as_deref().unwrap_or_default()
    }

    pub fn snapshot(&self) -> Option<Arc<[SnippetFile]>> {
        self.files.clone()
    }

    pub fn error(&self) -> Option<&AppError> {
        self.error.as_ref()
    }

    pub fn is_stale(&self, now: Instant) -> bool {
        self.invalidated
            || self
                .updated_at
                .is_none_or(|updated| now.saturating_duration_since(updated) >= STALE_TIME)
    }

    pub fn observe(&mut self, request: EditorRequest, now: Instant) {
        if !matches!(request.kind(), Kind::List) || !request.is_active() {
            return;
        }
        self.waiting
            .retain(|waiting| waiting.is_active() && waiting.owner() != request.owner());
        if let Some(files) = &self.files {
            self.finished.push(EditorReply {
                request,
                result: Ok(Outcome::Listed(files.to_vec())),
            });
        } else {
            self.waiting.push(request);
        }
        if self.is_stale(now) && self.pending.is_none() {
            self.needs_read = true;
        }
    }

    pub fn next_read(&mut self) -> Option<Request> {
        self.waiting.retain(EditorRequest::is_active);
        if !self.needs_read || self.pending.is_some() {
            return None;
        }
        self.needs_read = false;
        let request = Request {
            lifetime: Arc::downgrade(&self.lifetime),
            operation: Arc::new(()),
        };
        self.pending = Some(request.clone());
        Some(request)
    }

    pub fn accept(&mut self, reply: Reply, now: Instant) -> bool {
        if !reply.request.is_active()
            || !self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.same_request(&reply.request))
        {
            return false;
        }
        self.pending = None;
        match reply.result {
            Ok(files) => {
                if self.files.as_deref() != Some(files.as_slice()) {
                    self.files = Some(files.into());
                }
                self.updated_at = Some(now);
                self.error = None;
                self.invalidated = false;
                let files = self.files();
                let replies = self
                    .waiting
                    .iter()
                    .filter(|request| request.is_active())
                    .map(|request| EditorReply {
                        request: request.clone(),
                        result: Ok(Outcome::Listed(files.to_vec())),
                    })
                    .collect::<Vec<_>>();
                self.finished.extend(replies);
            }
            Err(error) => {
                self.finished.extend(
                    self.waiting
                        .iter()
                        .filter(|request| request.is_active())
                        .map(|request| request.clone().failed(error.clone())),
                );
                self.error = Some(error);
            }
        }
        self.waiting.clear();
        true
    }

    pub fn invalidate(&mut self) {
        self.invalidated = true;
        self.needs_read = true;
        self.pending = None;
    }

    pub fn disconnected(&mut self) {
        if let Some(request) = self.pending.clone() {
            self.accept(
                request.failed(AppError::Forbidden(
                    "remote snippet catalog connection is closed".into(),
                )),
                Instant::now(),
            );
        }
        self.invalidated = true;
        self.needs_read = false;
    }

    pub fn reconnect(&mut self, now: Instant) {
        if self.is_stale(now) {
            self.needs_read = true;
        }
    }

    pub fn take_finished(&mut self) -> Vec<EditorReply> {
        std::mem::take(&mut self.finished)
    }
}
