use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeStyle {
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub font_style: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeSetting {
    pub scope: Option<Vec<String>>,
    pub settings: Option<ThemeStyle>,
}
