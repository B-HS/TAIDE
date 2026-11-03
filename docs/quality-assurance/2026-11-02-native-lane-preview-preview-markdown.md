# lane preview 묶음 E2 (preview-markdown) — 미리보기·Markdown

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/preview`(branch `lane/preview`) · 사양 `docs/research/2026-10-28-lane-editor-preview-spec.md` 2.10·2.18~2.22·3절 E2 · 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(결정 7 A, 8 A, 9 A, 10 A)
> 경로 약어는 사양 0절과 같습니다. 이 문서의 `APP`·`UI`는 lane worktree 기준입니다.

## 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| preview-3 | complete | 보조 창 파일 탭 메뉴의 다시 열기(Editor/Preview) `[A]` 추가. 주 창 `[A]`는 shell-78 작업에 이미 있음 |
| preview-6 | complete | 이미지 읽기/해석 실패 분리(결정 8 A), HTML·오디오·비디오 준비 중 빈 배경, 도달하지 않던 디버그 문구 제거. `[A]` 검사 함수 2개(실패 화면·외부 열기 toast 1개, 준비 중 빈 배경 1개는 리뷰 반영 때 추가) |
| preview-5 | partial | 외부 수정(PNG·CSV)·두 pane 공유·외부 삭제·프로젝트 닫힘 `[A]`. 남은 것은 아래 "남은 범위" |
| preview-14 | partial | 결정 9 A(theme 변경 시 열린 media WebView에 host 스크립트로 CSS만 갱신, 재생 유지) 구현과 `[C]`. 실제 재생·seek·AX는 GUI 허락 항목 |
| preview-15 | partial | preview-14와 같은 경로(오디오) |
| preview-38 | not-applicable | 결정 10 A. 원본 출시 대상이 macOS뿐 |
| editor-72 | partial | 결정 7 A 목록 중 `kbd`/`samp` 표시를 고침(`[C]`·`[U]`). 나머지 목록은 "남은 범위" |

## preview-3 탭 우클릭 "다른 편집기로 다시 열기"

- 원본: 사양 2.18(`TS/features/tab/tab-context-menu.tsx:58-59, 93, 150-158`, `TS/widgets/editor-area/pane-tab-bar.tsx:183, 226-227`). 주 창·보조 창 공통 메뉴.
- 구현: 새 코드 없음. 기존 `UI/shell.rs` 파일 탭 메뉴 → `intents::preview::Intent::ReopenWith` → `domains/preview.rs` `apply_preview_intent` → `open_with::Registry::set`이 보조 창 viewport에서도 같은 경로로 동작함을 확인했습니다.
- 검사 `[A]`: `APP/application-auxiliary-window-tests.rs`의 `실제_app_보조_창_파일_탭_메뉴의_다시열기는_편집기와_미리보기를_전환한다`. 보조 창 1개(slot 1)에 `pic.png` 탭만 두고, 그 viewport에 우클릭 → `tab.reopenEditorWith` 하위 메뉴 hover → `explorer.openWithEditor` 클릭 → `open_with` 표면이 `Editor`가 되고 보조 창에 파일 본문 텍스트가 그려짐 → 다시 `explorer.openWithPreview` → `Preview(Image)`.
- 대응표 정정: missing은 낡은 판정(사양 1.3). 주 창 근거는 `application-tab-menu-tests.rs`의 `실제_app_탭은_에이전트_터미널_glyph와_다시열기_탐색기_reveal_rename을_반영한다`.

## preview-6 외부 앱으로 열기 버튼과 UnsupportedPreview

- 원본: 사양 2.20(`TS/widgets/preview-pane/preview-pane.tsx:48-86`, `TS/features/preview/unsupported-preview.tsx:11-27`, `image-preview.tsx`).
    - 바이트가 필요한 형식의 읽기 실패 → `UnsupportedPreview`(아이콘·`preview.notSupported`·파일 이름·외부 열기 버튼). 외부 열기 실패 → 오류 toast.
    - 이미지 해석 실패 → 브라우저 깨진 이미지(대체 문구 = 파일 이름). 결정 8 A로 파일 이름만 가운데 표시.
    - HTML 읽는 중 → 빈 편집기 배경. 오디오·비디오 → 바로 요소(바이트를 읽지 않음).
- 구현:
    - `APP/preview.rs`: `read`가 `Result<Raster, Failure>`를 돌려줍니다. `read_approved` 자체 오류(승인·종료·파일 읽기)는 `Failure::Read`, `decode` 오류는 `Failure::Decode`(PDF·HWP 등 다른 형식의 기존 분리 방식과 같음). `Cache`의 `Entry::Failed`가 `Failure`를 담고, 응답 수락 단계의 검사 실패(픽셀 길이·애니메이션 예산·texture 상한)는 `Decode`로 둡니다. `accept`/`accept_with_other_bytes`/`error`의 타입이 바뀌었습니다.
    - `APP/domains/preview.rs` `show_preview_tab`: 이미지 `Decode`는 `centered_and_justified` 안에 파일 이름 label만, `Read`는 `preview_status::show`(다른 형식과 같은 UnsupportedPreview 화면·외부 열기 → `Shell::OpenPath`). HTML·오디오·비디오는 WebView 준비 전(source 없음·실패 없음)에 아무것도 그리지 않고(편집기 배경), 실패가 있을 때만 `preview_status::show`를 그립니다. 마지막 `native preview provider is not connected` 분기는 `PreviewKind` 8종이 모두 앞에서 처리돼 도달하지 않으므로 제거했고, 두 곳이 쓰는 `preview_status::Appearance`는 한 번만 만듭니다. 쓰이지 않게 된 `presentation` import를 지웠습니다.
    - `Reply::Preview.result` 타입을 `Result<Raster, Failure>`로 맞췄습니다. 통합 테스트 `tests/preview-animation.rs` 한 곳(`decode(..)`를 바로 `accept`에 넘기던 곳)은 바뀐 타입에 맞게 `.map_err(Failure::Decode)`만 붙였습니다(기대값 변경 없음).
- 검사 `[A]`(새 `APP/application-preview-tests.rs`):
    - `실제_app_이미지_미리보기는_읽기_실패와_해석_실패를_원본_화면으로_나누고_외부_열기_실패를_알린다`: 이미지가 아닌 바이트의 `broken.png` → `Failure::Decode`, 본문에 파일 이름 텍스트만 있고 `preview.notSupported` 문구·외부 열기 버튼 없음. 탭을 눌러 프로젝트 밖 `outside.png`(존재하는 PNG, 승인 밖이라 읽기 거절)로 전환 → `Failure::Read`, 본문에 `preview.notSupported`·파일 이름·외부 열기 버튼. 버튼 클릭 → `Shell::OpenPath` → `system_open_path`가 프로젝트 밖 경로를 거절 → 같은 오류의 `describe_error` 문구가 Error toast로 표시.
    - `실제_app_html과_오디오_미리보기는_준비되기_전에_로딩_표시_없이_빈_배경을_둔다`(리뷰 반영 때 추가, 아래 "수정" 절): `page.html`·`song.mp3` 두 pane에서 web 요청이 처음 시작된 프레임(그 프레임의 본문은 source·실패가 모두 없는 준비 중 분기)에 `common.loading`·`preview.notSupported` 문구가 없음.
    - 외부 열기 검사에 프로젝트 밖 경로를 쓴 이유: 앱 lib 테스트에는 platform 주입 포트가 없고(`bootstrap::NativePlatform`이 `/usr/bin/open` 실행), 프로젝트 안 파일(권한 0 포함)은 승인 검사를 통과해 실제 외부 앱 열기에 닿습니다. 승인 거절은 platform 호출 전에 끝나므로 실제 OS 동작 없이 실패 toast까지 확인됩니다. 실제 외부 앱이 열리는 성공 경로는 사양 4.2-5 허락 항목입니다.
- 원본과의 차이 기록: 없음(결정 8 A가 정한 대응).

## preview-5 외부 저장·이동·삭제 시 미리보기 갱신/무효화

- 원본: 사양 2.19. 단, 사양의 "PDF는 다시 읽으면 1쪽으로 돌아가고 배율은 유지"는 실제 코드와 다릅니다. `TS/features/preview/pdf-preview.tsx`의 load effect는 새 문서를 읽을 때 `setCurrentPage(1)`과 함께 `setScale(PDF_INITIAL_SCALE)`도 호출하므로 배율도 처음 값으로 돌아갑니다. native `preview_pdf::Cache::invalidate`가 `View::new`로 page·zoom을 모두 기본값으로 되돌리는 현재 동작이 원본과 같습니다(코드 변경 없음).
- 구현: 결함이 나오지 않아 코드 변경 없음. 근거 경로는 `APP/events.rs` `TreeChanges::record_preview` → `domains/preview.rs` `reconcile_previews`(형식별 캐시 무효화, 프로젝트에 없는 경로 제거) → 다음 프레임 `load_preview_requests`.
- 검사 `[A]` `실제_app_미리보기는_외부_수정과_삭제와_프로젝트_닫힘을_여러_뷰에_반영한다`: 가로 3분할(같은 `pixels.png` 두 pane + `table.csv`).
    1. 첫 표시: 이미지 texture 2x1이 두 pane에 같은 texture id로 2번 그려지고 CSV 셀 `first-cell`이 본문에 그려짐.
    2. 외부 수정: PNG를 3x1로, CSV 셀을 `next-cell`로 덮어쓰고 `AppEvent::FsChanged(Modified)`를 기록 → 새 texture 3x1이 두 pane에 2번, 본문에 `next-cell`, `first-cell` 없음.
    3. 외부 삭제: PNG를 지우고 `Removed` 기록 → `Failure::Read`, 두 pane 모두 `preview.notSupported`·파일 이름 표시, 탭 3개 유지(원본처럼 탭을 닫지 않음).
    4. 다시 만들기(`Created`) → 2x1 texture 복구.
    5. 프로젝트 닫힘(`close_project`) → 이미지 캐시(texture·실패) 제거, CSV workbook 제거, 프로젝트 해제.

## preview-14 비디오 / preview-15 오디오

- 원본: 사양 2.21. theme 변경은 CSS 변수만 바뀌어 재생 위치·일시정지·볼륨이 유지됩니다.
- 착수 전 확인: `vendor/wry-preview/src/wkwebview/mod.rs:349-356`는 `with_javascript_disabled()`를 `WKWebpagePreferences.allowsContentJavaScript = false`로 적용합니다. 이 설정은 페이지 스크립트만 막고 앱이 `evaluateJavaScript`로 넣는 스크립트는 막지 않으므로 결정 9 A의 전제가 성립합니다. 페이지 JS는 계속 끕니다.
- 구현:
    - `APP/preview_web_media.rs`: 문서의 `<style>` 내용을 만드는 `style(appearance)`를 꺼내 문서 생성과 새 `restyle_script(appearance)`(같은 style 문자열을 JSON 문자열로 넣은 `document.querySelector('style').textContent=..;`)가 함께 씁니다.
    - `APP/preview_web_cache.rs`: `Entry.restyled`. `restyle_media(restyled)`는 열린 view가 새 색을 받은 준비된 media 문서만 유지하고(재생 중인 media URL의 ticket·scope도 유지) 나머지 media는 기존처럼 무효화합니다. `release_restyled(placed)`는 이번 프레임 placement마다 (경로, 이미 열린 view가 있는지)를 받아, 경로의 모든 placement가 이미 열린 view일 때만 restyled 문서를 유지하고 나머지(표시되지 않음·새 view가 생길 탭이 있음)는 무효화합니다(발행된 문서는 옛 색이라 다시 읽음). 리뷰 반영 때 의미를 바꿨고 `invalidate_media()`는 지웠습니다(아래 "수정" 절).
    - `APP/preview_web_view.rs`(파일 끝 새 `impl Views` 블록, 구조 변경 없음): `open_tabs()`, `evaluate(tabs, script)`(macOS child의 `wry::WebView::evaluate_script`, 비macOS는 빈 집합).
    - `APP/domains/preview.rs`: `tick_preview_media`가 appearance 변경 때 오디오·비디오 탭의 열린 view에 `restyle_script`를 보내고, 닿은 탭의 경로만 `restyle_media`로 유지합니다. `show_preview_web_views`는 sync 전에 `release_restyled`를 호출합니다(리뷰 반영 때 sync 뒤에서 앞으로 옮김).
- 검사 `[C]`: `preview_web_media::tests::web_media_restyle_script는_문서와_같은_style만_새_색으로_바꾼다`, `preview_web_cache::tests::web_cache_restyle은_새_색을_받은_view만_유지하고_새_view나_닫힘이면_다시_읽는다`(유지된 문서의 source·scope 유지, 나머지 media scope 닫힘, HTML 영향 없음, 같은 경로에 새 view가 생길 placement가 있으면 다시 읽기, 표시되지 않으면 다시 읽기, 새로 읽은 문서는 release 대상 아님). 기존 `tests/preview-web-media.rs`의 media 무효화 경로는 `restyle_media(&HashSet::new())`로 그대로 통과해야 합니다(아래 검증).
- 남은 것: 실제 WKWebView에서 스크립트 적용·재생 유지·seek·컨트롤·키보드/AX 동작은 GUI 실행이 필요해 사양 4.2-2 허락 항목입니다. 그래서 partial을 유지합니다.

## preview-38 비macOS(Windows/Linux)

- 결정 10 A: 원본 제품의 빌드·출시는 macOS뿐(`.github/workflows/ci.yml`·`release.yml`, 사양 2.22)이므로 not-applicable로 정정합니다.
- 코드 대조(비macOS 컴파일 확인은 `rustup target add`가 필요한 환경 변경이라 하지 않음): 비macOS에서 PDF `preview_pdf::decode`(84줄 분기)는 오류 → `Failure::Decode` → `preview_status` 해석 실패 화면과 외부 열기 버튼. AVIF·ICC(`preview.rs` 100·108줄 분기 밖)는 `image` crate 경로의 해석 오류 → 이미지 `Decode` → 파일 이름 화면(이번 preview-6 변경). WebView `Views::sync`(비macOS)는 placement가 있으면 `Err` → `show_preview_web_views`가 `renderer_failed` → `Failure::Decode` → `preview_status` 화면. 이번에 추가한 `open_tabs`·`evaluate`는 비macOS에서 빈 집합을 돌려 패닉 경로가 없습니다.

## editor-72 Markdown 미리보기 분할 (결정 7 A 범위)

- 원본: 사양 2.10. 렌더는 `marked` → `DOMPurify.sanitize`, 표시 클래스는 `TS/features/editor/markdown-preview.tsx`. Tailwind 선택자는 `code`만 배경 상자로 꾸미고, `kbd`/`samp`는 브라우저 기본 스타일(`font-family: monospace`)만 받습니다.
- 이전 상태: parser(`APP/editor-markdown-html.rs`)는 `kbd`/`samp`를 `style.code = true`, `code_element = None`으로 만들고, 미리보기 UI는 `style.code && code_frame`이면 모두 code 상자(배경·둥근 모서리·좌우 4px)와 너비 padding을 적용해 원본에 없는 상자를 그렸습니다.
- 구현(UI, 미리보기 한정):
    - `UI/editor-markup.rs` `inlines`: 미리보기(`is_preview`)에서는 `code_element`가 있는 span만 code 상자를 그립니다. 비미리보기(LSP 문서 등 legacy `None`)는 기존대로 상자를 그립니다.
    - `UI/editor-markup-preview.rs` `inline_widths`: code padding 너비를 `code_element`가 있는 요소의 첫 span에만 더합니다(그리기와 같은 기준).
    - 미리보기 문서는 `editor-markdown-html.rs` `inlines`가 모든 `code` 요소에 `Some` 식별자를 주므로(`show_preview`의 유일한 호출자는 `APP/editor-markdown.rs`) 미리보기의 `None` code span은 `kbd`/`samp`뿐입니다.
- 검사:
    - `[C]` `editor-markdown-parse-tests.rs`의 `kbd와_samp는_monospace지만_code_요소_박스가_아니다`(kbd·samp는 `code` 스타일·식별자 없음, `code` 안의 kbd는 그 code 요소 식별자를 받음).
    - `[U]` `UI/tests/editor-markup-preview.rs`의 `미리보기_kbd와_samp는_코드_요소가_아니라_배경_없이_monospace만_쓴다`(식별자 없는 code span: 배경 rect 없음·Monospace, 식별자 있는 code: 배경 rect·Monospace).
    - 기존 `[U]` `인라인_코드_프레임은_원본_둥근_배경과_링크_클릭을_보존한다`의 입력 span을 `code_element: None` → `Some(0)`으로 바꿨습니다. 이 검사는 Markdown 미리보기의 `<a><code>` 상자와 링크 클릭을 보는 검사인데, 실제 parser는 그 경우 `Some`을 만듭니다. 기대값(상자·둥근 모서리·링크 1회)은 바꾸지 않았습니다.
- `[A]`: 기존 `application::markdown_tests` 4건을 다시 실행해 실제 앱 경로(main.rs → `NativeApplication` → `editor-markdown.rs` → `show_preview`)가 유지됨을 확인했습니다. `kbd` 전용 `[A]`는 이번에 만들지 않았습니다(남은 범위).

## 검증

모두 lane worktree, `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/preview`.

| 명령 | 결과 |
| --- | --- |
| `cargo build --quiet --manifest-path native/taide-native-app/Cargo.toml --example native-lsp-mock` | 성공(LSP 검사 준비) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- application::preview_tests 실제_app_보조_창_파일_탭_메뉴의_다시열기는 --test-threads=2` | 첫 실행: 보조 창 다시 열기 1 통과, 새 preview 2건은 판정 방식 문제로 실패(accesskit Label 대신 본문 텍스트 shape·`Shape::texture_id`로 판정하도록 검사만 수정, 제품 코드 영향 없음). 재실행 `application::preview_tests` 2 통과 |
| `cargo test --quiet ... --lib -- preview_web_cache preview_web_media preview_web_view --test-threads=2` | 7 통과 |
| `cargo test --quiet ... --lib -- preview kbd와_samp application::markdown_tests 실제_app_보조_창_파일_탭_메뉴의_다시열기는 application::tab_menu_tests --test-threads=2` | 45 통과 |
| `cargo test --quiet ... --test preview --test preview-animation --test preview-events --test preview-formats --test preview-macos --test preview-web-media` | 1+3+1+3+2+1 = 11 통과 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test editor-markup-preview` | 첫 실행 새 검사가 `TexturesDelta` 미정리로 panic(검사 코드 결함) → `textures_delta.clear()` 추가 뒤 24 통과 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 첫 실행 차이(새 검사·새 메서드 줄바꿈) → Edit로 맞춘 뒤 통과 |
| `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 통과 |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 성공. 경고는 기존 것만(vendor `wry-preview` deprecated·unsafe, `editor-code-actions.rs` `is_applying`, `zen.rs` 543 `mut`, `fixture_with_emmet_peek`) |
| `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection` | 성공. 기존 경고 `tests/workbench.rs` 1816 `mut`만 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib --test editor-documentation` | 167 + 13 통과(비미리보기 code 상자 경로 유지) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 성공. remote-web 기존 dead code 경고만. 동결 소스·Cargo 파일 무변경 |

fmt 맞춤 뒤에는 의미 변경이 없어(줄바꿈만) 앞선 성공 검사를 재사용했습니다. 필터 없는 앱 전체 테스트는 병합 때 메인이 실행합니다.

실패를 먼저 보이는 순서: preview-6·kbd/samp는 검사를 먼저 쓰지 않고 구현과 함께 썼습니다. 리뷰 반영 때 제품 코드를 변경 전 동작으로 잠시 되돌려 새 검사가 실패함을 확인한 뒤 원래대로 복구했습니다(아래 "수정" 절의 검증 표).

## 남은 범위

- preview-5: HTML·PDF 외부 수정의 `[A]`(WebView·PDFKit 렌더가 필요한 형식이라 앱 lib kittest에서 화면 근거를 만들지 못함. 무효화 경로는 다른 형식과 같은 `reconcile_previews`), 앱 안 rename 뒤 새 경로 표시·override 유지 `[A]`(탐색기 inline rename 흐름), 앱 안 삭제는 휴지통 경로라 허락 항목(사양 4.2-4).
- preview-14·15: 실제 WKWebView에서 restyle 스크립트 적용·재생 유지·seek·컨트롤·키보드/AX(사양 4.2-2 GUI 허락). theme 변경 경로의 `[A]`도 WebView가 있어야 해서 만들지 못했습니다. "열린 view에 새 색 적용"의 자동 근거는 script 문자열 `[C]`와 cache `[C]`뿐이고, `tick_preview_media` → `Views::evaluate` → WKWebView 경로는 자동으로 실행된 적이 없습니다.
- preview-14·15 알려진 창(리뷰 지적): `Views::evaluate`는 wry `evaluate_script`의 `is_ok()`를 "적용됨"으로 봅니다. 이 호출은 스크립트를 보내고 바로 반환해 JS 실행 결과를 알 수 없습니다. 자식 WebView가 만들어진 직후 문서가 commit되기 전(수십 ms)에 theme가 바뀌면 `document.querySelector('style')`이 null이라 스크립트가 실패해도 항목이 restyled로 남아, 그 view가 닫힐 때까지 이전 색이 보일 수 있습니다. 실제 WKWebView 확인이 필요한 GUI 허락 항목이라 이번에 고치지 않았습니다(완료 콜백 `evaluate_script_with_callback`으로 확인하는 방식은 실기 검증과 함께 정할 일).
- preview-6 이미지 해석 실패 범위(리뷰 지적, 결정 필요): `Failure::Decode`에는 깨진 이미지뿐 아니라 자원 한도 초과도 들어갑니다. `decode`의 파일 크기 예산(`READ_ONLY_FILE_BYTES`), `rgba_bytes`의 변 한도·64MiB 픽셀 예산, `image` crate `Limits`, native 미지원 raster 형식, 수락 단계의 애니메이션 예산·`native preview texture cache is full`이 모두 `invalid(..)`(`AppError::InvalidArgument`)라서 구분할 타입이 없습니다. 예: 6000x4000 JPEG(RGBA 96MB)는 원본(브라우저)에서 정상 표시, 변경 전 native는 "지원 안 함 + 외부 열기", 지금은 파일 이름만 보입니다. 결정 8 A는 깨진 이미지를 전제한 결정이라 자원 한도에는 그대로 맞지 않습니다. 고치려면 여러 decoder(`preview.rs`·`preview_animation.rs`·`preview_svg.rs`·`preview_macos.rs`)에 한도 오류 분류를 새로 두어야 해 작은 결정이 아니므로 구현하지 않고 메인 확인으로 넘깁니다.
- editor-72(결정 7 A 목록 중 남은 것): `br` 줄 배치 검증, 혼합 inline code 줄 배치, 중첩 code 상자, 문단 안 이미지 경계, raw `input`(checkbox 외) 기본 모양(비활성), 첫 블록(table) 위 여백, `details`/`summary` 접힘 토글(새 `Block` 변형 → `ED` 공개 타입 변경과 동결 미러 확인 필요), `a name`/`id` 앵커 이동, 표·목록 AccessKit 역할, `kbd` 전용 `[A]`. 비대상(결정 7 A): 임의 CSS(`style` 속성)·상대 굵기·counter·`col`/`colgroup` 스타일·SVG·MathML·audio/video.
- 공용 파일 변경: `APP/application.rs`에 테스트 모듈 등록 3줄(`preview_tests`)을 `markdown_tests` 등록 바로 뒤에 추가했습니다. 사양은 `domains/preview.rs` 등록을 적었습니다. `domains`는 `application`의 자식 모듈이라 `project_rail_tests`의 `pub(super)` fixture에도 접근할 수 있으므로 가시성은 이유가 아닙니다(리뷰 지적으로 정정). 실제 이유는 테스트 파일이 다른 `application-*-tests.rs`와 같이 `use super::*;`(`NativeApplication`·`presentation`·`egui`·`Tab` 등)와 `use super::project_rail_tests::{Fixture, step, pointer, node_rect, ..}`로 부모가 `application`이라고 가정하기 때문입니다. `domains/preview.rs`에 등록하면 이 import를 모두 경로로 바꿔야 해서 기존 앱 테스트 모듈 등록 목록에 두었습니다. editor lane도 근처에 `mod`를 추가하면 병합 때 줄 충돌이 날 수 있으나 독립된 3줄이라 두 블록을 모두 남기면 됩니다.
- `preview_web_view.rs`: 구조(`Views` 필드·`sync`)는 바꾸지 않고 새 `impl Views` 블록(`open_tabs`·`evaluate`)을 `#[cfg(test)] mod tests` 바로 앞(코드 영역의 끝)에 추가했습니다. shell lane의 viewport 분리와 병합할 때 `children` 키가 바뀌면 이 두 메서드도 같은 키로 맞춰야 합니다.

## 수정 (preview lane 리뷰 반영)

리뷰의 차단 항목은 없었습니다. 과대 판정 4건과 비차단 10건을 실제 코드로 확인했습니다.

### 과대 판정

1. preview-6 `[A]` 건수: 리뷰 시점에는 검사 함수가 1개(해석 실패 화면·읽기 실패 화면·외부 열기 실패 toast)였고, HTML·오디오·비디오 준비 중 빈 배경에는 자동 검사가 없었습니다. 근거 부족으로 받아 `실제_app_html과_오디오_미리보기는_준비되기_전에_로딩_표시_없이_빈_배경을_둔다`를 추가했습니다. 대응표 행 문구를 "검사 함수 2개"와 각 함수가 보는 것으로 정정했고, 비디오는 오디오와 같은 분기(`kind != Image`의 source 없음·실패 없음)라 별도 검사가 없다고 적었습니다. complete는 유지합니다.
2. 대응표 상단 합계 표와 9줄 서문 문장: 작업 지시("상단 합계 표는 건드리지 않습니다")에 따라 이 lane에서 고치지 않았습니다. 여러 lane이 같은 줄을 바꾸는 병합 충돌 지점이라 메인이 병합 때 합산합니다. 이 묶음만 반영한 기대값은 다음과 같습니다(리뷰 계산과 같음).
    - preview 행: complete 34, partial 3, unwired 0, missing 0, not-applicable 2, frozen 1(40행). 변경은 preview-3 missing → complete, preview-6 partial → complete, preview-38 partial → not-applicable.
    - 합계 행: complete 316, partial 89, unwired 110, missing 72, not-applicable 11, frozen 1(599행).
    - 9줄 서문: 기능 분모 587행(599 − not-applicable 11 − frozen 1), 완료 316행·미완료 271행(부분 89·미연결 110·미구현 72). 다른 lane 결과와의 합산은 메인 몫입니다.
3. not-applicable 범례: 대응표 5줄 범례 문장을 "웹 기술/내부 지표 전용이거나 사용자 결정으로 범위 밖인 항목(예: preview-38, 결정 10 A)"으로 고쳤습니다. 공용 문장이라 병합 때 다른 lane의 같은 줄 변경이 있으면 함께 합쳐야 합니다.
4. preview-14·15 partial: 문서상 과장은 아니라는 리뷰 판단에 동의합니다. "남은 범위"에 자동 근거의 한계(`tick_preview_media` → `Views::evaluate` → WKWebView 경로 미실행)를 명시했습니다.

### 비차단

1. 이미지 `Decode` 범위가 자원 한도 초과까지 덮음: 실제 코드로 확인했습니다(모든 한도가 `invalid(..)`). 결정 8 A의 전제 밖이고 고치려면 여러 decoder에 오류 분류를 새로 두어야 해 구현하지 않고 메인 결정으로 넘겼습니다("남은 범위"의 preview-6 항목).
2. `restyled`를 경로 단위로 기록하는 문제: 리뷰가 본 판정 방식(열린 view가 있는 경로면 유지)은 이미 바뀌었습니다. 지금 `release_restyled(placed)`는 이번 프레임 placement마다 (경로, 그 탭에 열린 view가 있는지)를 받아, 같은 경로에 열린 view가 없는 placement가 하나라도 있으면 그 문서를 무효화합니다. `Views::sync`는 placement가 없는 탭의 child를 지우므로(`children.retain`) 숨은 탭에는 열린 view가 없습니다. 리뷰 시나리오(pane B의 `song.mp3` 활성화)에서는 그 프레임에 (song, false)가 들어와 무효화되고 새 색 문서를 다시 읽으므로 색이 어긋나지 않습니다(cache `[C]`의 `release_restyled([(shown, true), (shown, false)])` 단언). 원본과의 차이: 이 드문 경우 pane A의 view도 문서 교체로 다시 만들어져 재생 위치가 처음으로 돌아갑니다. 원본은 theme 변경이 CSS 변수만 바꾸고(사양 2.21) B 활성화가 A의 요소를 건드리지 않으므로 A의 재생이 유지됩니다. 문서가 경로 단위로 발행되는 cache 구조를 탭 단위로 바꿔야 하는 일이라 남깁니다. 변경 전 native(theme 변경 즉시 모든 media 다시 읽기)보다 나빠지지는 않습니다.
3. `Views::evaluate`의 `is_ok()` 판정 창: "남은 범위"에 적었습니다(GUI 허락 항목).
4. `web_preview_paths()` 추가 호출: 지금 `show_preview_web_views`는 맵을 한 번 만들어 `release_restyled`·`live`·renderer 실패 루프가 함께 씁니다. 변경 전에는 `live`용 1회와 sync 실패 때 placement마다 1회였으므로 정상 프레임의 `preview_tab_paths` 호출 수는 변경 전과 같습니다. `tick_preview_media`의 2회는 appearance가 바뀐 프레임에만 돕니다.
5. `PreviewCache::invalidate_media`: 지웠고 `tests/preview-web-media.rs`는 `restyle_media(&HashSet::new())`를 씁니다.
6. 내부 구현 `///`: `release_restyled`·`restyle_media`·`restyle_script`의 doc을 지웠습니다. 남은 새 `///`는 `preview_web_view.rs`의 공개 메서드 `open_tabs`·`evaluate` 계약 두 줄입니다. 원본 CSS 변수 방식의 배경은 이 문서 preview-14 절에 있습니다.
7. `application.rs` 등록 이유: "남은 범위"의 공용 파일 항목을 정정했습니다(가시성이 아니라 `use super::*` 이름공간 가정).
8. `preview_web_view.rs` 키: 기록 유지, 코드 변경 없음.
9. editor-72: partial 유지(결정 7 A 목록 10개 중 `kbd`/`samp` 1건). 파일 이름 추출 3곳(`can_reopen_with`, `show_image` 인자, 이미지 `Decode` 분기)은 UI crate의 기존 공개 함수 `command_palette_file_match::file_name`(마지막 `/` 뒤 조각, 같은 결과)으로 모았습니다.
10. 구현 전 실패 확인: 이번에 했습니다(아래 표). 사양의 `[U]`(이미지 읽기 실패 화면 아이콘·접근성 이름)는 `[A]`로 대신합니다. 읽기 실패 화면은 PDF·HWP 등이 이미 쓰는 `preview_status::show`를 그대로 부르므로 아이콘 모양에 새 `[U]`를 만들지 않았고, 외부 열기 버튼의 접근성 이름은 `[A]`가 accesskit label(`preview.openExternally`)로 찾아 누르는 것으로 확인합니다. `tests/preview-web-media.rs` 확장 대신 cache `[C]`를 둔 이유는 restyle 판정이 `Cache`에만 있고 bridge·HTTP 경로(`set_media_appearance`)는 바뀌지 않았기 때문입니다.

### 수정 검증

모두 lane worktree, `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/preview`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- application::preview_tests preview_web_cache preview_web_media preview_web_view --test-threads=2` | 10 통과(리뷰 반영 코드의 첫 실행, 새 준비 중 검사 포함) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test preview-web-media --test preview-animation` | 1 + 3 통과 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 통과 |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 성공. 경고는 기존 것만(vendor `wry-preview`, `is_applying`, `zen.rs` 543 `mut`, `fixture_with_emmet_peek`) |
| 구현 전 실패(UI): `UI/editor-markup.rs` 상자 조건을 변경 전 `span.style.code && code_frame`로 잠시 되돌리고 `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test editor-markup-preview -- 미리보기_kbd와_samp는` | 1 실패(`tests/editor-markup-preview.rs:959` `!has_code_background(&key)`) → Edit로 복구 |
| 구현 전 실패(앱): `domains/preview.rs` 이미지 `Decode` 분기를 변경 전 본문(`preview.notSupported`·제목·오류 문구·기본 버튼)으로, web 준비 중 분기를 `preview_status::show(.., error(path), ..)`(실패 없음이면 spinner·`common.loading`)로 잠시 되돌리고 `cargo test --quiet ... --lib -- application::preview_tests --test-threads=2` | 1 통과·2 실패(`application-preview-tests.rs:302` `!shows_text(&output, &loading)`, `:225` `!shows_text(&output, &not_supported)`). 외부 수정 검사는 통과 → Edit로 복구 |
| 복구 확인 `git diff --stat`(두 파일) | 되돌리기 전과 같은 줄 수(`domains/preview.rs` 161, `editor-markup.rs` 5) |
| 복구 뒤 `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test editor-markup-preview` | 24 통과 |
| 복구 뒤 앱 lib 필터(첫 줄과 같은 명령) | 10 통과 |
| 복구 뒤 `cargo fmt -- --check`(앱·UI) | 둘 다 통과 |

UI crate 소스는 복구 뒤 앞선 검증 때와 내용이 같아 UI `cargo check --all-targets`와 remote-web Wasm check는 앞선 성공을 재사용했습니다.
