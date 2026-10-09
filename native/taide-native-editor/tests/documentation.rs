use lsp_types::{
    Documentation, Hover, HoverContents, MarkedString, MarkupContent, MarkupKind,
    ParameterInformation, ParameterLabel, Position, Range, SignatureHelp, SignatureInformation,
};
use taide_native_editor::documentation::{
    Content, HoverPart, SignatureTriggers, Signatures, parameter_range,
};

fn range(start: u32, end: u32) -> Range {
    Range::new(Position::new(0, start), Position::new(0, end))
}

#[test]
fn 호버는_언어코드와_markdown_plaintext_종류_배열_순서를_보존한다() {
    let hover = Hover {
        contents: HoverContents::Array(vec![
            MarkedString::from_markdown("**문서**".into()),
            MarkedString::from_language_code("rust".into(), "fn example() {}".into()),
            MarkedString::from_markdown(" \n ".into()),
        ]),
        range: None,
    };
    let part = HoverPart::new(hover, range(0, 5), Position::new(0, 2)).unwrap();
    assert_eq!(part.range, range(0, 5));
    assert_eq!(
        part.contents,
        vec![
            Content::Markdown("**문서**".into()),
            Content::Code {
                language: "rust".into(),
                text: "fn example() {}".into()
            },
        ]
    );
    assert_eq!(
        Content::from(MarkupContent {
            kind: MarkupKind::PlainText,
            value: "**그대로**".into()
        }),
        Content::Plain("**그대로**".into())
    );
    assert_eq!(
        Content::from(Documentation::String("원문".into())),
        Content::Plain("원문".into())
    );
}

#[test]
fn 빈_호버와_반전_범위_다른_위치의_응답은_표시하지않는다() {
    let make = |text: &str, span| Hover {
        contents: HoverContents::Scalar(MarkedString::String(text.into())),
        range: span,
    };
    assert!(HoverPart::new(make(" \n", None), range(0, 5), Position::new(0, 2)).is_none());
    assert!(
        HoverPart::new(
            make("문서", Some(range(5, 0))),
            range(0, 5),
            Position::new(0, 2)
        )
        .is_none()
    );
    assert!(
        HoverPart::new(
            make("문서", Some(range(3, 5))),
            range(0, 5),
            Position::new(0, 2)
        )
        .is_none()
    );
    assert_eq!(
        HoverPart::new(
            make("문서", Some(range(1, 3))),
            range(0, 5),
            Position::new(0, 2)
        )
        .unwrap()
        .range,
        range(1, 3)
    );
}

#[test]
fn 인자_오프셋은_utf16에서_byte로_변환하며_잘린_보조단위를_거절한다() {
    let label = "f(\u{1f642}, 값)";
    assert_eq!(
        parameter_range(label, &ParameterLabel::LabelOffsets([2, 4])),
        Some(2..6)
    );
    assert_eq!(
        parameter_range(label, &ParameterLabel::LabelOffsets([6, 7])),
        Some(8..11)
    );
    for offsets in [[3, 4], [2, 3], [7, 6], [0, 99]] {
        assert!(parameter_range(label, &ParameterLabel::LabelOffsets(offsets)).is_none());
    }
}

#[test]
fn 문자열_인자는_원본_js_word_경계와_정규식_특수문자_원문을_사용한다() {
    assert_eq!(
        parameter_range("maximum(max, x)", &ParameterLabel::Simple("max".into())),
        Some(8..11)
    );
    assert_eq!(
        parameter_range("f(x.y, [a])", &ParameterLabel::Simple("x.y".into())),
        Some(2..5)
    );
    assert_eq!(
        parameter_range("함수(값, x)", &ParameterLabel::Simple("값".into())),
        Some(7..10)
    );
    assert!(parameter_range("f(xyz)", &ParameterLabel::Simple("x".into())).is_none());
    assert!(parameter_range("f(x)", &ParameterLabel::Simple(String::new())).is_none());
}

fn signature(label: &str, active_parameter: Option<u32>) -> SignatureInformation {
    SignatureInformation {
        label: label.into(),
        documentation: None,
        parameters: Some(vec![
            ParameterInformation {
                label: ParameterLabel::Simple("x".into()),
                documentation: None,
            },
            ParameterInformation {
                label: ParameterLabel::Simple("y".into()),
                documentation: None,
            },
        ]),
        active_parameter,
    }
}

#[test]
fn 복수_서명은_양방향_순환하고_서명별_인자가_전역보다_우선한다() {
    let help = SignatureHelp {
        signatures: vec![signature("f(x, y)", None), signature("g(x, y)", Some(1))],
        active_signature: Some(99),
        active_parameter: Some(0),
    };
    let mut model = Signatures::new(help).unwrap();
    assert_eq!(model.index(), 0);
    assert_eq!(model.parameter_range(), Some(2..3));
    assert!(!model.next(false, false));
    assert!(model.next(true, true));
    assert_eq!(model.index(), 1);
    assert_eq!(model.parameter_range(), Some(5..6));
    assert!(!model.next(true, false));
    assert!(model.next(true, true));
    assert_eq!(model.index(), 0);
    assert!(model.next(false, true));
    assert_eq!(model.index(), 1);
}

#[test]
fn 빈_서명은_닫고_범위밖_활성인자는_강조와_인자문서를_생략한다() {
    assert!(
        Signatures::new(SignatureHelp {
            signatures: Vec::new(),
            active_signature: None,
            active_parameter: None
        })
        .is_none()
    );
    let model = Signatures::new(SignatureHelp {
        signatures: vec![signature("f(x, y)", None)],
        active_signature: None,
        active_parameter: Some(99),
    })
    .unwrap();
    assert_eq!(model.active().label, "f(x, y)");
    assert!(model.parameter().is_none());
    assert!(model.parameter_range().is_none());
}

#[test]
fn trigger는_provider_옵션을_합치고_마지막_입력과_활성_retrigger를_구분한다() {
    let options = [
        lsp_types::SignatureHelpOptions {
            trigger_characters: Some(vec!["(".into(), "".into()]),
            ..Default::default()
        },
        lsp_types::SignatureHelpOptions {
            trigger_characters: Some(vec![",".into()]),
            retrigger_characters: Some(vec![")".into()]),
            ..Default::default()
        },
    ];
    let triggers = SignatureTriggers::from_options(&options);
    assert!(triggers.matches("function(", false));
    assert!(triggers.matches("x,", false));
    assert!(triggers.matches("x)", true));
    assert!(!triggers.matches("x)", false));
    assert!(!triggers.matches("(x", true));
    assert!(!triggers.matches("", true));
    assert!(!SignatureTriggers::default().matches("(", true));
}
