use std::collections::BTreeSet;

use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Meta, Token, punctuated::Punctuated};

pub fn is_product_source(path: &str) -> bool {
    let is_product_root = path.starts_with("src/")
        || path.starts_with("src-tauri/src/")
        || (path.starts_with("crates/") && path.contains("/src/"));
    if !is_product_root || path == "src/shared/api/bindings.ts" {
        return false;
    }
    if path.split('/').any(|component| {
        matches!(
            component,
            "tests"
                | "__tests__"
                | "fixtures"
                | "__fixtures__"
                | "generated"
                | "testing"
                | "test-support"
        )
    }) {
        return false;
    }
    let filename = path.rsplit('/').next().unwrap_or_default();
    if filename.contains(".test.") || filename.contains(".spec.") || filename == "test_support.rs" {
        return false;
    }
    path.ends_with(".rs") || path.ends_with(".ts") || path.ends_with(".tsx")
}

pub fn nonblank_physical_lines(source: &str) -> usize {
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn cfg_without_test(meta: &Meta) -> Option<bool> {
    match meta {
        Meta::Path(path) if path.is_ident("test") => Some(false),
        Meta::List(list) => {
            let predicates = list
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()?;
            let values = predicates.iter().map(cfg_without_test).collect::<Vec<_>>();
            if list.path.is_ident("all") {
                if values.contains(&Some(false)) {
                    return Some(false);
                }
                return values
                    .iter()
                    .all(|value| *value == Some(true))
                    .then_some(true);
            }
            if list.path.is_ident("any") {
                if values.contains(&Some(true)) {
                    return Some(true);
                }
                return values
                    .iter()
                    .all(|value| *value == Some(false))
                    .then_some(false);
            }
            if list.path.is_ident("not") && values.len() == 1 {
                return values[0].map(|value| !value);
            }
            None
        }
        _ => None,
    }
}

fn is_test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("test") {
            return true;
        }
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        attribute
            .parse_args::<Meta>()
            .ok()
            .and_then(|meta| cfg_without_test(&meta))
            == Some(false)
    })
}

#[derive(Default)]
struct TestLines {
    excluded: BTreeSet<usize>,
}

macro_rules! visit_test_item {
    ($method:ident, $item:ident) => {
        fn $method(&mut self, item: &'ast syn::$item) {
            if is_test_only(&item.attrs) {
                self.excluded
                    .extend(item.span().start().line..=item.span().end().line);
                return;
            }
            visit::$method(self, item);
        }
    };
}

impl<'ast> Visit<'ast> for TestLines {
    visit_test_item!(visit_item_mod, ItemMod);
    visit_test_item!(visit_item_fn, ItemFn);
    visit_test_item!(visit_item_struct, ItemStruct);
    visit_test_item!(visit_item_enum, ItemEnum);
    visit_test_item!(visit_item_impl, ItemImpl);
    visit_test_item!(visit_item_const, ItemConst);
    visit_test_item!(visit_item_static, ItemStatic);
    visit_test_item!(visit_item_use, ItemUse);
    visit_test_item!(visit_item_type, ItemType);
    visit_test_item!(visit_item_trait, ItemTrait);
    visit_test_item!(visit_item_macro, ItemMacro);
    visit_test_item!(visit_impl_item_fn, ImplItemFn);
    visit_test_item!(visit_impl_item_const, ImplItemConst);
    visit_test_item!(visit_impl_item_type, ImplItemType);
}

pub fn rust_product_lines(source: &str) -> Result<usize, syn::Error> {
    let syntax = syn::parse_file(source)?;
    let mut test_lines = TestLines::default();
    test_lines.visit_file(&syntax);
    Ok(source
        .lines()
        .enumerate()
        .filter(|(index, line)| {
            !line.trim().is_empty() && !test_lines.excluded.contains(&(index + 1))
        })
        .count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 제품_소스만_포함하고_테스트_생성_실험_도구는_제외한다() {
        for path in [
            "crates/taide-runtime/src/lib.rs",
            "src-tauri/src/main.rs",
            "src/widgets/editor.tsx",
        ] {
            assert!(is_product_source(path), "{path}");
        }
        for path in [
            "crates/taide-runtime/tests/port.rs",
            "src/features/editor.test.tsx",
            "src/test-support/fixture.ts",
            "src/shared/api/bindings.ts",
            "experiments/native-shell-spike/src/main.rs",
            "tools/migration-metrics/src/lib.rs",
            "docs/sample.ts",
        ] {
            assert!(!is_product_source(path), "{path}");
        }
    }

    #[test]
    fn 중간의_테스트_모듈만_제외하고_뒤의_제품_코드는_보존한다() {
        let source = "fn before() {}\n#[cfg(test)]\nmod tests {\nconst TEXT: &str = r#\"} fake brace\"#;\n}\nfn after() {}\n";
        assert_eq!(rust_product_lines(source).unwrap(), 2);
    }

    #[test]
    fn 복합_cfg는_테스트가_없으면_불가능한_항목만_제외한다() {
        let source = "#[cfg(all(test, unix))]\nstruct TestOnly;\n#[cfg(any(test, debug_assertions))]\nstruct DebugProduct;\n#[cfg(not(test))]\nfn product() {}\n";
        assert_eq!(rust_product_lines(source).unwrap(), 4);
    }

    #[test]
    fn 테스트_메서드는_제외하고_제품_메서드는_남긴다() {
        let source =
            "struct Product;\nimpl Product {\n#[cfg(test)]\nfn test_helper() {}\nfn run() {}\n}\n";
        assert_eq!(rust_product_lines(source).unwrap(), 4);
    }

    #[test]
    fn 잘못된_rust는_계측_성공으로_위장하지_않는다() {
        assert!(rust_product_lines("fn unfinished(").is_err());
        assert_eq!(
            nonblank_physical_lines("const text = 1\n\n// retained comment\n"),
            2
        );
    }
}
