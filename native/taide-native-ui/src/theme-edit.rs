use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(feature = "native-host")]
use taide_model::app_event::AppEvent;
use taide_model::identifier::ensure_safe_component;
use taide_model::{
    error::{AppError, AppResult},
    theme::{Theme, ThemeSummary},
};
#[cfg(feature = "native-host")]
use taide_runtime::{AppServices, AppState, settings_actions, theme_actions};

use crate::{
    settings_owner::Owner,
    theme_draft::{Draft, Mode},
};

#[cfg(feature = "native-host")]
use crate::theme_draft::builtin_id;

pub struct Session {
    authority: Authority,
    source_id: String,
    mode: Mode,
}

#[derive(Clone)]
struct Authority {
    owner: Owner,
    active: Arc<AtomicBool>,
}

impl Authority {
    #[cfg(feature = "native-host")]
    fn check(&self, state: &AppState) -> AppResult<()> {
        if state.is_shutting_down()
            || !self.active.load(Ordering::Acquire)
            || !self.owner.is_active(state)
        {
            return Err(AppError::Forbidden(
                "native theme editor owner is no longer active".into(),
            ));
        }
        Ok(())
    }
}

impl Session {
    pub fn new(owner: Owner, source_id: String, mode: Mode) -> AppResult<Self> {
        ensure_safe_component(&source_id)?;
        Ok(Self {
            authority: Authority {
                owner,
                active: Arc::new(AtomicBool::new(true)),
            },
            source_id,
            mode,
        })
    }

    pub fn load_request(&self, create_name: String) -> LoadRequest {
        LoadRequest {
            operation: Arc::new(()),
            authority: self.authority.clone(),
            source_id: self.source_id.clone(),
            mode: self.mode,
            create_name,
        }
    }

    pub fn save_request(&self, draft: &Draft) -> AppResult<SaveRequest> {
        if !draft.matches_source(&self.source_id, self.mode) {
            return Err(AppError::InvalidArgument(
                "native theme draft belongs to another editor".into(),
            ));
        }
        self.save_theme_request(draft.build()?)
    }

    pub fn save_theme_request(&self, theme: Theme) -> AppResult<SaveRequest> {
        ensure_safe_component(&theme.id)?;
        if self.mode == Mode::Edit && theme.id != self.source_id {
            return Err(AppError::InvalidArgument(
                "native theme update identifier does not match its editor".into(),
            ));
        }
        Ok(SaveRequest {
            operation: Arc::new(()),
            authority: self.authority.clone(),
            source_id: self.source_id.clone(),
            mode: self.mode,
            theme,
        })
    }

    pub fn delete_request(&self) -> AppResult<DeleteRequest> {
        if self.mode != Mode::Edit {
            return Err(AppError::InvalidArgument(
                "an unsaved theme duplicate cannot be deleted".into(),
            ));
        }
        Ok(DeleteRequest {
            operation: Arc::new(()),
            authority: self.authority.clone(),
            source_id: self.source_id.clone(),
        })
    }

    pub fn owns_load(&self, request: &LoadRequest) -> bool {
        Arc::ptr_eq(&self.authority.active, &request.authority.active)
    }

    pub fn owns_save(&self, request: &SaveRequest) -> bool {
        Arc::ptr_eq(&self.authority.active, &request.authority.active)
    }

    pub fn owns_delete(&self, request: &DeleteRequest) -> bool {
        Arc::ptr_eq(&self.authority.active, &request.authority.active)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.authority.active.store(false, Ordering::Release);
    }
}

#[derive(Clone)]
pub struct LoadRequest {
    operation: Arc<()>,
    authority: Authority,
    source_id: String,
    mode: Mode,
    create_name: String,
}

impl LoadRequest {
    pub fn owner(&self) -> &Owner {
        &self.authority.owner
    }
    pub fn is_active(&self) -> bool {
        self.authority.active.load(Ordering::Acquire)
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn create_name(&self) -> &str {
        &self.create_name
    }

    pub fn resolve(
        &self,
        themes: &[ThemeSummary],
        source: taide_model::theme::ResolvedTheme,
        base: taide_model::theme::ResolvedTheme,
    ) -> AppResult<Draft> {
        if !self.is_active() {
            return Err(AppError::Forbidden(
                "native theme editor owner is no longer active".into(),
            ));
        }
        Draft::from_resolved(
            &self.source_id,
            self.mode,
            self.create_name.clone(),
            themes,
            source,
            base,
        )
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    #[cfg(feature = "native-host")]
    pub async fn execute(&self, services: &AppServices) -> AppResult<Draft> {
        self.authority.check(&services.state)?;
        let request = self.clone();
        let state = services.state.clone();
        services
            .tasks
            .run_blocking_result("native-theme-editor-load", move || {
                request.authority.check(&state)?;
                Draft::load(
                    &state,
                    &request.source_id,
                    request.mode,
                    request.create_name,
                )
            })
            .await
    }
}

#[derive(Clone)]
pub struct SaveRequest {
    operation: Arc<()>,
    authority: Authority,
    source_id: String,
    mode: Mode,
    theme: Theme,
}

impl SaveRequest {
    pub fn owner(&self) -> &Owner {
        &self.authority.owner
    }
    pub fn is_active(&self) -> bool {
        self.authority.active.load(Ordering::Acquire)
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    #[cfg(feature = "native-host")]
    pub async fn execute(&self, services: &AppServices) -> AppResult<ThemeSummary> {
        self.authority.check(&services.state)?;
        let mutation = services.state.begin_owned_mutation().await;
        let request = self.clone();
        let state = services.state.clone();
        let events = services.events.clone();
        services
            .tasks
            .run_blocking_result("native-theme-editor-save", move || {
                let _mutation = mutation;
                request.authority.check(&state)?;
                let themes = theme_actions::theme_list(&state)?;
                let existing = themes.iter().find(|theme| theme.id == request.theme.id);
                match (request.mode, existing) {
                    (Mode::Create, Some(_)) => {
                        return Err(AppError::InvalidArgument(
                            "native theme duplicate identifier is already occupied".into(),
                        ));
                    }
                    (Mode::Edit, Some(theme))
                        if !theme.builtin && theme.id == request.source_id => {}
                    (Mode::Edit, _) => {
                        return Err(AppError::InvalidArgument(
                            "native editable theme no longer exists".into(),
                        ));
                    }
                    (Mode::Create, None) => {}
                }
                let summary = theme_actions::theme_save(&state, request.theme)?;
                let theme_id = state.settings.read().theme_id.clone();
                events.publish(AppEvent::ThemeChanged { theme_id });
                Ok(summary)
            })
            .await
    }
}

#[derive(Clone)]
pub struct DeleteRequest {
    operation: Arc<()>,
    authority: Authority,
    source_id: String,
}

impl DeleteRequest {
    pub fn owner(&self) -> &Owner {
        &self.authority.owner
    }
    pub fn is_active(&self) -> bool {
        self.authority.active.load(Ordering::Acquire)
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn same_request(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.operation, &other.operation)
    }

    #[cfg(feature = "native-host")]
    pub async fn execute(&self, services: &Arc<AppServices>) -> AppResult<()> {
        self.authority.check(&services.state)?;
        let request = self.clone();
        let worker_services = services.clone();
        services
            .tasks
            .run_nonabortable_result("native-theme-editor-delete-operation", async move {
                request.execute_admitted(&worker_services).await
            })
            .await
    }

    #[cfg(feature = "native-host")]
    async fn execute_admitted(&self, services: &AppServices) -> AppResult<()> {
        let _mutation = services.state.begin_mutation().await;
        let request = self.clone();
        let state = services.state.clone();
        let theme_type = services
            .tasks
            .run_blocking_result("native-theme-editor-delete-prepare", move || {
                request.authority.check(&state)?;
                let themes = theme_actions::theme_list(&state)?;
                if !themes
                    .iter()
                    .any(|theme| theme.id == request.source_id && !theme.builtin)
                {
                    return Err(AppError::InvalidArgument(
                        "native editable theme no longer exists".into(),
                    ));
                }
                Ok(theme_actions::theme_get(&state, request.source_id)?.theme_type)
            })
            .await?;
        self.authority.check(&services.state)?;
        let current = services.state.settings.read().clone();
        if current.theme_id == self.source_id {
            let mut next = current;
            next.theme_id = builtin_id(theme_type).into();
            let broadcasted = settings_actions::apply_and_broadcast(
                &services.state,
                next,
                |_, _| async {},
                services.events.as_ref(),
            )
            .await?;
            if !broadcasted.follow_system_theme {
                services.events.publish(AppEvent::ThemeChanged {
                    theme_id: broadcasted.theme_id,
                });
            }
        }
        let request = self.clone();
        let state = services.state.clone();
        services
            .tasks
            .run_blocking_result("native-theme-editor-delete", move || {
                request.authority.check(&state)?;
                theme_actions::theme_delete(&state, request.source_id)
            })
            .await?;
        let theme_id = services.state.settings.read().theme_id.clone();
        services.events.publish(AppEvent::ThemeChanged { theme_id });
        Ok(())
    }
}

#[derive(Clone)]
pub enum Command {
    Load(LoadRequest),
    Save(Box<SaveRequest>),
    Delete(DeleteRequest),
}

pub enum Reply {
    Loaded {
        request: LoadRequest,
        result: AppResult<Box<Draft>>,
    },
    Saved {
        request: Box<SaveRequest>,
        result: AppResult<ThemeSummary>,
    },
    Deleted {
        request: DeleteRequest,
        result: AppResult<()>,
    },
}

impl Command {
    #[cfg(feature = "native-host")]
    pub async fn execute(self, services: &Arc<AppServices>) -> Reply {
        match self {
            Self::Load(request) => {
                let result = request.execute(services).await.map(Box::new);
                Reply::Loaded { request, result }
            }
            Self::Save(request) => {
                let result = request.execute(services).await;
                Reply::Saved { request, result }
            }
            Self::Delete(request) => {
                let result = request.execute(services).await;
                Reply::Deleted { request, result }
            }
        }
    }

    pub fn failed(self, error: AppError) -> Reply {
        match self {
            Self::Load(request) => Reply::Loaded {
                request,
                result: Err(error),
            },
            Self::Save(request) => Reply::Saved {
                request,
                result: Err(error),
            },
            Self::Delete(request) => Reply::Deleted {
                request,
                result: Err(error),
            },
        }
    }
}

impl Reply {
    pub fn error(&self) -> Option<&AppError> {
        match self {
            Self::Loaded { result, .. } => result.as_ref().err(),
            Self::Saved { result, .. } => result.as_ref().err(),
            Self::Deleted { result, .. } => result.as_ref().err(),
        }
    }

    pub fn owner(&self) -> &Owner {
        match self {
            Self::Loaded { request, .. } => &request.authority.owner,
            Self::Saved { request, .. } => &request.authority.owner,
            Self::Deleted { request, .. } => &request.authority.owner,
        }
    }
}
