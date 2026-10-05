use std::collections::BTreeMap;

use taide_model::identifier::ensure_safe_component;
use taide_model::{
    error::{AppError, AppResult},
    ids::TabId,
    theme::{ResolvedTheme, THEME_SCHEMA_VERSION, Theme, ThemeSummary, ThemeType},
};
#[cfg(feature = "native-host")]
use taide_runtime::{AppState, theme_actions};

const ID_SUFFIX_LENGTH: usize = 4;
const HEX_COLOR_LENGTHS: [usize; 3] = [3, 6, 8];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Create,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorDomain {
    Colors,
    Terminal,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyntaxPatch {
    pub fg: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Draft {
    source_id: String,
    mode: Mode,
    base: ResolvedTheme,
    loaded: ResolvedTheme,
    current: ResolvedTheme,
}

impl Draft {
    #[cfg(feature = "native-host")]
    pub fn load(
        state: &AppState,
        source_id: &str,
        mode: Mode,
        create_name: String,
    ) -> AppResult<Self> {
        if state.is_shutting_down() {
            return Err(AppError::Forbidden(
                "native theme draft host is closing".into(),
            ));
        }
        ensure_safe_component(source_id)?;
        let themes = theme_actions::theme_list(state)?;
        let summary = themes
            .iter()
            .find(|theme| theme.id == source_id)
            .ok_or_else(|| AppError::NotFound(format!("theme not found: {source_id}")))?;
        if mode == Mode::Edit && summary.builtin {
            return Err(AppError::InvalidArgument(
                "cannot edit a builtin theme".into(),
            ));
        }
        let source = theme_actions::theme_get(state, source_id.into())?;
        let base_id = if summary.builtin {
            source_id
        } else {
            builtin_id(source.theme_type)
        };
        let base = theme_actions::theme_get(state, base_id.into())?;
        Self::from_resolved(source_id, mode, create_name, &themes, source, base)
    }

    pub fn from_resolved(
        source_id: &str,
        mode: Mode,
        create_name: String,
        themes: &[ThemeSummary],
        source: ResolvedTheme,
        base: ResolvedTheme,
    ) -> AppResult<Self> {
        ensure_safe_component(source_id)?;
        ensure_safe_component(&base.id)?;
        let summary = themes
            .iter()
            .find(|theme| theme.id == source_id)
            .ok_or_else(|| AppError::NotFound(format!("theme not found: {source_id}")))?;
        if mode == Mode::Edit && summary.builtin {
            return Err(AppError::InvalidArgument(
                "cannot edit a builtin theme".into(),
            ));
        }
        let base_id = if summary.builtin {
            source_id
        } else {
            builtin_id(source.theme_type)
        };
        if source.id != source_id
            || source.theme_type != summary.theme_type
            || base.id != base_id
            || base.theme_type != source.theme_type
        {
            return Err(AppError::InvalidArgument(
                "theme draft inputs do not match their catalog".into(),
            ));
        }
        let mut current = source;
        if mode == Mode::Create {
            current.id = unique_id(&current.name, themes);
            current.name = create_name;
        } else {
            current.id = source_id.into();
        }
        Ok(Self {
            source_id: source_id.into(),
            mode,
            base,
            loaded: current.clone(),
            current,
        })
    }

    pub fn current(&self) -> &ResolvedTheme {
        &self.current
    }

    pub(crate) fn matches_source(&self, source_id: &str, mode: Mode) -> bool {
        self.source_id == source_id && self.mode == mode
    }

    pub fn base(&self) -> &ResolvedTheme {
        &self.base
    }

    pub fn rename(&mut self, name: String) {
        self.current.name = name;
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.current.name != self.loaded.name
            || self.current.colors != self.loaded.colors
            || self.current.syntax != self.loaded.syntax
            || self.current.terminal != self.loaded.terminal
    }

    pub fn color_changed(&self, domain: ColorDomain, key: &str) -> bool {
        let (base, current) = self.color_maps(domain);
        base.get(key) != current.get(key)
    }

    pub fn syntax_changed(&self, key: &str) -> bool {
        self.current.syntax.get(key) != self.base.syntax.get(key)
    }

    pub fn set_color(&mut self, domain: ColorDomain, key: &str, value: String) -> AppResult<()> {
        let current = match domain {
            ColorDomain::Colors => &mut self.current.colors,
            ColorDomain::Terminal => &mut self.current.terminal,
        };
        let token = current.get_mut(key).ok_or_else(|| {
            AppError::InvalidArgument(format!("unknown theme color token: {key}"))
        })?;
        *token = value;
        Ok(())
    }

    pub fn reset_color(&mut self, domain: ColorDomain, key: &str) -> AppResult<()> {
        let (base, _) = self.color_maps(domain);
        let value = base.get(key).cloned().ok_or_else(|| {
            AppError::InvalidArgument(format!("unknown base theme color token: {key}"))
        })?;
        self.set_color(domain, key, value)
    }

    pub fn set_syntax(&mut self, key: &str, patch: SyntaxPatch) -> AppResult<()> {
        let token = self.current.syntax.get_mut(key).ok_or_else(|| {
            AppError::InvalidArgument(format!("unknown theme syntax token: {key}"))
        })?;
        if let Some(fg) = patch.fg {
            token.fg = fg;
        }
        if let Some(bold) = patch.bold {
            token.bold = bold;
        }
        if let Some(italic) = patch.italic {
            token.italic = italic;
        }
        Ok(())
    }

    pub fn reset_syntax(&mut self, key: &str) -> AppResult<()> {
        let style = self.base.syntax.get(key).cloned().ok_or_else(|| {
            AppError::InvalidArgument(format!("unknown base theme syntax token: {key}"))
        })?;
        self.current.syntax.insert(key.into(), style);
        Ok(())
    }

    pub fn is_valid(&self) -> bool {
        !self.current.name.trim().is_empty()
            && self.current.colors.values().all(|value| valid_color(value))
            && self
                .current
                .terminal
                .values()
                .all(|value| valid_color(value))
            && self
                .current
                .syntax
                .values()
                .all(|style| valid_color(&style.fg))
    }

    pub fn changed_count(&self) -> usize {
        self.current
            .colors
            .keys()
            .filter(|key| self.color_changed(ColorDomain::Colors, key))
            .count()
            + self
                .current
                .terminal
                .keys()
                .filter(|key| self.color_changed(ColorDomain::Terminal, key))
                .count()
            + self
                .current
                .syntax
                .keys()
                .filter(|key| self.syntax_changed(key))
                .count()
    }

    pub fn preview(&self) -> ResolvedTheme {
        let mut preview = self.current.clone();
        preview.syntax_overrides = self
            .current
            .syntax
            .keys()
            .filter(|key| self.syntax_changed(key))
            .cloned()
            .collect();
        preview.warnings.clear();
        preview
    }

    pub fn build(&self) -> AppResult<Theme> {
        if !self.is_valid() {
            return Err(AppError::InvalidArgument(
                "native theme draft has an empty name or invalid color".into(),
            ));
        }
        ensure_safe_component(&self.current.id)?;
        ensure_safe_component(&self.base.id)?;
        Ok(Theme {
            version: THEME_SCHEMA_VERSION,
            id: self.current.id.clone(),
            name: self.current.name.clone(),
            theme_type: self.current.theme_type,
            extends: Some(self.base.id.clone()),
            palette: BTreeMap::new(),
            colors: self
                .current
                .colors
                .iter()
                .filter(|(key, _)| self.color_changed(ColorDomain::Colors, key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            syntax: self
                .current
                .syntax
                .iter()
                .filter(|(key, _)| self.syntax_changed(key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            terminal: self
                .current
                .terminal
                .iter()
                .filter(|(key, _)| self.color_changed(ColorDomain::Terminal, key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            token_colors: self
                .current
                .token_colors
                .clone()
                .filter(|rules| Some(rules) != self.base.token_colors.as_ref()),
            author: self.current.author.clone(),
            license: self.current.license.clone(),
            source: self.current.source.clone(),
        })
    }

    fn color_maps(
        &self,
        domain: ColorDomain,
    ) -> (&BTreeMap<String, String>, &BTreeMap<String, String>) {
        match domain {
            ColorDomain::Colors => (&self.base.colors, &self.current.colors),
            ColorDomain::Terminal => (&self.base.terminal, &self.current.terminal),
        }
    }
}

pub fn builtin_id(theme_type: ThemeType) -> &'static str {
    match theme_type {
        ThemeType::Dark => "taide-dark",
        ThemeType::Light => "taide-light",
    }
}

fn valid_color(value: &str) -> bool {
    let value = value.trim();
    if value.eq_ignore_ascii_case("transparent") {
        return true;
    }
    value.strip_prefix('#').is_some_and(|hex| {
        HEX_COLOR_LENGTHS.contains(&hex.len()) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn unique_id(name: &str, themes: &[ThemeSummary]) -> String {
    let mut slug = String::new();
    for character in name.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    let base = if slug.is_empty() {
        "custom-theme"
    } else {
        slug
    };
    if !themes.iter().any(|theme| theme.id == base) {
        return base.into();
    }
    loop {
        let entropy = TabId::new().to_string();
        let suffix: String = entropy
            .rsplit('-')
            .next()
            .unwrap_or_default()
            .chars()
            .take(ID_SUFFIX_LENGTH)
            .collect();
        let id = format!("{base}-{suffix}");
        if !themes.iter().any(|theme| theme.id == id) {
            return id;
        }
    }
}

#[cfg(all(test, feature = "native-host"))]
#[path = "../../taide-native-app/src/theme-draft-tests.rs"]
mod tests;
