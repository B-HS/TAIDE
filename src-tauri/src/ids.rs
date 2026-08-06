use serde::{Deserialize, Serialize};
use specta::Type;

macro_rules! string_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Type)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(format!("{}-{}", $prefix, uuid::Uuid::new_v4()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

string_id!(ProjectId, "prj");
string_id!(PaneId, "pane");
string_id!(TabId, "tab");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 아이디는_접두사를_가지고_유일하다() {
        let a = ProjectId::new();
        let b = ProjectId::new();

        assert!(a.as_str().starts_with("prj-"));
        assert_ne!(a, b);
    }

    #[test]
    fn 아이디는_문자열로_투명하게_직렬화된다() {
        let id = TabId("tab-fixed".to_string());
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"tab-fixed\"");
    }
}
