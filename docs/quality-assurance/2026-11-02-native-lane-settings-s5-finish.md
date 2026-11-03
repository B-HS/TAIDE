# native lane settings 묶음 S5: 마감·교차 확인 (s5-finish)

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/settings`(브랜치 `lane/settings`) · 사양 `docs/research/2026-10-28-lane-settings-spec.md` 2I~2K, 2B(23), 3절 S5
> 결정: 사양 4.1 전부 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`). 실행 중 scrollback은 terminal-agent lane 구현·이 lane 소비(같은 문서). 앞 묶음 S1~S4의 미커밋 변경 위에서 이어 작업했습니다.
> 다른 lane 병합 상태: 이 worktree에는 shell lane(shell-91 알림 포트·`native_notifications`·`notify_event`, shell-93), terminal-agent lane(`set_history`/`configure_history`), explorer-search·git lane 산출물이 없습니다(`rg native_notifications|configure_history|set_history|fn notify_event` 결과 0건).

## 1. 대상 ID와 판정

| ID | 내용 | 판정 |
| --- | --- | --- |
| settings-3 | TOC 클릭 스크롤과 활성 표시 | complete |
| settings-23 | OS 알림 발송 범주 7종 | partial(shell-91·git-7·explorer-search-33 미병합, 6절) |
| settings-24 | Editor 글꼴 크기·family picker | complete |
| settings-28 | 표시·입력 옵션의 에디터 반영 | partial(리거처 한계, semantic=editor lane, diff 2종=git lane, 6절) |
| settings-30 | Terminal section | partial(실행 중 scrollback = terminal-agent-11 미병합, 6절) |
| settings-67 | 편집한 키가 실제 동작에 반영 | partial(실행 경로 없는 명령 45개, 소비자는 commands-ui·영역 lane, 6절) |
| settings-78 | 오류 표시 경로 | partial(이 lane 2곳만 반영, 나머지는 메인 통합, 6절) |
| settings-80 | Interface 설정값의 소비처 | partial(reveal·searchOnType·debounce·IDE diff는 다른 lane, minimap App 증거 없음, 6절) |
| settings-83 | Monaco 키코드 바인딩 가능 여부와 Dialog 포커스 트랩 | complete(실기 포커스·VoiceOver는 4.2 허락 항목) |

## 2. 원본 동작 근거

- settings-3: `src/widgets/settings-view/settings-view.tsx:59-92`(14 section id, `SETTINGS_TOC_ITEMS` 13개 + 조건부 Performance), `:133-140`(`handleTocSelect`: 활성 갱신, 대상 top − 컨테이너 top + scrollTop − `SETTINGS_SCROLL_OFFSET_PX`(32)로 `scrollTo({behavior: 'smooth'})`, 브라우저가 최대 스크롤로 자름), `:188-197`(TOC `sticky top-8`).
- settings-24·30: 사양 2J 인용(`settings-editor-section.tsx:64-80`, `font-picker.tsx`, `settings-terminal-section.tsx`, `terminal-view.tsx:217-257`).
- settings-28·80: 사양 2J 인용(`code-editor-settings.ts:43-65`, `pane-node-view.tsx` 환영 화면, `app-sidebar.tsx` 배지 등).
- settings-67: `src/shared/lib/keymap/keymap.ts`(override 병합) — 사양 2K.
- settings-78: 사양 2K와 `docs/quality-assurance/2026-10-06-native-batch3-toast-feedback.md` 3.3(527 app file 정합 실패 = 편집기 저장소 내부 오류, 3605 테마 미리보기 실패 = TS 대응 없음).
- settings-83: `src/shared/lib/monaco/monaco-keybinding.ts:19-95`(`MONACO_KEY_CODE`), `:103`(`resolveMonacoKeyCode`가 `key.toLowerCase()`로 조회), Radix Dialog/AlertDialog 포커스 트랩.

## 3. 구현

### 3.1 UI crate

- `settings-view.rs`(settings-3 근본 수정): TOC 클릭 스크롤을 `ui.scroll_to_rect(card − 32, Align::TOP)`에서 "카드 루프가 끝난 뒤 `ui.scroll_with_delta(clip_top + 32 − card_top)`"으로 바꿨습니다. egui `ScrollArea`는 정렬 대상 스크롤에 content ui의 `item_spacing`을 더해(`vendor/egui-input/src/containers/scroll_area.rs:1121-1132`) 카드가 32px가 아니라 약 34.7px에 멈췄고([U] 검사가 `title=55.72, anchor=53`으로 재현), 루프 중간에 낸 대상은 뒤 카드의 내부 ScrollArea가 가져갈 수 있었습니다. 델타는 egui가 최대 오프셋으로 자르므로 원본 `scrollTo`의 자르기와 같고, 애니메이션은 style의 `scroll_animation`(원본 `smooth`)입니다.
- `keymap.rs`: 끝에 검사 모듈 선언 1개(`#[cfg(test)] #[path = "keymap-settings-tests.rs"] mod settings_tests;`). 동작 코드는 바꾸지 않았습니다.
- 새 `keymap-settings-tests.rs`: 아래 5절 [C] 두 검사.
- `settings-sections-view-tests.rs`: `Scene.screen` 필드(TOC 스크롤용 900px viewport), TOC 검사, 대화상자 5종 공통 계약 검사.

### 3.2 앱 crate

- `settings-ports.rs`: `Ports::font_families: ListFontFamilies`(기본 `taide_font::service::list_families`). 검사는 고정 글꼴 3개를 넣습니다(실제 시스템 글꼴 목록 비의존).
- `domains/settings.rs`: `Command::ReadSettingsResource { request, ports }`(제출 지점 1곳이 `sections_ports`를 함께 보냄, dispatch가 `ports.font_families`를 부름). settings-78 결정 3 A: `poll_settings_revisions`의 app file 정합 실패와 `sync_theme_preview` 실패의 `self.status = Some(..)` 2곳을 `log::warn!`으로 바꿨습니다(둘 다 TS 대응 없는 native 내부 오류). 끝에 검사 모듈 1줄(`settings_consumers_tests`).
- `application-settings-sections-tests.rs`(검사 도우미): `Scene::launch_empty_pane`(Settings 탭 대신 빈 pane, 기본 layout의 탭을 비움), `Scene::resize`.
- 새 `application-settings-consumers-tests.rs`: 아래 5절 [A]/[C] 검사.
- 공용 파일(`application.rs`, `host.rs`, `lib.rs`)은 이번 묶음에서 고치지 않았습니다.

## 4. 사양·원본과 다른 점(근거)

1. settings-23은 사양대로 구현하지 않았습니다. 사양의 묶음 검사(기록 platform + `TerminalCommandFinished`/`LspInstallProgress(Done)` → 범주 스위치별 1회/0회)는 shell-91의 주입 platform과 생산자가 있어야 하는데 이 worktree에 없어 작성하지 않았습니다(사양 3절 주의 5: 대체 구현 금지). `application-settings-notifications-tests.rs`는 만들지 않았습니다.
2. settings-30 실행 중 scrollback은 결정 문서가 terminal-agent lane 구현으로 정했으므로 사양 2J의 `TERM/lib.rs`·`session.rs`·`terminal_host.rs` 추가를 하지 않았습니다(이 worktree에 `set_history`/`configure_history` 없음).
3. settings-78은 사양 1.3·2K대로 "다른 lane 파일의 지점은 병합 뒤 메인 통합"입니다. 이 lane 소유 파일(`domains/settings.rs`)의 2곳만 반영했고, 상태바 라벨과 `status` 필드 제거는 남은 지점이 0이 된 뒤에 해야 해서 하지 않았습니다.
4. settings-24 검사에서 글꼴 항목은 자기 글꼴 미리보기로 그려져 일반 글자 모양으로 잡히지 않으므로, 목록 존재·필터·검색은 접근성 노드(이름)로 확인했습니다. 시스템 기본 항목과 trigger 문구는 글자 모양으로 확인했습니다.
5. settings-3 [A]는 section 데이터가 모두 들어온 뒤(조회 중 문구 0개) 클릭합니다. 데이터가 들어오며 위쪽 카드 높이가 바뀌면 클릭 시점에 계산한 위치가 어긋나는데, 원본도 클릭 시점의 `getBoundingClientRect`로 한 번 계산하므로 같은 동작입니다.
6. settings-3 마지막 항목(Performance)은 카드가 짧아 32px 위치까지 갈 수 없으면 페이지 끝에서 멈추는 것을 확인했습니다(원본 `scrollTo` 자르기와 같음). native는 기존 코드가 카드 끝에 viewport 절반의 여백을 두어 원본보다 더 내려갈 수 있습니다(이번 범위 밖, 기존 동작 유지).
7. settings-67의 "전수"는 `catalog::rows`의 모든 행(mac 218행, 그 밖 215행)입니다. 실행 경로 판정은 사양 2K처럼 `command_dispatch::accepts`를 활성 프로젝트·포커스 슬롯·쓰기 가능 편집기 문맥으로 부른 결과입니다. 사양 2K (3)의 명령 재사용은 `domains::settings::Command::Sections { request, ports }`와 `sections::Action::{CliInstall, CliUninstall, SyncUpload, SyncDownload}`가 이미 crate 안에서 공개라 추가 변경이 없습니다.
8. settings-24의 글꼴 목록은 실제 시스템 글꼴을 읽지 않도록 포트로 주입했습니다. 기본값은 기존과 같은 `taide_font::service::list_families`입니다.

## 5. 검증

실행 명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`입니다.

| 범위 | 명령 | 결과 |
| --- | --- | --- |
| [U] 실패 확인 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection -- 목차는_호스트에서`(스크롤 수정 전) | 1 failed(`title=55.71875, anchor=53`, 카드가 32px 대신 약 34.7px) |
| [C][U] 새 UI 검사 | 같은 crate `-- settings_tests 목차는_호스트에서 설정_대화상자_5종 --nocapture` | 3 passed(drift, override 전수 + 라우팅 목록 출력, 대화상자 5종), TOC 1 failed(위 행) → 스크롤 수정 뒤 `-- 목차는_호스트에서` 1 passed |
| UI lib 전체 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` | 218 passed |
| UI 통합 | `... --features native-host,inspection --test settings-code-controls --test snippet-editor` | 1 + 18 passed |
| [A][C] 새 App 검사 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_consumers_tests --test-threads=2` | 5 passed(TOC, 글꼴 picker, 환영 화면, 터미널, accepts 목록). 중간 실패와 수정은 6절 |
| 앱 settings·keymap 회귀 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation remote_sync remote_http remote_dispatch keymap --test-threads=2` | 101 passed(S1~S5 App 검사 포함) |
| 인용한 기존 증거 | `... --lib -- 실제_app_rail은_표시_glyph와 자동_포맷은_입력과 --test-threads=2` | 2 passed |
| 앱 통합 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test settings-controls` | 2 passed |
| 컴파일 | `cargo check --all-targets`(UI `--features native-host,inspection`, 앱 기본) | 통과, 경고는 기존 것뿐(`tests/workbench.rs` unused mut, vendor wry, `is_applying`, `zen.rs`, `fixture_with_emmet_peek`) |
| 동결 미러 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과, 경고는 기존 `browser-editor.rs`·`browser-application.rs` |
| 형식 | `cargo fmt --manifest-path <UI·앱 Cargo.toml> -- --check` | 차이 없음 |
| lockfile·미러 | `git diff --stat -- Cargo.lock native/taide-native-app/Cargo.lock native/taide-native-ui/Cargo.lock native/taide-remote-web` | 변경 없음 |
| settings-78 집계 | `grep -c "status = Some("`(앱 `application.rs`·`domains/*.rs`) | `domains/settings.rs` 0, 나머지 합계 94 |

- 실제 앱 경로: `main.rs` → `NativeApplication`(`eframe::App::logic`/`ui`) → `show_window` → `domains::settings::show_settings_tab` → `Views::show_with_host`(TOC·picker) / `load_settings_requests` → `domains::settings::dispatch`(`ReadSettingsResource`, `UpdateSettings`) → 소비자(`shell.rs` 환영 화면, `terminal_host::Hub::configure_cursors`, `tick_terminal_agent_fonts`, 새 터미널 생성).
- [U] `keymap-settings-tests.rs`: (1) drift — 원본 표의 모든 키가 `editor_key` 참, 출력 가능 ASCII 95자 + `egui::Key::ALL`의 캡처 이름 + 원본 키 전체에서 `editor_key(k) == 원본.contains(lowercase(k))`. (2) override 전수 — 대체 키가 기본 상태에서 아무것도 dispatch하지 않음을 먼저 확인, 행마다 새 Keymap에 그 행만 override, 새 키 → `Dispatch(id)`, 기본 키 → 그 id 아님, 기본 chord → 두 번째 단계에서 그 id로 resolve 안 됨. 라우팅 안 되는 행은 keymap 행이 아니고 `runs_via_command`도 아닌 monaco 25개뿐임을 단언하고 목록을 출력.
- [U] `settings-sections-view-tests.rs`: TOC(900px viewport, 미러 8·호스트 14 순서, Lsp 클릭 → 카드가 viewport 위 32px(제목 inset은 Appearance 카드로 보정), 목차가 viewport 위 32px에 고정, Performance 클릭 → 32px 또는 페이지 끝(휠 400px로 더 내려가지 않음)), 대화상자 5종 공통 계약(`assert_dialog_contract`).
- [A] `application-settings-consumers-tests.rs`:
    1. TOC: perf 게이트 켠 실제 App, section 조회가 모두 끝난 뒤 900px로 줄여 14항목 순서 확인, Lsp 클릭 → 활성 Lsp, 카드 제목이 목차 첫 버튼과 같은 높이(+ 제목 inset), Performance 클릭 → 카드가 화면 안.
    2. 글꼴 picker: 4절 4·8 참고, 단계별 `editorFontFamily`(메모리·settings.json)와 trigger 문구 확인.
    3. 환영 화면: `Scene::launch_empty_pane`, off → `editor.noFileOpen`, on → 복귀.
    4. 터미널: `/bin/cat`, cursor `Underline` + blink 반전 + 크기 19 → 실행 중 세션 `core.cursor_style()`가 `terminal_settings::cursor(settings)`와 같고 이전 값과 다름, appearance 글자 크기 19, shell `/bin/../bin/cat` → 기존 세션 shell `/bin/cat` 유지, 새 터미널 shell은 새 값.
    5. accepts 목록: 218행 중 173 수락·45 거절(목록은 대응표 settings-67 비고), 거절 행 중 `shell_keymap::supports`인 것은 없음.

## 6. 남은 범위와 수정 내역

- 중간 실패와 수정: (a) App 환영 화면 검사 첫 실행은 기본 layout에 Welcome 탭이 있어 실패 → `launch_empty_pane`이 탭을 비우게 고침. (b) App 글꼴 검사는 글꼴 항목이 미리보기로 그려져 글자 모양 검색이 0건 → 접근성 노드로 확인. (c) App TOC 검사는 section 데이터가 들어오는 중에 클릭해 카드 위치가 뒤로 밀림 → 조회 완료 뒤 클릭. 셋 다 검사 쪽 원인이며 제품 코드 변경은 3.1의 스크롤 수정뿐입니다.
- settings-23: shell-91(`native_notifications`, `notify_event`, 주입 platform), git-7, explorer-search-33 병합 뒤 메인 통합에서 사양 2B의 묶음 App 검사(범주별 스위치 off → 0회)를 추가하고 complete로 올립니다.
- settings-28: 새 App 소비자 검사(wordWrap 표시 줄 수, rulers 위치, lineNumbers gutter, render whitespace)는 이번에 쓰지 못했습니다(파일 편집기 탭 fixture 필요). 소비자 없음 3종(리거처 한계, semantic = editor lane, diff 2종 = git lane)은 그대로 의존입니다.
- settings-30: terminal-agent-11(실행 중 scrollback) 병합 뒤 "실행 중 세션 scrollback 변경 → history 줄 수" App 검사를 추가하고 complete로 올립니다.
- settings-67: 거절 45개 명령의 소비자는 commands-ui lane(wave 3)과 각 영역 lane 몫입니다.
- settings-78: 남은 94곳은 다른 lane·공용 파일입니다. 메인 통합에서 결정 3 A로 처리한 뒤 상태바 라벨·`status` 필드를 지웁니다.
- settings-80: minimap off App 증거, reveal(explorer-search-25)·searchOnType·debounce(explorer-search-34)·IDE diff 자동 열기(git-49).
- settings-83: 실기 포커스·VoiceOver는 허락 항목(4.2-9). 모든 Monaco 동작 실행은 editor lane.
- App 검사의 기본 LSP 감지 포트는 실제 `PATH`를 읽어 서버 존재만 확인합니다(읽기 전용, 앞 묶음과 같음).

## 7. 수정(리뷰 반영, 2026-11-03)

### 7.1 settings-24 — 과대 판정 반영(비고 보강)

- 리뷰 지적: 사양 [A]의 "선택 → `editor_fonts::Families` 재적재 요청"을 새 검사가 단언하지 않음(설정 값·파일 저장·trigger 문구까지).
- 확인: 지적이 맞습니다. 앱 프레임 `tick_terminal_agent_fonts`(`domains/terminal_agent.rs`)는 매 프레임 `editor_fonts::Families::new(&settings)`를 `terminal_fonts::Loader::update`에 넘기고, Loader는 직전 시도와 다르면 다시 적재합니다. 이 재적재 자체는 단위 검사 `editor_fonts는_편집기_글꼴_설정이_바뀌면_같은_로더에_다시_적재를_요청한다`(`editor-fonts.rs`)가 근거입니다.
- 수정: `application-settings-consumers-tests.rs` `실제_글꼴_picker는_모노스페이스_필터_검색_시스템기본과_선택을_설정에_반영한다`에 실제 앱 설정에서 만든 `Families`의 `editor`가 선택 뒤 `Requested::Named(선택 글꼴)`, 시스템 기본 뒤 `Requested::Default`인지 단언을 더했습니다. Loader의 `attempted`는 `terminal_fonts.rs`(terminal-agent 영역) 비공개 필드라 App 검사에서 직접 읽지 않았고, 그 파일을 고치지 않았습니다.
- 판정: complete 유지, 대응표 비고에 위 연결 근거를 한 줄로 적었습니다.

### 7.2 검증

| 범위 | 명령(공통 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`) | 결과 |
| --- | --- | --- |
| [A] settings-24 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- 실제_보조_창_settings는 실제_auto_tab_토글은 실제_글꼴_picker는 --test-threads=2` | 글꼴 검사 통과(같은 실행 2 passed: settings-5·24) |

### 7.3 리뷰 비차단 항목 처리(lane 전체)

| 항목 | 처리 | 근거·검사 |
| --- | --- | --- |
| hooks 상태 캐시가 OFF→ON 뒤 낡은 채 남음(settings-17) | 수정 | 재현 UI 검사 `hooks_목록은_껐다_켜면_다시_mount되어_오래된_상태를_다시_조회한다`(61초 전 응답으로 캐시 → OFF → ON → 조회 0건으로 실패 확인) 뒤 `OwnerState::is_hooks_list_shown`을 두고 목록이 다시 그려지는 프레임을 `mounting`처럼 `hooks_list`에 넘김. 원본 목록 컴포넌트 재mount + staleTime 60초 재조회와 같음. 같은 검사가 통과, 다음 프레임은 다시 조회하지 않음 |
| `format_bytes`의 반올림이 JS `toFixed(1)`과 동률에서 다름(settings-33 진행 표시) | 수정 | `to_fixed_one`이 `(값 × 10).round() / 10`(동률은 큰 쪽, `toFixed`와 같음) 뒤 소수 1자리로 씀. `바이트_형식은_원본_경계를_따른다`에 1280B → `1.3KB`, 1152B → `1.1KB`, 1.25MiB → `1.3MB` 추가. 수정과 단언을 같은 편집에서 넣어 수정 전 실패 실행은 따로 하지 않았습니다(이전 식 `{:.1}`은 1.25를 짝수 쪽 `1.2`로 씀) |
| 결정 3A의 `log::warn!` 두 곳이 매 프레임 쌓임 | 수정 | `domains/settings.rs` `ReportedFailures`(preview, app file reconcile)와 `unreported_failure`: 같은 문구는 한 번만 기록, 성공하면 지우고 다시 실패하면 다시 기록. 단위 검사 `매_프레임_실패는_같은_문구를_한_번만_기록하고_회복_뒤_다시_기록한다`. 공용 `application.rs` State 생성자에 `reported_failures: Default::default()` 1줄 |
| `Reply::Sections`가 `closing`을 확인하지 않음 | 수정 | 다른 응답처럼 `self.closing.is_none()`일 때만 받음. 종료 중 상태를 만드는 App 검사는 따로 쓰지 않았습니다(기존 `AppFile`·`ThemeEdit` 분기와 같은 조건) |
| `Command::Sections` 제출 실패 시 알림 2개 | 수정 | 재현 App 검사 `실제_sections_요청_제출이_실패하면_오류_알림은_한_번만_뜬다`(bridge를 잠시 빼고 새로고침 클릭 → 수정 전 오류 toast 2개 `native Settings section host is disconnected or full`, `native host is closing`) 뒤 `load_settings_requests`가 실패 응답을 알림 없이 `Data::accept`로만 반영(제출 실패 toast는 `submit`이 이미 띄움). 수정 뒤 1개, 새로고침 진행 표시 해제 |
| `external_url.rs` 문서 주석의 옛 경로 | 수정 | `taide_model::remote::is_valid_allowed_host`로 고침(이 lane이 함수를 옮김) |
| `extract_vsix`의 `theme_list` 실패를 `unwrap_or_default()`로 삼킴 | 유지(리뷰 반박) | 원본 `plugin-manager.tsx:32`가 `const { data: themes = [] } = useQuery(themeListQueryOptions())`라 목록 조회 실패 시 중복 감지가 같은 방식으로 꺼집니다 |
| 비밀번호 입력의 egui undoer가 비밀 텍스트를 메모리에 남김 | 하지 않음 | 메모리 안 위생 항목이고 egui `TextEditState` undoer를 지우는 방식 확인이 필요해 이번에는 손대지 않았습니다(남은 일) |

검증: UI lib `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` 219 passed. 앱 `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- domains::settings:: app_file_views theme_ presentation --test-threads=2` 42 passed(다시 쓴 settings-5, 단언을 더한 settings-24, 새 settings-38·제출 실패 App 검사, 새 단위 검사 1개 포함, 모든 수정 뒤 실행). 앱·UI·taide-theme·taide-infra `cargo check --all-targets` 통과(경고는 기존 것뿐), 동결 미러 wasm check 통과(기존 경고뿐), `cargo fmt -- --check` 네 crate 차이 없음(새 코드의 줄바꿈만 `cargo fmt`로 맞춤), `taide-theme` lib 63 passed.
