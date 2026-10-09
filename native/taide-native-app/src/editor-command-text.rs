use std::cmp::Ordering;
use std::sync::OnceLock;

use icu_collator::{Collator, CollatorBorrowed};
use icu_locale_core::Locale;
use taide_model::error::{AppError, AppResult};
use taide_native_syntax::MonacoTextTransforms;

pub(crate) struct Resources {
    collator: CollatorBorrowed<'static>,
    pub(crate) transforms: MonacoTextTransforms,
}

impl Resources {
    fn new() -> AppResult<Self> {
        let locale = sys_locale::get_locale()
            .unwrap_or_else(|| "en-US".into())
            .replace('_', "-");
        let locale = locale
            .parse::<Locale>()
            .map_err(|error| AppError::Internal(format!("editor collation locale: {error}")))?;
        let collator = Collator::try_new(locale.into(), Default::default())
            .map_err(|error| AppError::Internal(format!("editor collation data: {error}")))?;
        let transforms = MonacoTextTransforms::new()
            .map_err(|error| AppError::Internal(format!("editor text transforms: {error:?}")))?;
        Ok(Self {
            collator,
            transforms,
        })
    }

    pub(crate) fn compare(&self, left: &str, right: &str) -> Ordering {
        self.collator.compare(left, right)
    }
}

pub(crate) fn resources() -> AppResult<&'static Resources> {
    static RESOURCES: OnceLock<AppResult<Resources>> = OnceLock::new();
    RESOURCES
        .get_or_init(Resources::new)
        .as_ref()
        .map_err(Clone::clone)
}
