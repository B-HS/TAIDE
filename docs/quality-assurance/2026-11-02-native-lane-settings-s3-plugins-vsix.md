# native lane settings 묶음 S3: Plugins·VSIX·프롬프트·소유권 (s3-plugins-vsix)

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/settings`(브랜치 `lane/settings`) · 사양 `docs/research/2026-10-28-lane-settings-spec.md` 2.6, 2D(39), 2E(40~44), 2I(5), 3절 S3
> 결정: 사양 4.1 전부 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`). 앞 묶음 S2의 미커밋 변경 위에서 이어 작업했습니다.

## 1. 대상 ID와 판정

| ID | 내용 | 판정 |
| --- | --- | --- |
| settings-39 | AI 프롬프트 파일 열기 3종(app file 탭) | complete |
| settings-40 | Plugins 목록(이름, 버전, 활성, 오류 코드 7종) | complete |
| settings-41 | .zip 설치(파일 선택, 성공/실패 toast) | complete(실제 파일 선택 창은 4.2 허락 항목) |
| settings-42 | VSIX 가져오기 대화상자(후보·중복 확인·대비 실패·문법 3상태) | partial(7절: 기능 경로·이식 사례는 닫힘, `service.rs` 색 도우미 중복 제거 남음) |
| settings-43 | 새로고침, 플러그인 폴더 열기 | complete(실제 Finder 열기는 하지 않음, 5절) |
| settings-44 | 제거 확인 대화상자 | complete |
| settings-5 | 보조 창 포함 프로젝트별 탭 소유권 | partial(7절: 사양 검사 순서 전부 통과, 사양 3절 5번의 shell-5 병합 조건만 남음) |

## 2. 원본 동작 근거

- 사양 2D(39), 2E(40~44), 2I(5)의 원본 인용을 따릅니다. 이번에 다시 확인한 원본: `src/widgets/plugin-manager/{plugin-manager,plugin-install-button,plugin-uninstall-dialog,vsix-import-dialog}.tsx`, `src/features/plugin/{plugin-list-body,vsix-import-grammars-section}.tsx`, `src/entities/{plugin/plugin.query,vsix/vsix.query}.ts`, `src/shared/lib/vsix-theme-import.ts`, `src/shared/lib/theme-draft.ts`(`slugifyThemeId`, `generateUniqueThemeId`), `src/shared/lib/color.ts`, `src/shared/lib/theme-convert/{convert,types,jsonc,merge,resolve-colors,resolve-syntax,resolve-terminal,validate-completeness,ansi-palette,mapping-tables,ui-token-vocabulary,contrast,component-contrast-pairs,state-distinctness,state-distinctness-pairs}.ts`, `src/shared/ui/{alert-dialog,dialog,icon-button,button}.tsx`, `src/widgets/settings-view/settings-ai-section.tsx:28-32,63,119-130`, `settings-view.tsx:61-92`(Plugins는 AI 다음).
- 폴더 열기 백엔드는 `crates/taide-runtime/src/system_actions.rs:37` `system_open_app_data_path`가 `reveal_item_in_dir`(폴더를 Finder에 표시)을 부릅니다.

## 3. 구현

### 3.1 테마 변환 이식(42, `crates/taide-theme/src/convert/`)

- `color.rs`(← `color.ts` 중 변환이 쓰는 부분: hex 판정·파싱·합성·CIE76·WCAG 대비, JS `length`·`parseInt` 의미를 따르는 도우미), `jsonc.rs`(주석·끝 쉼표 제거, `#rgb`/`#rgba` 확장), `tables.rs`(색 이름공간 133개, syntax 후보, ANSI 기본 팔레트, 색 매핑 표, 대비 쌍 7, 컴포넌트 대비 쌍 36, 상태 구별 쌍 50), `mapping.rs`(후보 사슬·파생 규칙·family 대체·safe default, 목록 배경·불투명 전경·본문 전경 거리 판정), `state.rs`(상태 구별 수리 4패스·그림자 알파·잔여 위반), `contrast.rs`(blocking/advisory 쌍 수리, 컴포넌트 배경·전경 수리, 출력 오류), `mod.rs`(`convert_vscode_theme(raw_chain, theme_type) -> ConversionResult`).
- 후보 생성: `crates/taide-runtime/src/vsix_actions.rs` `vsix_theme_candidates(result, existing_ids)`(publisher-name slug, 여러 테마면 라벨 slug, 같은 VSIX 안 중복 `-2…`, uiTheme `vs`/`hc-light` → light, include chain 역순 병합, 실패 사유 parse/incomplete/contrast, 경고 수 = safe default + ANSI 대체 + 수리 + 잔여 구별 위반, Theme 메타 author/source). 후보 타입 `taide-model::vsix::{VsixThemeCandidate, VsixThemeImportFailure}`.

### 3.2 화면(UI crate)

- `Section::Plugins`(제목 `settings.plugins`, `Section::HOSTED`에 추가, AI 다음). 미러(`Views::show`)는 그대로 8 section입니다.
- `settings-sections-data.rs`: `Query::Plugins`(60초), 행동 `PluginReload`·`PluginUninstall`·`PluginInstall`·`VsixExtract`·`VsixSaveThemes`·`VsixImportGrammars`, 파일 선택 요청 `PickPurpose::{PluginArchive, Vsix(ticket)}`(`request_pick`/`take_pick`/`picked`, 취소는 아무 요청도 없음), 진행 상태(새로고침·설치·추출), VSIX 대화상자 ticket별 사건(`VsixEvent::{Extracted, Saved, GrammarsImported}`, 상한 16), 알림 `Notice::SuccessWith`(문구 인자).
- `settings-sections-view.rs`:
    - Plugins: 조회 중 muted `settings.loading`, 빈 목록(또는 조회 실패, 원본 `data = []`)은 점선 상자 두 줄, 행(이름 medium truncate·버전 monospace muted·활성 agent-running/비활성 muted·Trash 아이콘 버튼 tooltip), 오류 코드 7종 error 줄. 버튼 줄(wrap, gap 8): zip 설치(설치 중 비활성), VSIX 가져오기(추출 중 비활성), 새로고침(진행 중 비활성·RefreshCw 1회전/초), 폴더 열기(`Output::folders`에 `Plugins`).
    - 제거 AlertDialog sm: 제목 `settings.pluginUninstallConfirmTitle{name}`, 설명, `common.cancel` / 파괴 버튼 `settings.pluginUninstallButton`, 확인 즉시 닫힘 + `PluginUninstall`.
    - VSIX Dialog: 제목 `settings.pluginImportVsixButton`, 설명 displayName, 테마 영역(후보 없음 문구 / 최대 256px 스크롤 목록: 체크박스(실패면 비활성·행 50%), 라벨, 유형, 경고 수(TriangleAlert 12px), 중복 문구, 실패 사유) + 기본 xs `common.save`(선택 0개·저장 중 비활성), 중복이 있으면 확인 AlertDialog sm(설명 = 중복 문구 `" / "` 연결), 저장 시 중복은 `unique_theme_id`와 `themeEditor.duplicateNameTemplate`로 복사본. 문법 영역(`modal.separator` 구분선): 성공 CircleCheck + success 문구, 실패는 `describe_error` 문구 그대로(toast 없음), 아니면 outline xs 버튼. 하단 `common.close`, 오른쪽 위 X(`common.close` 접근 이름).
    - AI section 끝 구분선 아래 프롬프트 3행: 라벨 + 연필 아이콘 버튼(tooltip `prompts.editEntry`) → `Output::open_app_file = (AppFileTarget::Prompt { id }, 지역화 제목)`.
- 새 `settings-dialogs.rs`(`settings_view::dialogs`): Radix AlertDialog(바깥 클릭 무시, Escape 취소, 첫 포커스 취소 버튼, Tab/Shift+Tab 트랩, sm은 가운데 정렬·버튼 2열)와 Dialog(바깥 클릭·Escape 닫힘, IME 조합 중 Escape 무시, 모서리 X). 닫히면 연 버튼으로 포커스를 돌려 줍니다(`dialogs::settle`).
- `theme-draft.rs`: `unique_id`를 `pub(crate) unique_theme_id(name, used: &HashSet<String>)`로 일반화하고 기존 호출은 위임합니다.
- 아이콘 `Icon::Import`, `Icon::RefreshCw`(새 SVG `resources/icons/import.svg`, `refresh-cw.svg`, lucide path, 같은 폴더 LICENSE).

### 3.3 앱

- `domains/settings.rs`: `execute_sections`에 Plugins 조회·새로고침·제거·설치, VSIX 추출(`vsix_extract_themes` + 블로킹 후보 생성, 그 시점 테마 id 목록 포함)·테마 저장(순서대로 `theme_save`, 첫 오류 중단)·문법 가져오기(`vsix_import_plugin` + `commit_staged_vsix_plugin`). 테마 저장 응답 뒤 `Views::invalidate_catalog`. `pick_settings_files(frame)`가 대기 중인 선택을 `Ports::pick_file`로 처리합니다(필터 `Plugin Archive`/`zip`, `VSIX`/`vsix`, 제목 지역화). `show_settings_tab`이 `open_app_file`을 `app_file_views::open_command`로 보냅니다.
- `settings-ports.rs`: `pick_file: file_dialogs::OpenPicker`(기본 `file_dialogs::open`, rfd). `file-dialogs.rs`에 `OpenRequest`·`OpenPicker`·`open` 추가.
- `app-file-views.rs`: `open_command(project, layout, scope, target, title)`(보조 창이면 그 창 focused pane, 다른 프로젝트면 없음). `settings_command`는 위임합니다.
- 공용 `application.rs`: `show_window`에서 `load_settings_requests` 앞에 `self.pick_settings_files(frame);` 1줄.

## 4. 사양·원본과 다른 점(근거)

1. 대화상자 등장·퇴장 애니메이션(Radix 200ms fade/zoom)은 넣지 않고 `Transition::SETTLED`로 그립니다. 버튼 hover 색 전이·그림자도 단순화했습니다. 동작(트랩·Escape·바깥 클릭·포커스 복귀)은 원본과 같습니다.
2. 포커스 복귀는 닫힌 대화상자가 egui의 top modal에서 빠진 다음 프레임에 요청합니다(같은 프레임 요청은 modal 규칙 때문에 버려짐).
3. 새로고침 아이콘 회전은 S2 스피너 선례처럼 감소 모션과 무관하게 돕니다(원본 CSS에 `prefers-reduced-motion` 규칙 없음). 사양 2E-43의 "reduced motion이면 정지"와 다릅니다.
4. `Data`는 모든 Settings owner가 공유하므로 VSIX 추출 결과를 그 버튼을 누른 owner에게만 전달하려고 ticket을 붙였습니다.
5. 복사본 id 계산의 사용 중 집합은 추출 시점의 테마 id 목록(+ 이번 저장분)입니다. 원본은 대화상자가 그려질 때의 목록이라, 대화상자가 열린 사이 다른 곳에서 테마가 바뀐 경우만 다를 수 있습니다.
6. JSONC 끝 쉼표 제거는 문자열을 인식합니다(원본 정규식이 문자열 안 `,}`도 지우는 결함은 재현하지 않음, 사양 결정). tokenColors의 비문자열 `foreground`·`scope` 등은 없는 값으로 봅니다(원본은 예외로 대화상자가 깨짐).
7. 상태 구별 수리의 palette 우선 분기는 넣지 않았습니다. 변환 테마는 palette가 비어 그 분기가 실행되지 않습니다(`convert.ts` `CONVERTED_THEME_PALETTE` 주석).
8. `contrast.ts`의 카탈로그 검증 함수(`validateComponentContrast`, `validateFixedForegroundContrast`, `validateTerminalAnsiContrast`, 예외 표)는 변환이 쓰지 않아 이식하지 않았습니다. 사양의 "`service.rs` 테스트 모듈 색·대비 도우미를 convert로 옮기기"도 하지 않았습니다(6절).
9. 폴더 열기 App 증거: 실제 Finder를 열지 않으려고 기록 platform으로 `system_open_app_data_path`의 경로만 확인했습니다. UI 클릭 → `Output::folders` → `OpenSettingsFolder`는 [U] 검사와 기존 연결 코드로 확인합니다.
10. (7절에서 대체) 처음 settings-5 App 검사는 탭 메뉴 이동 대신 보조 창 layout을 복원했고, "다시 mount"는 보조 owner View가 사라진 것만 확인했습니다.
11. 검사 재사용을 위해 S2 App 검사 파일의 `Scene`·도우미를 `pub(super)`로 열고 `Scene::context()`를 추가했습니다(동작 변화 없음).

## 5. 검증

실행 명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`입니다.

| 범위 | 명령 | 결과 |
| --- | --- | --- |
| [C] 변환·taide-model | `cargo test --manifest-path Cargo.toml -p taide-theme -p taide-model` | 28 + 59 passed(변환 10개 포함) |
| [C] 후보 생성·runtime | `cargo test --manifest-path Cargo.toml -p taide-runtime --lib -- --test-threads=4` | 182 passed(후보 4개 포함) |
| [U] UI lib 전체 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` | 193 passed |
| UI 통합 | `... --features native-host,inspection --test settings-code-controls --test snippet-editor` | 1 + 18 passed(포맷 전 실행, 이후 변경은 포맷·미사용 인자 이름뿐) |
| [A] 실제 App | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_plugins_tests settings_sections_tests app_file_views settings_ theme_ snippet_host_tests remote_preferences presentation --test-threads=2` | 42 passed(S3 3개 포함) |
| 컴파일 | `cargo check --all-targets`(UI `--features native-host,inspection`, 앱 기본) | 통과, 경고는 기존 것뿐 |
| src-tauri | `cargo check --manifest-path Cargo.toml -p taide` | 통과 |
| 동결 미러 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과, 경고는 기존 `browser-editor.rs`·`browser-application.rs` |
| 형식 | `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`(UI, 앱, taide-theme, taide-runtime) | 차이 없음. taide-model은 기존 `lib.rs`(`ids`/`identifier` 순서) 차이만 있고 이번 `vsix.rs`는 차이 없음(범위 밖이라 그대로 둠) |
| lockfile | `git status --short` | `Cargo.toml`·`Cargo.lock` 변경 없음 |

- [C] `convert/tests.rs`: JSONC(주석·끝 쉼표·문자열 보존), 전경 후보 판정(축약 알파, ΔE 0/5.4/77.7), 어휘·쌍 표 drift(Rust 필수 키, TS `state-distinctness-pairs.ts`·`component-contrast-pairs.ts` 라벨 순서, `ui-token-vocabulary.ts` syntax 키), 최소 dark/light 완전 변환·safe default·ANSI 16 대체, include 체인·scope 우선·중복 규칙, tokenColors 보존·빈 룰 제외, matchHighlight 반투명·본문 동일색 배제, 대비 수리 2패스 고지·textLink 채택·everforest/rose-pine 프록시, 상태 구별 수리·terminal 미러.
- [C] `vsix_actions` tests: slug id·충돌, NLS 표시명 무관 id, 여러 테마 라벨·`-2` 중복, light 유형, parse 실패, tokenColors, include 체인, 잔여 구별 위반이 경고 수에 더해짐.
- [U] `settings-sections-data-tests.rs`: Plugins 응답(목록 교체·무효화·toast 키·진행 해제), VSIX ticket 전달·상한·저장 성공 문구 인자·제출 실패 응답. `settings-sections-view-tests.rs`: 미러 8 / 호스트 순서(Keymap < LSP < AI < Plugins), 목록 3상태·오류 코드 7종 색, 버튼(선택 요청·취소 무요청·설치 중 비활성·새로고침 비활성과 목록 교체·폴더 출력), 제거 대화상자(첫 포커스 취소·Tab 순환·바깥 클릭 무시·Escape·Trash로 포커스 복귀·확인 즉시 닫힘과 요청), 프롬프트 3행·tooltip 3·출력, VSIX(빈 목록·경고 수 색·중복·실패 사유·처음 선택), 저장(확인 → 복사본 id/이름·extension), 문법 3상태, X·바깥 클릭·Escape 닫힘.
- [A] `application-settings-plugins-tests.rs`(실제 `NativeApplication`, 경로 main.rs → `NativeApplication` → `show_window` → `show_settings_tab` → `Views::show_with_host` → `sections_view::plugins` → `pick_settings_files` → `domains::settings::dispatch`):
    1. 임시 plugins(정상 1, id 불일치 1) → 행·`settings.pluginError.idMismatch`; 가짜 picker zip → 설치 성공 toast·행·디렉터리, 요청 필터/확장자/제목; 취소 → 요청 없음; 깨진 zip → 오류 toast; 새 디렉터리 + 새로고침 → 행; 제거 확인 → 디렉터리 삭제·성공 toast; 디스크에서 지운 플러그인 제거 → 오류 toast; 기록 platform으로 폴더 경로 = 데이터/plugins.
    2. 실제 `.vsix`(테마 2 + 문법 1)를 가짜 picker로: 저장 → 성공 문구(`count` 2, extension) · 테마 2개 증가; 문법 가져오기 → 플러그인 설치·성공 toast; Escape로 닫고 다시 열기 → 중복 문구 → 확인 → 복사본 2개(이름 `"{name} copy"`); 문법 재가져오기 → `error.plugin.alreadyInstalled` 문구가 대화상자 안에 보이고 오류 toast는 없음.
    3. 프로젝트 2개(메인 활성 B), A의 Settings 탭을 가진 보조 창 viewport에서 프롬프트 연필 → A 보조 pane에 `AppFile { Prompt }`(지역화 제목), Settings 탭으로 돌아가 settings.json → 같은 pane, B 탭 수 불변, TOC Plugins 활성 → 창 닫기 → 탭 메인 복귀·보조 owner View 제거.

## 6. 남은 범위

- 실제 파일 선택 창·실제 Finder 표시는 4.2 허락 항목이라 자동 검사에서 부르지 않았습니다.
- 사양 3.1 "service.rs 테스트 모듈 색·대비 도우미를 `convert::color`·`convert::contrast`로 옮겨 중복 제거"는 하지 않았습니다. 두 쪽 표는 각각 TS 정본과 drift 검사로 묶여 있습니다.
- `contrast.ts`의 카탈로그 전용 검증(컴포넌트 대비 위반 문구, 고정 전경, 터미널 ANSI, 예외 표)과 `theme-convert` 카탈로그 테스트(`bundled-theme-*.test.ts`)는 변환 경로가 쓰지 않아 이식하지 않았습니다.
- 대화상자 등장·퇴장 애니메이션(4절 1).
- 5종 대화상자 공통 트랩 증거 묶음(settings-83)은 S5 몫입니다. 이번에는 제거 AlertDialog의 트랩·복귀와 VSIX Dialog의 닫힘 3종을 검사했습니다.

## 7. 수정(리뷰 반영, 2026-11-03)

### 7.1 settings-5 — 과대 판정 반영

- 리뷰 지적: 복원한 보조 창 layout을 썼고, 닫힘 뒤에는 보조 owner View가 사라진 것만 단언해 메인 새 mount와 TOC 활성 Appearance를 확인하지 않음.
- 확인: 지적이 맞습니다. 탭 메뉴 이동(`tab.moveToNewWindow`)은 이 worktree에 이미 있어(`application-auxiliary-window-tests.rs` 이동 검사) 사양 2I 순서 그대로 검사할 수 있었습니다.
- 수정: `application-settings-plugins-tests.rs` `실제_보조_창_settings는_자기_프로젝트_창_pane에_settings_json과_프롬프트를_열고_닫히면_메인에서_새로_mount된다`를 다시 썼습니다.
    1. 프로젝트 A 활성 메인 tab bar의 Settings 탭을 오른쪽 클릭 → `tab.moveToNewWindow` → A의 보조 창(slot 1)과 그 pane을 layout에서 읽음(복원 layout 사용 안 함).
    2. `ShellMutation::ActivateProject(B)`로 메인 활성을 B로 바꿈.
    3. 보조 viewport에서 프롬프트 연필 → A 보조 pane에 `AppFile { Prompt }`(지역화 제목), Settings 탭 → settings.json → 같은 pane, B 탭 수 불변, TOC Plugins 활성.
    4. 보조 창 닫기 → 탭이 A 메인 pane으로 복귀 → A 활성·Settings 탭 활성화 → 메인 owner(`A, 메인 pane, 탭`)가 새로 생기고 `active == Section::Appearance`, 보조 owner는 없음.
- 판정: 사양 2I [A] 순서는 모두 통과했습니다. 다만 사양 3절 5번과 2I 완료 판정이 "shell-5 병합 뒤"를 조건으로 두었고 이 worktree에는 shell lane(wave 1) 병합이 없으므로 partial로 낮추고 메인에 보고합니다(병합본에서 같은 검사 재실행이 남은 일).

### 7.2 settings-42 — 과대 판정 반영

- 리뷰 지적: TS 사례(약 80개)를 같은 기대값으로 옮기지 않았고(Rust 10개), 사양 2E-1의 `service.rs` 테스트 도우미 중복 제거도 하지 않음.
- 확인: 처음 [C] 10개는 jsonc 5·mapping-tables 4 사례를 같은 기대값으로 담았고, convert.test.ts 25 사례와 vsix-theme-import 8 사례는 묶음 검사(5절 [C] 목록)로 덮었지만 state-distinctness·contrast 사례는 거의 옮기지 않았습니다. 지적이 맞습니다.
- 수정: `crates/taide-theme/src/convert/tests.rs`에 TS 기대값 그대로의 검사 4개(37 사례)를 더했습니다.
    - `상태_구별성_검증은_ts_사례와_같은_쌍을_잡는다`: `state-distinctness.test.ts` validate 11 사례(기준 팔레트, 바탕 동일, 반투명 합성, dracula, transparent 컨테이너 건너뜀, 옅은 축, ayu 형제, everforest 경계, 형제 surface 선언, shadow 하한, 평평한 탭).
    - `상태_구별성_수리는_ts_사례와_같은_값과_문구를_낸다`: repair 7 사례(최소 변화 `#232333` 문구, 결정·멱등, 대안 토큰, 반복, 형제, shadow `#00000026`, 본문 전경 없음). palette 우선 사례 1개는 4절 7(변환 테마는 palette가 비어 실행되지 않는 분기)로 제외.
    - `출력_대비_검증과_쌍_수리는_ts_사례와_같다`: `contrast.test.ts`의 `validateOutputColors`·`repairContrastPairs` 11 사례(반투명/불투명 matchHighlight, editor.foreground 수리, 합성 항등, 8자리 전경, 선택 행 저대비여도 5쌍 통과·5쌍 실패는 거부, list.foreground, 두 배경 공유, rose-pine-dawn 2패스 금지, nord 보호). 같은 사례 안의 `validateSelectionRowContrast` 단언은 카탈로그 전용이라 뺐습니다.
    - `컴포넌트_대비_수리는_ts_사례와_같다`: `repairComponentContrast` 8 사례(git 데코 최소 이동, 여러 표면 1회 이동·라벨, 반대 명도 미수리, list.foreground 선택 행 보호, 예외 보호, menu.itemHover 교체 문구, 구별 안 되는 후보 거절, ayu 임계 미달). 검사에서 부르려고 `contrast.rs` `repair_component_contrast`를 `pub(super)`로 열었습니다(동작 변화 없음).
- 옮기지 않은 TS 사례: `contrast.test.ts`의 카탈로그 전용 7개(`validateComponentContrast` 2, `validateTerminalAnsiContrast` 2, `isExemptComponentContrastViolation` 1, `validateFixedForegroundContrast` 2)는 변환 경로가 쓰지 않는 함수라 4절 8 그대로입니다(번들 테마 카탈로그 검증은 `service.rs` 테스트 모듈이 맡음).
- 판정: 이식 사례 근거는 보강했지만 사양 2E-1의 `service.rs` 테스트 모듈 색·대비 도우미(`hex_to_rgb`, `relative_luminance`, `contrast_ratio`, `composite_over_background`, `delta_e76`, `foreground_contrast_ratio` 등 39곳 호출) 중복 제거는 여전히 하지 않았으므로 partial입니다. 두 구현은 `hex_to_rgb` 반환형(튜플/`Rgb`)과 `is_hex_color`의 공백 처리가 달라, 바꾸면 카탈로그 검사 전체를 다시 맞춰야 합니다.

### 7.3 검증

| 범위 | 명령(공통 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`) | 결과 |
| --- | --- | --- |
| [C] 변환 | `cargo test --quiet --manifest-path Cargo.toml -p taide-theme --lib convert::` | 14 passed(새 4개 포함) |
| 앱 컴파일 | `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 통과, 경고는 기존 것뿐 |
| [A] settings-5 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- 실제_보조_창_settings는 실제_auto_tab_토글은 실제_글꼴_picker는 --test-threads=2` | 2 passed(settings-5, settings-24). 같은 실행의 settings-38 새 검사 1차 실패는 S2 문서 7절 |
| [A] 최종 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- domains::settings:: app_file_views theme_ presentation --test-threads=2` | 42 passed(모든 수정 뒤, S5 문서 7.3) |
| [C] taide-theme 전체 | `cargo test --quiet --manifest-path Cargo.toml -p taide-theme --lib` | 63 passed(형식 정리 뒤) |
| 형식 | `cargo fmt --manifest-path crates/taide-theme/Cargo.toml -- --check`, 앱 crate 같은 명령 | 새 검사 줄바꿈을 `cargo fmt`로 맞춘 뒤 차이 없음 |
