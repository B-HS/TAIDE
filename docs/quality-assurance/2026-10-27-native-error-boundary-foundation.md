# 영역 장애 격리 기반 (배치 24 단계 E2 2/3)

## 대상

- shell-12: 전역 ErrorBoundary(위젯별 fallback). 이번 범위는 공용 경계와 Welcome·Activity Bar(rail)·상태 표시줄 세 영역입니다. 사이드바 패널·편집기 경계는 다음 작업입니다.
- 사용자 결정: A안(`2026-10-27-native-error-isolation-file-drop.md` shell-12 절). render를 "snapshot 읽기 + 영역 소유 상태 + intent 출력"으로 바꾼 영역에만 `std::panic::catch_unwind` 경계를 둡니다.
- 원본: `src/shared/ui/error-boundary.tsx`, `src/widgets/app-shell/app-shell.tsx`, `src/widgets/window-chrome/status-bar-content.tsx`

## 원본 동작

- 경계(`error-boundary.tsx`):
    - 32-44: 자식 render/effect 오류를 잡아 `hasError`로 바꾸고 `console.error(error, errorInfo)`로 남깁니다.
    - 46-53: 처음 fallback으로 바뀔 때 fallback 컨테이너에 focus를 옮깁니다.
    - 55, 118-121: 재시도는 state를 초기화합니다. React가 잡은 순간 subtree를 버렸으므로 재시도는 새 mount입니다.
    - 62-81: fallback은 `role='alert'`, `tabIndex=-1` 컨테이너입니다. `bg-app-background text-app-foreground`, 세로 가운데 정렬, `gap-2`(8px), `p-4`, `text-sm`(14px, 줄 높이 20px)입니다.
        - 영역 이름(`t(labelKey)`)과 `errorBoundary.description`(`text-xs` 12px/16px, `text-app-sidebar-icon-default`)을 보입니다.
        - 개발 빌드에서만 `error.stack`을 `pre`로 보입니다. `text-status-error`, 10px, 왼쪽 정렬, 최대 높이 128px, 넘치면 scroll입니다.
        - `common.retry` 버튼은 `Button variant='outline' size='xs'`입니다(`src/shared/ui/button.tsx` 8·14·21: 높이 24px, 가로 8px, 12px medium, `rounded-md`).
- 위치(`app-shell.tsx`):
    - 183-190: 프로젝트가 0개일 때 Welcome(`errorBoundary.welcome`, `h-full w-full`)
    - 194-202: Activity Bar(`errorBoundary.sidebar`, `h-full w-14 shrink-0`). 프로젝트 drag의 `DndContext`(192)와 `DragOverlay`(223)는 이 경계 밖입니다.
    - 226-234: 상태 표시줄(`errorBoundary.statusBar`, `h-6 w-full shrink-0`). 바의 배경·위 테두리는 경계 안 `StatusBar`의 것입니다.
- 상태 표시줄 영역 소유 상태(`status-bar-content.tsx` 47, 151, 169): 시스템 사용량 상세 modal의 열림 상태와 modal 자체가 경계 안입니다. 경계가 잡으면 함께 사라집니다.
- 색: `text-status-error`는 `--taide-status-indicator-error`입니다(`src/shared/styles/global.css` 241).

## 구현

### 공용 경계(`native/taide-native-ui/src/error-boundary.rs`)

- `error_boundary::show(ui, rect, region, colors, text, contents) -> Outcome`:
    - 정상 상태면 `contents`를 `(region, generation)` salt의 child `Ui`(max_rect=영역)에서 `catch_unwind`로 실행합니다. 결과(영역 출력)는 `Outcome::Shown`으로 돌려주고, 셸은 그때만 적용합니다.
    - panic이면 그 pass의 영역 출력을 버립니다. 실패 상태를 저장하고 `request_discard`로 같은 frame을 다시 그립니다. 그 pass와 이후 pass에서 fallback을 그리고 `Outcome::Caught`/`Outcome::Fallback`을 돌려줍니다.
    - 재시도 버튼은 실패를 지우고 generation을 올립니다. 다음 pass의 영역 `Ui` id가 바뀌어 scroll 같은 egui 영역 상태가 새로 시작합니다(원본의 새 mount).
    - 경계 상태는 egui temp data(`(키, 영역, instance, viewport)` id)에 둡니다. 영역 소유 상태라 앱 상태와 섞이지 않습니다.
    - mount 수명(리뷰 수정, 아래 "리뷰 수정" 절): 상태에 마지막으로 그린 frame 번호를 두고, 원본에서 조건부 렌더로 unmount되는 영역(Welcome·Activity Bar·상태 표시줄)이 직전 frame에 그려지지 않았으면 실패를 지우고 generation을 올려 새 mount로 시작합니다. 원본 `Panel`이 접혀도 children을 남기는 사이드바 패널은 실패를 유지합니다.
- panic 정리(vendored egui `native/taide-native-app/vendor/egui-input/src` 근거):
    - `ui.rs` 2207-2219 `scope_dyn`: 경계 child `Ui`는 panic 뒤에도 남고, 그 안의 자손 `Ui`는 unwind 중 `Drop`(2893-2902)으로 min_rect만 기록하고 사라집니다. 커서가 어긋난 child는 버리고, fallback은 같은 rect의 새 child에 그립니다.
    - egui 잠금: `ctx.write`류 closure 안에서 panic하면 guard가 unwind로 먼저 풀립니다. 이어지는 `Ui::drop`이 잠금을 다시 잡아도 교착이 없습니다.
    - 열린 popup: `memory/mod.rs` 824-837 `end_pass`는 그 pass에 다시 보이지 않은 popup을 닫습니다. panic pass 뒤 같은 frame의 discard pass에는 영역이 그려지지 않으므로, 영역이 연 메뉴는 그 frame 안에 닫힙니다.
    - discard: `context.rs` 1018-1070 `run_dyn`은 discard pass에 입력 event를 다시 주지 않습니다(`new_input.take()`). 다른 영역의 클릭이 두 번 적용되지 않습니다. `max_passes`(기본 2)를 넘으면 discard가 무시되는데, 그때도 같은 pass에서 fallback이 영역을 덮습니다.
    - id 충돌: fallback 위젯은 별도 salt라 panic pass에 일부 등록된 영역 위젯 id와 겹치지 않습니다. 재시도 뒤 영역 id는 새 generation입니다.
    - drag: egui `dragged_id`는 pointer를 놓을 때까지 남지만(`interaction.rs` 124-131) 읽는 위젯이 없어 효과가 없습니다.
- panic hook과 로그: 개발 빌드에서 처음 경계를 쓸 때 기존 hook을 감싼 hook을 한 번 설치합니다.
    - 경계 안(thread-local 깊이 > 0)의 panic이면 `panicked at 위치:` + 메시지 + backtrace 문자열을 thread-local에 남깁니다. 경계가 잡은 뒤 꺼내 fallback stack으로 씁니다.
    - 항상 이전 hook을 그대로 부릅니다. 기본 hook의 stderr 기록이 원본 `console.error`에 대응하는 로그입니다.
    - release 빌드는 hook을 바꾸지 않고 stack도 보이지 않습니다(원본 `import.meta.env.DEV`).
- fallback UI:
    - 영역 rect를 `app.background`로 채우고 그 rect로 clip합니다. accesskit role `Alert`인 focus 가능 컨테이너를 등록하고, 처음 그릴 때(discard되지 않는 pass) focus를 옮깁니다. `Alert` 노드의 label은 영역 이름, description은 `errorBoundary.description`입니다(리뷰 수정: 원본 `role='alert'` 컨테이너는 안의 글 전체가 낭독되는데, egui Label은 이 노드의 자식이 아니라 노드 자체에 이름·설명을 둡니다).
    - Welcome·Activity Bar: 세로 가운데 배치, 8px 간격입니다.
        - 영역 이름 14px/20px foreground, 설명 12px/16px `appSidebar.iconDefault`
        - 개발 빌드 stack: 10px monospace, `statusIndicator.error`(egui `error_fg_color`), 왼쪽 정렬, 최대 128px scroll. 줄 높이는 원본처럼 `text-sm` 비율(20/14)을 이어받습니다.
        - 재시도 버튼: 12px medium, 높이 24px, 가로 padding 8px, 반경 6px
    - 상태 표시줄: 같은 항목을 가로 한 줄 가운데에 둡니다(아래 "원본과 다르게 둔 점").
    - 문구는 기존 locale 키 `errorBoundary.welcome`·`errorBoundary.sidebar`·`errorBoundary.statusBar`·`errorBoundary.description`·`common.retry`입니다.
- 검사 전용 주입: `cfg(test)` 또는 `inspection` feature에서만 `error_boundary::inject_panic(ctx, region)`이 있습니다. 다음 pass에서 그 영역 `contents`가 끝까지 실행된 직후(출력이 다 쌓인 뒤) panic합니다. release 앱에는 컴파일되지 않습니다.

### 영역 전환(intent 출력)

- Welcome(`shell.rs` `main_window`): 이미 `&impl ShellSurfaces` 읽기와 intent 출력만 합니다. 영역 지역 intent 목록에 모았다가 `Shown`일 때만 셸 intent에 더합니다.
- Activity Bar:
    - `project_rail`·`project_button`·`project_tooltip`은 `&impl ShellSurfaces`(읽기)와 영역 출력 `RailOutput { intents, icons }`만 받습니다.
    - 아이콘은 앱의 공유 glyph texture cache(`&mut`)로 그립니다. 그래서 영역 안에서는 `Painter::add(Shape::Noop)`로 자리만 예약하고 요청(`DeferredIcon`)을 남깁니다. `Shown`일 때 셸이 `ShellSurfaces::rail_icon(painter, slot, ..)`으로 그 자리를 채웁니다(`Painter::set`). 예약한 자리라 그리는 순서(배지·tooltip 안 위치)는 그대로입니다. drag 미리보기(경계 밖)는 같은 경로로 바로 채웁니다.
    - 프로젝트 drag 상태(`project_drag` Memory)는 원본 `DndContext`처럼 경계 밖 창 단위 상태라 버리지 않습니다. rail 항목 등록은 호출마다 원자적이고 다음 frame `begin_frame`이 새로 시작합니다. 그래서 원본처럼 rail이 fallback이어도 진행 중인 slot drop이 끝까지 동작한다고 판단합니다. 이 주장은 구조 근거(`project_drag` Memory가 경계 밖)만 있고 대응 검사는 없습니다.
- 상태 표시줄:
    - 바 배경·위 테두리를 경계 안으로 옮겼습니다(원본 `StatusBar` 소유). fallback이 바 전체를 덮습니다.
    - `ShellSurfaces`에 `StatusOutput` 연관 타입과 `prepare_status_bar(&mut)`, `status_bar(&self, .., &mut StatusOutput)`, `apply_status_bar(&mut, StatusOutput)`, `discard_status_bar(&mut)`를 두었습니다. 경계 밖에서 준비하고, 경계 안에서는 읽기만 하고, `Shown`일 때 적용하고, `Caught`일 때 영역 소유 앱 상태를 버립니다.
    - 앱(`application.rs` `AppSurfaces`):
        - 준비: 문제 버튼 glyph(`problems::Views::prepare_status`), IDE 상태 아이콘(`status_ide::Icons::prepare`), 시스템 사용량 아이콘(요약이 있을 때만 `Icon::prepare`) texture를 만듭니다. 실패는 기존처럼 상태 문구로 보이고 그 항목은 그리지 않습니다(원본도 오류면 그 항목을 그리지 못함).
        - 읽기: `problems::Views::status_button(&self)`, `status_ide::show_prepared`, `system_usage_view::show_prepared_status`를 씁니다. 문제 토글 횟수, 글꼴 크기 `HostCommand`, 사용량 상세 열기 요청, 오류 문구를 `StatusBarOutput`에 모읍니다.
        - 적용: focused slot 문제 panel 토글, `commands` 추가, `system_usage.detail_open = true`, 마지막 오류를 상태 문구로 둡니다(기존 "마지막 오류가 남음"과 같음).
        - 버림: `system_usage.detail_open = false`. 원본 상세 modal이 경계 안 상태라 함께 사라지는 것에 대응합니다.
        - 문제 토글 버튼의 Space 눌림 상태(`SpaceInputs`)는 `Views` 필드에서 그 버튼 id의 egui temp data로 옮겼습니다. 영역 id 아래라 재시도 때 새로 시작합니다.
    - 기존 `show_status`(problems)·`status_ide::show`·`system_usage_view::show_status`는 준비+읽기를 합친 형태로 남겨 기존 단위 검사가 그대로 씁니다. 앱 경로에서는 쓰지 않아 `cfg(test)`입니다.
- 공유 glyph cache(`taide-native-ui/src/glyph-icons.rs`): `paint`를 `prepare`(texture 생성) + `paint_prepared`(읽기)로 나누고, 같은 mesh를 돌려주는 `shape`를 더했습니다. 결과는 기존 `Image::paint_at`(회전 mesh, 픽셀 반올림 rect)과 같습니다(vendored `widgets/image.rs` 369-396, 840-875). 문제 panel의 severity 버튼은 그리기 전에 `prepare`를 부릅니다.
- 경계 안에 남긴 공유 상태와 그 근거:
    - tooltip provider(`tooltips::Provider`)와 egui Memory: 호출마다 잠금 안에서 원자적으로 바뀌고, 보이지 않은 위젯 항목을 pass 단위로 정리하는 UI 기반 상태입니다. provider는 std `Mutex`라 provider 자체가 panic하면 poison으로 이후 호출이 실패합니다. 이 경우는 지금처럼 앱 종료로 이어집니다.
    - Welcome·rail `+` 메뉴의 최근 목록 조회 요청 표시(`welcome.rs` `State::wanted: Cell<bool>`): 그리는 동안 `true`로만 바뀌는 수요 신호입니다. 중간에 panic해도 깨질 불변식이 없고, 원본도 query 구독이 render 중 시작됩니다.
    - 설정·에이전트·IDE 상태 조회: `parking_lot` 읽기 잠금만 씁니다.

### 원본과 다르게 둔 점(근거)

- 상태 표시줄 fallback 배치: 원본은 24px 바에 세로 column(`p-4`, `gap-2`)을 넣습니다. border-box라 상자는 32px가 되고, 내용은 위로 본문을 덮고 아래로 창 밖에 잘립니다. 재시도 버튼이 거의 보이지 않는 결함입니다. native는 같은 항목을 가로 한 줄에 두어 바 안에서 모두 보이고 누를 수 있게 했습니다. 이 배치에서는 개발 빌드 stack을 바에 넣지 않고 hook 로그(stderr)로만 남깁니다.
- 넘침: 원본 fallback은 overflow가 보이는 상태라 rail(56px)에서 글이 `p-4` 안쪽 폭(24px)을 넘어 padding 위로 넘칩니다. native는 영역 rect로 clip하고 영역 폭 전체를 줄바꿈 폭으로 씁니다. 줄바꿈 위치·픽셀 대조는 배치 31입니다.
- 상태 표시줄의 chord 불일치 깜빡임(`chordNoMatchFlash`, `status-bar-content.tsx` 48)은 native에서 keymap 공유 상태라 버리지 않습니다. 몇백 ms 표시 상태입니다.
- 사용량 상세 modal은 native에서 상태 표시줄 밖에서 그려집니다. 경계가 잡으면 `detail_open`을 내려 닫습니다. 원본 unmount는 닫힘 animation 없이 사라지지만 native는 기존 닫힘 경로를 탑니다.

## 변경 파일

- 새 파일: `native/taide-native-ui/src/error-boundary.rs`, `native/taide-native-app/src/application-error-boundary-tests.rs`
- `native/taide-native-ui/src/lib.rs`: `error_boundary` 모듈 연결
- `native/taide-native-ui/src/shell.rs`: 세 경계, `ShellSurfaces` 상태 표시줄 메서드와 `rail_icon(painter, slot, ..)`, `RailOutput`·`DeferredIcon`
- `native/taide-native-ui/src/glyph-icons.rs`: `prepare`·`paint_prepared`·`shape`(지정 범위 밖 최소 수정: 공유 glyph cache를 경계 밖에서 준비하고 안에서 읽기만 하려고 나눔)
- `native/taide-native-ui/tests/workbench.rs`: 검사 surfaces의 새 trait 형태, 경계 검사 3건
- `native/taide-native-app/src/application.rs`: `AppSurfaces`의 상태 표시줄 준비·읽기·적용·버림, `rail_icon`, 검사 모듈 연결
- `native/taide-native-app/src/problems.rs`, `status-ide.rs`, `system-usage-view.rs`: 준비와 읽기 분리(지정 범위 밖 최소 수정: 상태 표시줄을 읽기 전용으로 바꾸는 데 필요)
- Cargo.toml·Cargo.lock, 동결 remote-web은 바꾸지 않았습니다. remote-web은 `ShellSurfaces`를 구현하지 않습니다.

## 검증

- 실제 앱 경로: `main.rs` → `NativeApplication::ui` → `show_window` → `NativeShell::show` → `main_window` → `error_boundary::show`(상태 표시줄·Welcome·Activity Bar) → `AppSurfaces::{prepare_status_bar, status_bar, apply_status_bar, discard_status_bar, rail_icon}`.
- 검사 순서: 새 검사는 구현과 같은 단계에서 썼습니다. 구현 전에는 `error_boundary` 모듈과 새 trait 메서드가 없어 컴파일되지 않으므로 구현 전 실패 실행은 따로 하지 않았습니다(구현을 끄는 재현은 작업 규칙상 하지 않음). 첫 실행의 실패는 모두 검사 쪽 결함이었고 고친 뒤 통과했습니다.
    - 재시도 직후 같은 frame의 클릭: egui는 직전 pass 위젯 rect로 hit-test하므로 새로 그려진 위젯은 다음 frame부터 눌립니다. 재시도 뒤 한 frame을 더 그린 다음 누르게 고쳤습니다.
    - Welcome `app.openFolder` 위치를 글자 포함 검색으로 찾아 `app.openFolderHint`를 눌렀습니다. 접근성 버튼 이름으로 찾게 고쳤습니다.
    - 앱 검사에서 영역 이름·설명을 accesskit label로 찾았습니다. egui Label은 글을 label이 아닌 value로 내보내므로 그려진 글자로 찾게 고쳤습니다.
- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench --locked --offline --target-dir …`: 38/38 통과(11.10초). 새 3건:
    - 상태 표시줄: 검사 surfaces의 `status_bar`가 출력을 쌓은 뒤 실제로 panic합니다. 그 frame의 출력은 적용되지 않고(`status_projects` 비어 있음) `discard_status_bar`가 한 번 불립니다. 바 rect 그대로의 `Alert`에 focus가 있고, 영역 이름·설명·재시도가 보이며, rail과 slot은 그대로 그려집니다. panic이 계속되는 동안 fallback이 유지되고 버림은 다시 불리지 않습니다. 재시도 뒤 바가 다시 그려지고 출력이 적용됩니다.
    - Activity Bar: 주입 panic과 같은 frame의 프로젝트 클릭 요청과 지연 아이콘이 버려집니다. fallback은 rail rect이고 slot·상태 표시줄은 유지됩니다. 개발 빌드 stack(`panicked at …` + 메시지)이 보입니다. 재시도 뒤 아이콘이 다시 채워집니다. `+` 메뉴가 열린 채 panic하면 그 frame 안에 popup이 닫히고, 재시도 뒤 메뉴는 닫힌 새 상태이며 프로젝트 클릭이 다시 요청을 냅니다.
    - Welcome: 주입 panic과 같은 frame의 `app.openFolder` 클릭이 버려지고 fallback이 본문 rect를 덮습니다. 상태 표시줄은 유지됩니다. 재시도 뒤 `app.openFolder`가 다시 요청을 냅니다.
- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib …`: 163/163 통과(0.53초). glyph cache 단위 검사를 포함합니다.
- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir … -- problems:: problems_icons:: status_ide:: system_usage_view:: status_editor:: welcome:: project_rail_tests error_boundary_tests`: 40/40 통과(85.89초).
    - 새 실제 App 2건(`application-error-boundary-tests.rs`):
        - 프로젝트 1개: 사용량 상세를 연 상태에서 상태 표시줄 panic을 주입하면 상세가 닫히고(`discard_status_bar`), 문제 토글 버튼 대신 fallback이 보이며 rail 프로젝트 버튼은 남습니다. 재시도 뒤 문제 토글이 돌아옵니다. 이어서 rail panic은 rail 버튼만 fallback으로 바꾸고 상태 표시줄은 유지하며, 재시도 뒤 rail 버튼이 돌아옵니다. 지연 경로로 프로젝트 glyph texture(`native-glyph-Rocket-*`)가 만들어집니다.
        - 프로젝트 0개: Welcome panic은 `app.openFolder` 대신 fallback을 보이고 상태 표시줄은 유지합니다. 재시도 뒤 `app.openFolder`가 돌아옵니다.
    - 바꾼 경로와 맞닿은 기존 검사: 문제 상태 버튼·panel(Space·Enter·accesskit 토글 포함), IDE 상태, 시스템 사용량, 글꼴 크기 버튼, Welcome, 실제 App rail 4건(glyph texture·메뉴·그룹·drag)입니다.
- `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline …`: exit 0(기존 경고만).
- `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection …`와 기본 feature `--lib`: exit 0(기존 경고만).
- `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline …`: exit 0(기존 경고만). remote-web `Cargo.lock`은 바뀌지 않았습니다.
- `cargo fmt … -- --check`: UI crate는 차이 없음입니다. 앱 crate의 남은 차이는 모두 기존 코드입니다(`experiments/lsp-coordinator-spike/src/bin/mock-server.rs`, `application.rs` 960행 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서).
- 실행하지 않은 것: 필터 없는 앱 전체 테스트(배치 끝 메인 담당), GUI 실기. 앱 검사 binary 링크 때 macOS 링커가 `__eh_frame` 크기로 compact unwind를 쓰지 못한다는 경고를 냅니다. unwind 성능 경고이고 panic 포착은 위 검사로 동작을 확인했습니다.

## 리뷰 수정 (차단 2건과 비차단 일부)

### 차단 1: panic pass의 intent 폐기 단언이 아무것도 검증하지 못함

- 확인: 리뷰 지적이 맞았습니다. `rail_frame`·`welcome_frame`(workbench)이 `context.run_ui`의 매 pass마다 `intents = shell.show(..)`로 덮어써 마지막 pass의 intent만 돌려줬습니다. panic frame은 `request_discard`로 pass 2가 돌고 입력 event는 pass 1에만 들어가므로(vendored `context.rs` `run_dyn`의 `new_input.take()`), 마지막 pass의 intent는 항상 비어 있었습니다.
- 수정(검사 하네스): 두 하네스가 모든 pass의 intent를 모읍니다(`intents.extend(shell.show(..))`). 앱도 eframe pass마다 `ui`를 불러 그 pass의 intent를 적용하므로 이쪽이 실제 경로에 가깝습니다. 하네스를 쓰는 기존 검사 43건은 모두 그대로 통과했습니다(기존 검사는 단일 pass frame이라 결과가 같음).
- 대조군: 검사 surfaces에 `is_first_pass_discarded`(상태 표시줄 `status_bar`가 pass 0에서 `request_discard`)를 두고 `discarded_frame_click`을 더했습니다. panic 없이 같은 모양(2 pass, 클릭은 pass 1)의 frame에서 프로젝트 클릭은 활성화 intent를, Welcome `app.openFolder` 클릭은 `OpenFolder`를 냅니다. 같은 클릭이 panic frame에서는 나오지 않습니다. 두 frame 모두 `cumulative_pass_nr` 차이가 2인 것도 단언합니다. 그래서 `Outcome::Caught`에서 영역 intent를 셸 intent에 더하도록 바뀌면 panic frame 단언이 실패합니다.
- 리뷰가 권한 "Caught 구현을 일부러 바꿔 실패 확인"은 작업 규칙(구현을 끄거나 되돌려 실패를 재현하는 편집 금지)상 하지 않았고, 위 대조군으로 대신했습니다.

### 차단 2: 영역이 사라졌다 다시 보여도 실패 상태가 남음

- 확인: 리뷰 지적이 맞았습니다. 상태는 `(영역, instance, viewport)` id로만 보관되고 수명 추적이 없었습니다. 또 정상(`Shown`) 경로는 상태를 저장하지 않고 바로 돌아갔습니다.
- 원본 근거:
    - `src/widgets/app-shell/app-shell.tsx` 183-190(`projects.length === 0`일 때만 Welcome 경계), 194-202(`!zen && !sidebarRailCollapsed`일 때만 Activity Bar 경계), 226-234(`!(zen && hideStatusBar)`일 때만 상태 표시줄 경계): 조건이 거짓이 되면 경계가 unmount되어 `hasError`가 사라지고, 다시 참이 되면 새 인스턴스(`state = INITIAL_ERROR_BOUNDARY_STATE`, `src/shared/ui/error-boundary.tsx` 20, 33)로 시작합니다.
    - 사이드바 패널은 다릅니다. `src/widgets/app-shell/project-shell.tsx` 142-152와 `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 131-134에서 경계는 `collapsible collapsedSize={0}` `Panel` 안에 있고, 접기(126-134의 `panel.collapse()`, Zen 포함)는 크기만 바꿉니다. `react-resizable-panels` 4.12.2 `Panel`(`node_modules/react-resizable-panels/dist/react-resizable-panels.js` 1952-2075)은 접힘과 관계없이 `children`을 항상 렌더합니다(2053-2069). 그래서 원본 사이드바 패널 fallback은 접었다 펴도 남습니다.
- 수정(`native/taide-native-ui/src/error-boundary.rs`):
    - `State`에 마지막으로 그린 frame 번호(`frame: Option<u64>`, `ctx.cumulative_frame_nr()`)를 두고, `show_instance`는 매 호출(정상·실패·fallback 모두) 끝에 상태를 저장합니다.
    - `Region::is_unmounted_while_hidden()`(사이드바 패널만 거짓)인 영역이 직전 frame에 그려지지 않았으면(`저장된 frame + 1 < 지금 frame`) 실패를 지우고 generation을 올립니다. 새 id라 영역의 egui 상태(scroll·popup·Space 눌림 등)도 새로 시작합니다(원본 새 mount).
    - `cumulative_frame_nr`는 viewport별이고 `run_ui` 끝에 한 번만 오릅니다(vendored `context.rs` 1080, 1984-2011). 같은 frame의 discard pass는 번호가 같아 영향이 없습니다. 같은 "직전 frame에 그렸는가" 판정은 앱의 `editor_keymap_targets` 정리(`application.rs` 7898-7906)도 씁니다.
- 검사(workbench, 새 3건):
    - `welcome_fallback은_프로젝트가_생겨_사라졌다가_다시_0개가_되면_새_welcome으로_그린다`: 프로젝트 0개에서 Welcome panic → fallback이 다음 frame에도 남음 → 프로젝트가 생긴 frame(Welcome 없음) → 다시 0개가 되면 `Alert` 없이 `app.openFolder`가 보입니다.
    - `액티비티바와_상태표시줄_fallback은_숨겼다_다시_보이면_새로_그린다`: 두 영역 panic → fallback 유지 확인 → Zen(`hide_status_in_zen`)으로 둘 다 숨김 → 해제하면 둘 다 정상으로 그려지고 상태 표시줄 출력이 적용되며 rail 지연 아이콘이 다시 채워집니다. 이어서 rail만 panic → `sidebar_rail_collapsed`로 숨김 → 펼치면 rail이 정상입니다.
    - `사이드바_패널_fallback은_원본처럼_접었다_펴도_남는다`: focused slot 패널 panic → 그 프로젝트 `sidebar_collapsed`로 접었다 펴도, Zen 진입·해제 뒤에도 `errorBoundary.sidebarPanel` fallback과 `Alert`가 남습니다.
- 작업 순서: 이 차단 항목은 구현 수정을 먼저 적고 검사를 뒤에 썼습니다. 고친 뒤에 수정 전 실패를 보이려면 구현을 되돌려야 해서 작업 규칙상 하지 않았습니다. 수정 전 코드는 `Shown` 경로가 상태를 저장하지 않고 frame을 기록하지 않아, 위 검사의 "다시 보이면 새로 그림" 단언이 성립할 수 없는 구조였습니다.

### 반영한 비차단

- `Alert` 노드 이름·설명: 위 fallback UI 항목. workbench `assert_fallback`이 `Alert` 노드의 label(영역 이름 키)과 description(`errorBoundary.description`)을 단언합니다.
- 운영 경로의 `Shown` 적용 검사: 실제 App 검사 `실제_app은_재시도로_다시_그린_상태표시줄의_문제_토글_클릭을_focused_slot에_적용한다`를 더했습니다. 상태 표시줄 panic → fallback → 재시도 뒤 문제 토글 버튼을 누르면 `StatusBarOutput.problem_toggles` → `AppSurfaces::apply_status_bar` → `problems.toggle(focused_slot)`로 `application.problems.is_open(slot)`이 참이 됩니다(`application.rs` 10378-10391).
- slot drop 주장: 구조 근거만 있다고 고쳐 적었습니다(위 Activity Bar 항목).

### 반영하지 않은 비차단(사유)

- 상태 표시줄 fallback 동안 사용량 sampling 지속: 자원 낭비 수준이라 이번 범위에서 두지 않았습니다.
- `panic = "unwind"` 전제 장치: compile-time 단언은 `wasm32-unknown-unknown`(기본 abort)로 빌드하는 remote-web이 같은 UI crate를 쓰므로 그 빌드를 깨뜨립니다. 빌드 설정을 옮길 때 확인할 사항으로만 남깁니다.
- glyph cache `prepare`/`paint_prepared`/`shape` 단위 검사, 실제 App rail 재시도 뒤 mesh 확인: 워크벤치 `rail_icons` 기록과 기존 glyph 검사로 대신합니다.
- 검사용 `inject_panic` 보류, 전역 panic hook 1회 설치: 정보성 지적이라 바꾸지 않았습니다.

### 리뷰 수정 검증

- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench --locked --offline --target-dir …`: 43/43 통과(10.63초). 경계 검사 8건(기존 5건 + 새 3건)과 하네스를 쓰는 기존 검사 전부를 포함합니다.
- `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib …`: 163/163 통과(0.52초).
- `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib … -- problems:: problems_icons:: status_ide:: system_usage_view:: status_editor:: welcome:: project_rail_tests error_boundary_tests`:
    - 기본 병렬: 41/42 통과 두 번. 실패는 두 번 모두 `project_rail_tests::실제_app_그룹은_새그룹_생성_그룹추가_수정_열기결과_toast와_삭제확인을_runtime에_반영한다`(`application-project-rail-tests.rs` 793: 그룹 열기 결과 toast 대신 `native host command queue is full`, `host.rs` 16 `HOST_COMMAND_CAPACITY` 64)입니다.
    - 새 App 검사를 `--skip`해도 40/41로 같은 실패, 그 검사만 단독으로는 통과(49.50초), `--test-threads=3`으로는 42/42 통과(109.02초)했습니다. 동시에 도는 실제 App 검사 수에 따라 host 큐가 차는 부하 의존 실패로 판단합니다. 이번 변경은 host 명령을 만들지 않고, 그 검사는 영역을 숨기지 않아 새 mount 분기에 닿지 않습니다.
- `cargo check --quiet` UI `--all-targets --features native-host,inspection`, UI 기본 feature `--lib`, 앱 `--all-targets`, remote-web `--target wasm32-unknown-unknown --lib`: 모두 exit 0(기존 경고만). remote-web·각 `Cargo.lock`·`Cargo.toml`은 바뀌지 않았습니다.
- `cargo fmt … -- --check`: UI crate 차이 없음(이번에 쓴 줄은 `cargo fmt`로 맞춤). 앱 crate의 남은 차이는 위 기존 항목(`mock-server.rs`, `application.rs` 960행·테스트 모듈 순서, `lib.rs` 모듈 순서)뿐이고 새 검사 파일은 차이가 없습니다.
- 리뷰 수정 변경 파일: `native/taide-native-ui/src/error-boundary.rs`, `native/taide-native-ui/tests/workbench.rs`, `native/taide-native-app/src/application-error-boundary-tests.rs`, `docs/quality-assurance/2026-10-09-native-function-matrix.md`(shell-12 근거), 이 문서.

## 남은 범위

- 사이드바 패널·편집기(메인·보조 창) 경계: 공유 `EditorStore` 편집과 explorer 상태 변경을 render 밖으로 옮긴 뒤 다음 작업에서 적용합니다. 그 전까지 이 영역의 panic은 지금처럼 앱 종료와 hot-exit·세션 복원(shell-25)입니다.
- 루트 `errorBoundary.app` 경계: native에는 아직 없습니다.
- fallback 픽셀 대조(색·버튼 테두리·그림자·줄바꿈)는 배치 31입니다.
