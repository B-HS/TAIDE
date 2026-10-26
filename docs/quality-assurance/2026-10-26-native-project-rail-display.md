# 프로젝트 rail 표시 (glyph·rootMissing·에이전트 배지)

## 대상

- shell-33: 프로젝트 glyph. icon/label/color 표시와 `resolveProjectDisplay`를 포함합니다.
- shell-34: rootMissing 표시. dim과 tooltip 경고를 포함합니다.
- shell-35: 에이전트 상태 배지와 세션 tooltip을 포함합니다.
- 원본: `src/features/project/project-display-glyph.tsx`, `src/features/project/project-icon-button.tsx`, `src/features/project/agent-status-badge.tsx`, `src/widgets/app-sidebar/sortable-project-icon.tsx`, `src/widgets/app-sidebar/app-sidebar.tsx`

## 원본 동작

- 표시 해석(`src/shared/lib/project-display.ts` 82-91, 100-101):
    - label, icon, color를 각각 trim해 빈 값이면 없음으로 봅니다.
    - label이 있으면 label 모드이고 4코드포인트로 자릅니다(`PROJECT_LABEL_MAX_CODEPOINTS`, `src/shared/constants/project-display.ts` 9). 없고 icon이 있으면 icon 모드, 둘 다 없으면 기본(폴더) 모드입니다.
    - color는 `lane1`~`lane12`와 정확히 같을 때만 `--taide-graph-laneN`으로 씁니다(같은 파일 27-40, 48). 세 모드 모두에 적용되고 배경이 아니라 글자·아이콘 색만 바꿉니다.
    - label 글자 크기는 글자 수 N에 따라 16/16/14/11/9px이고 4글자는 `tracking-tight`입니다(같은 파일 19).
- glyph(`project-display-glyph.tsx` 21-33): label은 `font-semibold leading-none`, 아이콘은 `size-5`(20px)입니다. 아이콘 id는 카탈로그 56개(`src/shared/icons/project-icon-registry.ts` 74-131)에서 찾고, 없으면 `folder`입니다(140, 206-209).
- 버튼(`project-icon-button.tsx` 42-60):
    - 40px, `rounded-md`입니다. 기본 글자색은 `appSidebar.iconDefault`, hover 배경은 `appSidebar.itemHover`입니다. 활성이면 배경 `appSidebar.itemActive`, 글자 `app.foreground`이고 버튼 왼쪽 6px 밖에 2x20 `app.accent` 표시를 그립니다.
    - 내부 wrapper `overflow-hidden`이 라벨을 버튼 안으로 자릅니다.
    - `aria-label`은 이름입니다. 배지가 보이면 `이름 — 상태`입니다.
- rootMissing(`sortable-project-icon.tsx` 107-114, 133, 145-154): 버튼 전체가 `opacity-50`입니다. 클릭은 그대로 됩니다. tooltip(side right)은 이름, `TriangleAlert`(12px, gap 4px)와 `app.recentProjectRootMissing`, root(opacity 70)를 순서대로 보입니다.
- 에이전트(`app-sidebar.tsx` 84-90, `sortable-project-icon.tsx` 48-59, 106, 155-162):
    - 프로젝트별 `agent_list` 조회와 `agent:state-changed` push(`src/entities/agent/agent.query.ts` 16-21, 61-66)로 받은 목록을 씁니다.
    - 배지는 우선순위 awaitingInput > working > idle > unknown의 첫 최댓값 하나만 대표합니다. `agentStatusBadgeEnabled`(기본 true)가 꺼지면 배지와 aria 상태를 숨깁니다.
    - tooltip은 설정과 관계없이 모든 에이전트를 `agent.sessionTooltip`({name, status})으로 opacity 70에 보입니다.
    - 상태 문구 키(`src/shared/lib/agent-status-text.ts` 12-32)는 awaitingInput에 차단 이유가 있으면 `agent.blocked.*`, 그 밖에는 `agent.status.<activity>`입니다.
- 배지 모양(`agent-status-badge.tsx` 9-18): 버튼 오른쪽 아래에 붙습니다(`absolute right-0 bottom-0`). 모두 2px `appSidebar.background` ring이 있습니다.
    - working: 10px 채운 원이고 `motion-safe:animate-pulse`(2초, 50%에서 opacity 0.5, cubic-bezier(0.4,0,0.6,1))입니다.
    - awaitingInput: 12px 정사각형을 45도 돌려 채웁니다.
    - idle/unknown: 8px 원에 1px 테두리이고 속은 투명합니다.
- Rust 데이터: `poll_agents`가 열린 프로젝트마다 감지 결과를 `AgentStore::diff`에 저장하고 바뀌면 `AgentStateChanged`를 발행합니다(`crates/taide-runtime/src/agent_actions.rs` 135-174, `crates/taide-agent/src/store.rs` 42-55). native 앱은 시작할 때 이 poll을 띄웁니다(`native/taide-native-app/src/application-ports.rs` 30-55, `application.rs` 668). 이벤트가 오면 `PaintSink`가 다시 그립니다(`application.rs` 56-63).

## 구현

- UI:
    - `project-rail.rs`를 추가했습니다. 표시 해석, label 크기·자간 사다리, 상태 문구 키, 대표 에이전트, pulse 불투명도, 배지 상자·모양, `RailColors`, `RailIcon`을 담습니다.
    - `ShellColors.rail`은 `appSidebar.itemHover/itemActive`, `app.accent`, `appSidebar.iconAgent*`, `graph.lane1..12`를 테마에서 읽습니다(`presentation.rs`).
    - `ShellSurfaces`에 `project_agents`, `is_agent_status_badge_enabled`, `is_reduced_motion`, `rail_icon` 기본 구현을 추가했습니다. remote-web은 이 trait을 구현하지 않으며 소스는 그대로입니다.
    - `shell.rs`의 `project_button`이 원본 버튼을 그립니다. 활성/hover 배경, 활성 표시, label 또는 아이콘 glyph, 사용자 색, rootMissing 흐림, 배지를 포함합니다. 접근성 이름은 `이름 — 상태`입니다.
    - 활성 표시와 배지는 버튼 밖으로 나가므로 rail 폭 clip painter로 그립니다.
    - working 배지는 reduced motion이 아니면 pulse를 계산하고 다시 그리기를 요청합니다.
    - tooltip은 원본 줄 순서대로 오른쪽(`RectAlign::RIGHT`)에 보입니다.
    - `glyph-icons.rs`에 카탈로그 56개(`PROJECT_ICONS`)와 `project()`를 추가했습니다. 모르는 id는 folder로 풀립니다. lucide-react 1.28.0(`package.json` 56)의 아이콘 노드에서 새 SVG 43개를 `resources/project-icons/`에 옮겼고, 기존 13개 SVG(braces 포함)는 재사용합니다.
- App:
    - `AppSurfaces`가 `services.agents.agents_for(project)`(poll이 저장한 최신 목록)와 `settings.agent_status_badge_enabled`를 돌려줍니다.
    - reduced motion은 기존 `motion_preference`로 정합니다.
    - 아이콘은 기존 `explorer_icons`(`problems_icons`) painter로 `project()`/`TriangleAlert`를 칠합니다.

## 작은 결정과 근거

- 키보드 focus 표시: 원본 버튼에는 별도 class가 없고 브라우저 기본 focus ring이 쓰입니다. 그래서 focus일 때 `app.focusBorder` 1px 안쪽 테두리로 대신합니다. 이전 `egui::Button`도 focus 표시를 그렸습니다.
- hover와 활성이 겹칠 때: Tailwind variant 순서상 `hover:bg-*`가 이기므로 hover 배경을 씁니다.
- tooltip 글꼴과 지연: 기존 shell tooltip 채널(egui tooltip)을 그대로 씁니다. 줄 간격은 0입니다(`flex-col` 무간격).

## 실제 앱 경로

`main.rs` → `NativeApplication::new`(agent poll 시작, `application.rs` 668) → `eframe::App::ui` → `self.shell.show(ui, &snapshot, &mut surfaces)`(`application.rs` 7392) → `NativeShell::project_rail` → `project_button`(`shell.rs` 507) → `AppSurfaces::project_agents`/`is_agent_status_badge_enabled`/`rail_icon`(`application.rs` 8434-8465)입니다.

## 검증

- UI 단위: `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --features native-host,inspection --lib -- project_rail:: glyph_tests::`는 13통과입니다.
    - 표시 해석: label 우선, trim, 4코드포인트 자름, 빈 label이면 icon, lane 토큰 정확 일치. 글자 크기·자간 사다리도 확인했습니다.
    - 그 밖에 상태 문구 키, 대표 에이전트 첫 최댓값, pulse(0초 1.0, 1초 0.5), 배지 상자 위치·크기를 확인했습니다.
    - 카탈로그 56개 id·glyph 중복 없음과 원본 순서(code…package), 모르는 id·대소문자·공백·`constructor`의 folder fallback을 확인했습니다. 56개 SVG는 서로 다른 비어 있지 않은 raster입니다.
- UI 실제 egui(`--test workbench -- 프로젝트_rail`): 새 3건과 기존 rail 구역 1건, 4통과입니다.
    - 표시: 접근성 toggled 상태, 40px 버튼을 확인했습니다. label `가나다라`(9px, 음수 자간, lane3 색 × 0.5 흐림)이고 첫 글자 fallback은 없습니다. rocket(활성)·기본 아이콘은 20px 중앙에 글자색으로 그립니다. 활성 배경·2x20 accent 표시는 rail clip 안에 있고 색이 없으면 기본 글자색 × 0.5입니다.
    - 배지: `이름 — agent.blocked.permission`, `이름 — agent.status.working`을 확인했습니다. 마름모는 버튼 밖까지 그리고 clip 안에 있습니다. working 원은 1초에 × 0.5이고 reduced motion이면 원색입니다. 설정을 끄면 상태와 배지가 사라집니다. unknown/idle은 1px 테두리 원입니다.
    - tooltip: hover 뒤 오른쪽에 경고 문구·root·두 세션 줄(배지 설정 꺼짐에서도)이 보입니다. root 줄은 opacity 0.7이고 경고 아이콘은 12px이며 흐림이 적용되지 않습니다. 버튼 아이콘은 흐림 0.5입니다.
- UI 전체: `--features native-host,inspection --test workbench --lib`는 lib 143/143, workbench 19/19 통과입니다. workbench의 `unused mut` 경고(1639행)는 기존 탭 메뉴 검사의 것입니다.
- App: `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib -- project_rail_tests:: problems_icons::`는 2통과(30.61초)입니다.
    - 실제 `NativeApplication`(합성 temp 데이터, ide/hook 끔)에서 label `AL`을 그리고 `native-glyph-Rocket-*` texture를 칠하는 것을 확인했습니다.
    - 실제 poll이 쓰는 `AgentStore`에 working 세션을 넣으면 접근성 이름이 `alpha — <로케일 작업 중>`이 됩니다. `agent_status_badge_enabled=false`이면 저장소에 세션이 남아 있는 프레임에서도 상태가 빠집니다.
    - 기존 glyph 검사는 새 SVG 43개를 포함한 전체 SOURCES를 크기별로 raster합니다.
- `cargo check --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection`과 `cargo check --manifest-path native/taide-native-app/Cargo.toml --all-targets`는 exit 0입니다. App check는 새 App 검사 파일을 추가하기 전에 시작했습니다. 그 파일은 이후 lib test 빌드(위 2통과)에서 컴파일됐습니다.
- 동결 remote-web `cargo check --target wasm32-unknown-unknown --lib`는 exit 0입니다. 기존 remote-web dead_code 경고만 있고 소스·Cargo.lock 변경은 없습니다.
- `cargo fmt -- --check`: native-ui는 diff가 없습니다. native-app은 소유 파일 diff가 없고, 기존 `application.rs` 902/10019행, `lib.rs`, `experiments/lsp-coordinator-spike` diff만 남아 있어 손대지 않았습니다.
- 처음부터 실패하는 상태를 재현한 뒤 고치지는 않았습니다. 새 검사는 이전 구현에 없던 동작을 단언합니다. 이전 구현은 이름 첫 글자 fallback이었고 아이콘·배지·흐림·다줄 tooltip이 없었습니다. 구현을 끄는 재현은 작업 규칙상 하지 않았습니다.

## 남은 범위

- tooltip 채널: 셸의 다른 tooltip(`+` 메뉴 등)과 같은 egui tooltip을 씁니다. 원본 Radix tooltip의 400ms 지연(`src/app/providers/app-providers.tsx` 7, 20), 키보드 focus로 열기, `tooltip.background`/`text-xs` 외형은 공통 tooltip 기반의 남은 범위입니다. 앱의 `tooltips::Provider`는 한 줄 문자열만 지원해 아이콘이 있는 다줄 내용을 담지 못합니다.
- 프로젝트 표시 편집 dialog·초기화와 context menu는 shell-36/38(다음 단계)입니다.
- 화면 픽셀, CJK 4글자 실제 글꼴 폭, 실기 확인은 배치 31입니다.
