use std::sync::Arc;
use std::time::Duration;

use web_time::Instant;

use taide_model::{
    error::{AppError, AppResult},
    font::FontFamily,
    terminal::ShellProfile,
};

use crate::settings_owner::Owner;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fonts,
    Shells,
}

impl Kind {
    pub const ALL: [Self; 2] = [Self::Fonts, Self::Shells];

    pub fn command(self) -> &'static str {
        match self {
            Self::Fonts => "font_list",
            Self::Shells => "shell_profiles",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Fonts => 0,
            Self::Shells => 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    owner: Owner,
    kind: Kind,
    mount: Arc<()>,
}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.kind == other.kind
            && Arc::ptr_eq(&self.mount, &other.mount)
    }
}

impl Eq for Request {}

impl Request {
    pub fn owner(&self) -> &Owner {
        &self.owner
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn failed(self, error: AppError) -> Reply {
        match self.kind {
            Kind::Fonts => Reply::Fonts {
                request: self,
                result: Err(error),
            },
            Kind::Shells => Reply::Shells {
                request: self,
                result: Err(error),
            },
        }
    }

    #[cfg(feature = "native-host")]
    pub fn can_read(&self, state: &taide_runtime::AppState) -> bool {
        !state.is_shutting_down()
    }
}

pub enum Reply {
    Fonts {
        request: Request,
        result: AppResult<Vec<FontFamily>>,
    },
    Shells {
        request: Request,
        result: AppResult<Vec<ShellProfile>>,
    },
}

impl Reply {
    pub fn request(&self) -> &Request {
        match self {
            Self::Fonts { request, .. } | Self::Shells { request, .. } => request,
        }
    }
}

#[derive(Clone, Default)]
pub struct Resources {
    mount: Arc<()>,
    fonts: Option<AppResult<Arc<[FontFamily]>>>,
    shells: Option<AppResult<Arc<[ShellProfile]>>>,
    pending: Vec<Request>,
}

impl Resources {
    pub fn requests(&mut self, owner: &Owner) -> Vec<Request> {
        let requests = Kind::ALL
            .into_iter()
            .filter_map(|kind| {
                let complete = match kind {
                    Kind::Fonts => self.fonts.is_some(),
                    Kind::Shells => self.shells.is_some(),
                };
                (!complete && !self.pending.iter().any(|request| request.kind == kind)).then(|| {
                    Request {
                        owner: owner.clone(),
                        kind,
                        mount: Arc::clone(&self.mount),
                    }
                })
            })
            .collect::<Vec<_>>();
        self.pending.extend(requests.iter().cloned());
        requests
    }

    pub fn accept(&mut self, reply: Reply) -> bool {
        let kind = match &reply {
            Reply::Fonts { .. } => Kind::Fonts,
            Reply::Shells { .. } => Kind::Shells,
        };
        if reply.request().kind() != kind {
            return false;
        }
        let Some(index) = self
            .pending
            .iter()
            .position(|request| request == reply.request())
        else {
            return false;
        };
        self.pending.remove(index);
        match reply {
            Reply::Fonts { result, .. } => self.fonts = Some(result.map(Into::into)),
            Reply::Shells { result, .. } => self.shells = Some(result.map(Into::into)),
        }
        true
    }

    pub fn fonts(&self) -> Option<&[FontFamily]> {
        self.fonts
            .as_ref()
            .map(|result| result.as_deref().unwrap_or_default())
    }

    pub fn shells(&self) -> Option<&[ShellProfile]> {
        self.shells
            .as_ref()
            .map(|result| result.as_deref().unwrap_or_default())
    }

    pub fn error(&self, kind: Kind) -> Option<&AppError> {
        match kind {
            Kind::Fonts => self.fonts.as_ref().and_then(|result| result.as_ref().err()),
            Kind::Shells => self
                .shells
                .as_ref()
                .and_then(|result| result.as_ref().err()),
        }
    }

    pub fn refresh(&mut self) {
        *self = Self::default();
    }

    fn retry_failed(&mut self) {
        if self.error(Kind::Fonts).is_some() {
            self.fonts = None;
        }
        if self.error(Kind::Shells).is_some() {
            self.shells = None;
        }
    }

    fn expire(&mut self, kind: Kind) {
        match kind {
            Kind::Fonts => self.fonts = None,
            Kind::Shells => self.shells = None,
        }
    }
}

pub const RESOURCE_GC_TIME: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct Cache {
    resources: Resources,
    observed: bool,
    unused_since: [Option<Instant>; Kind::ALL.len()],
}

impl Cache {
    pub fn observe(&mut self, now: Instant, new_mount: bool) {
        self.collect(now);
        self.observed = true;
        self.unused_since.fill(None);
        if new_mount {
            self.resources.retry_failed();
        }
    }

    pub fn unobserve(&mut self, now: Instant) {
        if self.observed {
            self.observed = false;
            self.unused_since.fill(Some(now));
        }
        self.collect(now);
    }

    fn collect(&mut self, now: Instant) {
        if self.observed {
            return;
        }
        for kind in Kind::ALL {
            if self.unused_since[kind.index()]
                .is_some_and(|since| now.saturating_duration_since(since) >= RESOURCE_GC_TIME)
                && !self
                    .resources
                    .pending
                    .iter()
                    .any(|request| request.kind == kind)
            {
                self.resources.expire(kind);
                self.unused_since[kind.index()] = None;
            }
        }
    }

    pub fn requests(&mut self, owner: &Owner) -> Vec<Request> {
        self.resources.requests(owner)
    }

    pub fn snapshot(&self) -> Resources {
        self.resources.clone()
    }

    pub fn accept(&mut self, reply: Reply, now: Instant) -> bool {
        let kind = reply.request().kind;
        if !self.resources.accept(reply) {
            return false;
        }
        if !self.observed {
            self.unused_since[kind.index()] = Some(now);
        }
        true
    }

    pub fn refresh(&mut self) {
        self.resources.refresh();
    }
}
