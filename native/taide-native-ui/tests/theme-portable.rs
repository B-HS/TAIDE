use std::collections::BTreeMap;

use taide_model::{
    error::AppErrorKind,
    identifier::ensure_safe_component,
    ids::{PaneId, ProjectId, TabId},
    theme::{ResolvedTheme, SyntaxStyle, ThemeSummary, ThemeType},
};
use taide_native_ui::{
    settings_owner::Owner,
    theme_draft::{ColorDomain, Draft, Mode, SyntaxPatch},
    theme_edit::{Command, Reply, Session},
};

fn theme(id: &str) -> ResolvedTheme {
    ResolvedTheme {
        id: id.into(),
        name: "Synthetic theme".into(),
        theme_type: ThemeType::Dark,
        colors: BTreeMap::from([("app.accent".into(), "#123456".into())]),
        syntax: BTreeMap::from([(
            "keyword".into(),
            SyntaxStyle {
                fg: "#ABC".into(),
                bold: false,
                italic: false,
            },
        )]),
        terminal: BTreeMap::from([("red".into(), "#FF0000".into())]),
        token_colors: None,
        syntax_overrides: Vec::new(),
        warnings: vec!["synthetic warning".into()],
        author: Some("Synthetic author".into()),
        license: None,
        source: None,
    }
}

fn catalog() -> Vec<ThemeSummary> {
    [("taide-dark", true), ("synthetic-custom", false)]
        .into_iter()
        .map(|(id, builtin)| ThemeSummary {
            id: id.into(),
            name: "Synthetic theme".into(),
            theme_type: ThemeType::Dark,
            builtin,
        })
        .collect()
}

#[test]
fn 공용_테마초안은_조회출처_최소변경_복제와_검증을_보존한다() {
    let catalog = catalog();
    let mut draft = Draft::from_resolved(
        "synthetic-custom",
        Mode::Edit,
        "ignored".into(),
        &catalog,
        theme("synthetic-custom"),
        theme("taide-dark"),
    )
    .unwrap();
    assert!(!draft.has_unsaved_changes());
    assert_eq!(draft.changed_count(), 0);
    assert!(draft.build().unwrap().colors.is_empty());
    draft
        .set_color(ColorDomain::Colors, "app.accent", "#12345678".into())
        .unwrap();
    draft
        .set_syntax(
            "keyword",
            SyntaxPatch {
                bold: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(draft.has_unsaved_changes());
    assert_eq!(draft.changed_count(), 2);
    assert_eq!(draft.preview().syntax_overrides, ["keyword"]);
    assert!(draft.preview().warnings.is_empty());
    let built = draft.build().unwrap();
    assert_eq!(built.extends.as_deref(), Some("taide-dark"));
    assert_eq!(built.colors.len(), 1);
    assert_eq!(built.syntax.len(), 1);
    assert_eq!(built.author.as_deref(), Some("Synthetic author"));
    draft
        .reset_color(ColorDomain::Colors, "app.accent")
        .unwrap();
    draft.reset_syntax("keyword").unwrap();
    assert!(!draft.has_unsaved_changes());
    let before = draft.clone();
    assert!(
        draft
            .set_color(ColorDomain::Colors, "missing", "#fff".into())
            .is_err()
    );
    assert_eq!(draft, before);
    draft
        .set_color(ColorDomain::Terminal, "red", "bad".into())
        .unwrap();
    assert!(!draft.is_valid());
    assert!(draft.build().is_err());
    draft.reset_color(ColorDomain::Terminal, "red").unwrap();
    draft.rename(" ".into());
    assert!(draft.build().is_err());
    let copy = Draft::from_resolved(
        "taide-dark",
        Mode::Create,
        "Copy".into(),
        &catalog,
        theme("taide-dark"),
        theme("taide-dark"),
    )
    .unwrap();
    assert_eq!(copy.current().id, "synthetic-theme");
    assert_eq!(copy.current().name, "Copy");
    assert!(!copy.has_unsaved_changes());
    for (source, base) in [
        (theme("other"), theme("taide-dark")),
        (theme("synthetic-custom"), theme("taide-light")),
    ] {
        assert_eq!(
            Draft::from_resolved(
                "synthetic-custom",
                Mode::Edit,
                String::new(),
                &catalog,
                source,
                base,
            )
            .unwrap_err()
            .kind(),
            AppErrorKind::InvalidArgument
        );
    }
    assert!(
        Draft::from_resolved(
            "taide-dark",
            Mode::Edit,
            String::new(),
            &catalog,
            theme("taide-dark"),
            theme("taide-dark"),
        )
        .is_err()
    );
    for id in ["", ".", "..", "../outside", "a/b", "a\\b"] {
        assert_eq!(
            ensure_safe_component(id).unwrap_err().kind(),
            AppErrorKind::InvalidArgument
        );
    }
    assert!(ensure_safe_component("synthetic-custom").is_ok());
}

#[test]
fn 공용_편집요청은_동일_owner_재마운트와_폐기수명을_구별한다() {
    let owner = Owner {
        project: ProjectId::new(),
        pane: PaneId::new(),
        tab: TabId::new(),
    };
    let session = Session::new(owner.clone(), "synthetic-custom".into(), Mode::Edit).unwrap();
    let load = session.load_request(String::new());
    assert_eq!(load.owner(), &owner);
    assert_eq!(load.source_id(), "synthetic-custom");
    assert_eq!(load.mode(), Mode::Edit);
    assert_eq!(load.create_name(), "");
    assert!(load.same_request(&load.clone()));
    assert!(!load.same_request(&session.load_request(String::new())));
    let draft = load
        .resolve(&catalog(), theme("synthetic-custom"), theme("taide-dark"))
        .unwrap();
    let save = session.save_request(&draft).unwrap();
    let delete = session.delete_request().unwrap();
    assert_eq!(save.owner(), &owner);
    assert_eq!(delete.owner(), &owner);
    assert_eq!(save.source_id(), "synthetic-custom");
    assert_eq!(save.mode(), Mode::Edit);
    assert_eq!(save.theme().id, "synthetic-custom");
    assert_eq!(delete.source_id(), "synthetic-custom");
    assert!(save.same_request(&save.clone()));
    assert!(!save.same_request(&session.save_request(&draft).unwrap()));
    assert!(delete.same_request(&delete.clone()));
    assert!(!delete.same_request(&session.delete_request().unwrap()));
    let remounted = Session::new(owner.clone(), "synthetic-custom".into(), Mode::Edit).unwrap();
    assert!(!remounted.owns_load(&load));
    assert!(!remounted.owns_save(&save));
    assert!(!remounted.owns_delete(&delete));
    let reply = Command::Load(load.clone())
        .failed(taide_model::error::AppError::Forbidden("synthetic".into()));
    assert_eq!(reply.owner(), &owner);
    assert!(matches!(reply, Reply::Loaded { result: Err(_), .. }));
    drop(session);
    assert!(!load.is_active());
    assert!(!save.is_active());
    assert!(!delete.is_active());
    assert_eq!(
        load.resolve(&catalog(), theme("synthetic-custom"), theme("taide-dark"))
            .unwrap_err()
            .kind(),
        AppErrorKind::Forbidden
    );
    let create = Session::new(owner, "taide-dark".into(), Mode::Create).unwrap();
    assert!(create.save_request(&draft).is_err());
    assert!(create.delete_request().is_err());
}
