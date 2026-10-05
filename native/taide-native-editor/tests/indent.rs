use taide_model::file::{EditorConfigIndentStyle, EditorConfigOptions};
use taide_native_editor::indent::{IndentOptions, resolve};

const BASE_SIZE: u32 = 4;
const SPACE_SIZE: u32 = 2;
const TAB_WIDTH: u32 = 8;

#[test]
fn 파일_들여쓰기는_원본_style별_크기_우선순위와_미지정_축을_보존한다() {
    let current = IndentOptions {
        tab_size: BASE_SIZE,
        insert_spaces: true,
    };
    assert_eq!(resolve(&EditorConfigOptions::default(), current), current);
    for (style, size, spaces) in [
        (EditorConfigIndentStyle::Space, SPACE_SIZE, true),
        (EditorConfigIndentStyle::Tab, TAB_WIDTH, false),
    ] {
        let config = EditorConfigOptions {
            indent_style: Some(style),
            indent_size: Some(SPACE_SIZE),
            tab_width: Some(TAB_WIDTH),
            ..Default::default()
        };
        assert_eq!(
            resolve(&config, current),
            IndentOptions {
                tab_size: size,
                insert_spaces: spaces,
            }
        );
    }
    for (config, expected) in [
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Space),
                ..Default::default()
            },
            IndentOptions {
                tab_size: BASE_SIZE,
                insert_spaces: true,
            },
        ),
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Tab),
                indent_size: Some(SPACE_SIZE),
                ..Default::default()
            },
            IndentOptions {
                tab_size: SPACE_SIZE,
                insert_spaces: false,
            },
        ),
        (
            EditorConfigOptions {
                indent_size: Some(SPACE_SIZE),
                ..Default::default()
            },
            IndentOptions {
                tab_size: SPACE_SIZE,
                insert_spaces: false,
            },
        ),
        (
            EditorConfigOptions {
                indent_style: Some(EditorConfigIndentStyle::Space),
                tab_width: Some(TAB_WIDTH),
                ..Default::default()
            },
            IndentOptions {
                tab_size: TAB_WIDTH,
                insert_spaces: true,
            },
        ),
    ] {
        assert_eq!(
            resolve(
                &config,
                IndentOptions {
                    insert_spaces: false,
                    ..current
                }
            ),
            expected
        );
    }
}
