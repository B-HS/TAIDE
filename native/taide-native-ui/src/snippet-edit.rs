use std::sync::{Arc, Weak};

use taide_model::{
    error::{AppError, AppResult},
    snippet::SnippetFile,
};
#[cfg(feature = "native-host")]
use taide_runtime::{AppServices, AppState, snippet_actions};

use crate::settings_owner::Owner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    List,
    Save {
        file_name: String,
        content: String,
        create: bool,
    },
    Delete {
        file_name: String,
    },
}

#[derive(Clone)]
pub struct Request {
    owner: Owner,
    lifetime: Weak<()>,
    operation: Arc<()>,
    kind: Kind,
    #[cfg(feature = "native-host")]
    admitted: bool,
}

pub enum Outcome {
    Listed(Vec<SnippetFile>),
    Saved(SnippetFile),
    Deleted,
}

pub struct Reply {
    pub request: Request,
    pub result: AppResult<Outcome>,
}

impl Request {
    pub(crate) fn new(owner: Owner, lifetime: &Arc<()>, kind: Kind) -> Self {
        Self {
            owner,
            lifetime: Arc::downgrade(lifetime),
            operation: Arc::new(()),
            kind,
            #[cfg(feature = "native-host")]
            admitted: false,
        }
    }

    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }

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
    fn check(&self, state: &AppState) -> AppResult<()> {
        if state.is_shutting_down()
            || (!self.admitted && (!self.is_active() || !self.owner.is_active(state)))
        {
            return Err(AppError::Forbidden(
                "native snippet editor owner is no longer active".into(),
            ));
        }
        Ok(())
    }

    #[cfg(feature = "native-host")]
    pub fn admit(mut self, state: &AppState) -> AppResult<Self> {
        self.admitted = false;
        self.check(state)?;
        self.admitted = !matches!(self.kind, Kind::List);
        Ok(self)
    }

    #[cfg(feature = "native-host")]
    pub async fn execute(self, services: &AppServices) -> Reply {
        let result = match self.check(&services.state) {
            Ok(()) => {
                let request = self.clone();
                let state = services.state.clone();
                services
                    .tasks
                    .run_blocking_result("native-snippet-editor", move || {
                        request.check(&state)?;
                        match request.kind {
                            Kind::List => {
                                snippet_actions::snippet_list(&state).map(Outcome::Listed)
                            }
                            Kind::Save {
                                file_name, content, ..
                            } => snippet_actions::snippet_save(&state, file_name, content)
                                .map(Outcome::Saved),
                            Kind::Delete { file_name } => {
                                snippet_actions::snippet_delete(&state, file_name)
                                    .map(|()| Outcome::Deleted)
                            }
                        }
                    })
                    .await
            }
            Err(error) => Err(error),
        };
        Reply {
            request: self,
            result,
        }
    }
}

#[cfg(all(test, feature = "native-host"))]
mod tests {
    use super::*;
    use taide_model::{
        ids::{PaneId, ProjectId, TabId},
        layout::{PaneNode, Tab, TabKind},
        paths::AppPaths,
    };

    #[test]
    fn 승인된_snippet_쓰기만_편집기_unmount_이후_큐에서_계속되고_조회와_재승인은_거절된다() {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-m8-snippet-admission-{}", ProjectId::new())),
        ));
        let owner = Owner {
            project: ProjectId::new(),
            pane: PaneId::new(),
            tab: TabId::new(),
        };
        let mut layout = taide_layout::service::default_layout();
        layout.root = PaneNode::Leaf {
            id: owner.pane.clone(),
            tabs: vec![Tab {
                id: owner.tab.clone(),
                kind: TabKind::Settings,
                title: "Synthetic".into(),
                pinned: false,
                preview: false,
                dirty: false,
                view_state: None,
            }],
            active: Some(owner.tab.clone()),
        };
        layout.focused_pane = owner.pane.clone();
        state.layouts.write().insert(owner.project.clone(), layout);
        let lifetime = Arc::new(());
        let save = Request::new(
            owner.clone(),
            &lifetime,
            Kind::Save {
                file_name: "rust.json".into(),
                content: "{}".into(),
                create: false,
            },
        );
        let read = Request::new(owner.clone(), &lifetime, Kind::List)
            .admit(&state)
            .unwrap();
        let admitted = save.clone().admit(&state).unwrap();
        drop(lifetime);
        state.layouts.write().clear();
        assert!(!admitted.is_active());
        assert!(admitted.check(&state).is_ok());
        assert!(matches!(save.check(&state), Err(AppError::Forbidden(_))));
        assert!(matches!(read.check(&state), Err(AppError::Forbidden(_))));
        assert!(matches!(
            admitted.admit(&state),
            Err(AppError::Forbidden(_))
        ));
    }
}
