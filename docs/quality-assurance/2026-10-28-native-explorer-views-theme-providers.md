# 탐색기 보기 전환·테마/로케일 오류 배너·공통 tooltip 채널

## 대상

- shell-59: explorer 내부 4개 보기(files/search/git/outline) 전환
- shell-94: 테마·로케일 적용과 로드 오류 배너/재시도
- shell-98: 앱 provider(QueryClient·Tooltip·toast 계층)
- 추가: 탭·rail drag 해제 프레임 경계(2026-10-27-native-tab-drag.md 리뷰 비차단 항목)

## 원본 동작

### shell-59 보기 전환

- `src/widgets/explorer/explorer-panel.tsx`
    - 24-31: 보기 순서는 files(`explorer.title`, FolderTree)·search(`search.title`, Search)·git(`git.title`, GitBranch)·outline(`outline.title`, ListTree)입니다.
    - 177-200: `role=tablist`, `aria-label=explorer.sidebarSwitchLabel`, 높이 36px, 아래선 `tabBar.tabBorder`, 좌우 8px·간격 4px입니다. 각 보기는 `role=tab`·`aria-selected`인 24px `IconButton`(16px 아이콘, tooltip 아래쪽)입니다. 선택 보기는 `explorer.itemSelected` 바탕과 foreground, 나머지는 `appSidebar.iconDefault`와 hover 바탕입니다.
    - 202-260: files만 머리줄과 트리를 그리고 search·git·outline은 각 패널 본체를 그립니다.
    - 131-173: 키·명령 연결은 4가지입니다. `subscribeOpenSearchPanel`(search, 검색 요청 포함), `subscribeShowExplorerView`(files/git), reveal·rename(files 전환 후 요청)입니다. 모두 포커스 slot에서만 반응합니다.
- `src/widgets/explorer/explorer-container.tsx` 58: 보기 상태는 패널 mount마다 `useState('files')`입니다.
- `src/shared/ui/icon-button.tsx`: 버튼을 Radix Tooltip으로 감쌉니다. 기본 방향은 bottom입니다.
- 브라우저 버튼은 pointerdown에서 focus를 받습니다. Radix Tooltip은 pointer로 생긴 focus로는 열리지 않습니다.

### shell-94 테마·로케일 오류 배너

- `src/app/providers/theme-provider.tsx`
    - 23-24: 테마와 로케일 query의 `isError`를 읽습니다.
    - 100-112: 테마 실패면 `theme.loadFailed`와 `common.retry` 배너를 보입니다. 재시도는 테마 query `refetch`입니다. 로케일도 실패했으면 32px 아래에 둡니다(`stackOffsetPx`).
- `src/app/providers/locale-provider.tsx`
    - 8: 로케일 실패 문구는 번역 카탈로그가 아니라 고정 영어 문구입니다. 언어 팩 자체가 실패했기 때문입니다.
    - 25-32: 로케일 실패 배너는 맨 위(0)이고 재시도는 로케일 `refetch`입니다.
- `src/shared/ui/status-error-banner.tsx` 9, 18-28
    - `fixed inset-x-0 z-50`, `px-3 py-1.5 text-xs`, 간격 8px입니다.
    - 바탕은 `status-error`의 15%, 글자색은 `status-error`입니다(`global.css` 241: `statusIndicator.error`).
    - 14px `AlertTriangle`, 나머지 폭을 차지하는 문구, 밑줄 재시도 버튼 순서입니다. 쌓임 간격 상수는 32px입니다.
- react-query는 다시 불러오기에 실패하면 이전 데이터를 유지한 채 `isError`가 됩니다. 그래서 적용된 테마·언어는 그대로이고 배너만 보입니다.
- 두 provider는 주 창과 보조 창 모두에 mount됩니다(`src/app/app.tsx` 72-73, 100-101).

### shell-98 provider 계층

- `src/app/providers/app-providers.tsx` 7, 15-26: `QueryClientProvider` 안에 `TooltipProvider(delayDuration=400)`가 있고, 그 안에 화면과 `AppToaster`가 있습니다.
- `src/shared/ui/tooltip.tsx` 20-36
    - Portal, `rounded-md`, 1px `tooltip.border`, `tooltip.background`, `px-3 py-1.5 text-xs`, `app.foreground`, 화살표입니다.
    - 열고 닫을 때 fade·zoom 효과가 있습니다.
    - Radix는 지연 뒤 hover로 열고, 키보드 focus로는 바로 엽니다.
- 셸의 tooltip은 모두 이 provider를 거칩니다.
    - rail 프로젝트: `sortable-project-icon.tsx` 131-165, 오른쪽. 이름, root 누락 줄(12px `TriangleAlert` + gap 4px), opacity 70의 root, 에이전트 세션 줄을 세로로 쌓습니다.
    - rail `+`: `sidebar-add-project-menu.tsx` 41-48, 오른쪽
    - slot 헤더 경고: `shell-slot-header.tsx` 38-45, 아래쪽
    - slot 헤더 닫기: `IconButton`, 아래쪽
    - 탭바 `+`: `tab-bar-add-menu.tsx` 26-33, 아래쪽
    - 탭 닫기·고정 해제: `tab-item.tsx` 67-85, 아래쪽
    - 탭 에이전트: `tab-item.tsx` 58-63, 아래쪽
- `AppToaster`(sonner)는 `z-index: 999999999`(`sonner/dist/styles.css` 50)입니다. tooltip(`z-50`)보다 항상 위입니다.
- QueryClient는 웹 서버 상태 캐시입니다. native는 Host 요청·runtime 상태가 그 역할을 맡아 해당 표면이 없습니다.

### drag 해제 프레임

- `@dnd-kit/core`는 Escape를 누르면 drag를 취소합니다. 놓는 순간의 `over`는 그때의 충돌 판정입니다. 대상이 없으면 `over=null`이라 아무 데도 놓지 않습니다.

## 구현

### UI(`taide-native-ui`)

- `project-drag.rs`
    - `PointerInput::cancels`로 Escape·창 크기 변경을 판정하고, `advance`와 해제 프레임이 함께 씁니다.
    - 해제 프레임에서는 취소를 먼저 판정합니다.
    - 포인터 위치가 있으면 그 위치의 대상만 씁니다. 대상이 없으면 None입니다. 직전 대상은 포인터 위치가 아예 없을 때만 씁니다.
- `tab-drag.rs`: 같은 규칙을 적용했습니다.
- `tooltips.rs`(공통 tooltip owner)
    - `Line { icon, text, opacity }`와 `Icon { size, shape }`를 추가했습니다. `Provider::show_lines`가 여러 줄을 받습니다.
    - 줄 사이 간격은 0입니다. 아이콘 줄은 아이콘과 문구 사이 4px입니다. 문구는 foreground에 줄별 opacity를 적용합니다.
    - 접근성 tooltip 이름은 줄을 `\n`으로 이은 값이고, 줄마다 bounds를 둡니다.
    - 기존 `show`·`show_triggers`·`show_controlled`는 한 줄짜리로 같은 경로를 씁니다.
    - `Appearance::foreground`를 공개했습니다(아이콘 색).
- `shell.rs`
    - `TooltipLine`과 `ShellSurfaces::tooltip(response, align, lines)`를 추가했습니다.
    - 기본 구현 `egui_tooltip`은 이전과 같은 egui tooltip입니다. 아이콘은 `rail_icon`으로 그립니다. 그래서 remote-web은 동작이 그대로입니다.
    - 셸 tooltip 7곳을 모두 이 hook으로 보내고, 원본 방향을 매 프레임 넘깁니다: rail 프로젝트(오른쪽, 여러 줄과 경고 아이콘), rail `+`(오른쪽), slot 헤더 경고·닫기(아래), 탭바 `+`(아래), 탭 닫기·고정 해제(아래), 탭 에이전트(아래).
    - rail은 읽기 전용 오류 경계 안이라 `RailOutput.tooltips`로 미뤘다가 경계가 그려진 뒤 호출합니다.
    - 상수는 셸 공통 이름(`TOOLTIP_ICON_SIZE`/`GAP`/`SECONDARY_OPACITY`)으로 바꿨습니다.
- `toast.rs`: 알림 영역 층을 매 프레임 `Context::move_to_top`으로 올립니다. 같은 `Order::Tooltip` 안에서 나중에 처음 열린 tooltip이 toast를 덮던 순서를 원본 z-index 순서로 바로잡았습니다.

### App(`taide-native-app`)

- `symbol-sidebar.rs`
    - `View`를 Files·Search·Git·Outline 4개로 넓혔습니다. 아이콘은 `search.svg`와 기존 GitBranch glyph입니다.
    - 머리줄은 `explorer.sidebarSwitchLabel`의 TabList 접근성 노드이고 bounds를 둡니다.
    - 버튼은 누르는 프레임에 focus를 받습니다. 원본 브라우저의 pointerdown focus와 같습니다. 이전에는 놓은 다음 프레임에 focus가 생겨 공통 tooltip이 키보드 focus로 보고 클릭 직후 tooltip을 열었습니다.
- `navigation-icons.rs`: `Icon::Search`(`resources/icons/search.svg`)와 `Icon::GitBranch`(기존 glyph)를 추가했습니다.
- `application.rs`
    - 사이드바 렌더는 Search·Git이면 전환 줄만 그리고 본문은 비웁니다. 임시 안내는 없습니다.
    - 보기 상태는 기존 slot별 `symbol_sidebar::Views`에 있습니다.
- `AppSurfaces::tooltip`
    - 셸 tooltip 줄을 `tooltips::Line`으로 바꾸고 `Provider::show_lines`로 보냅니다.
    - 아이콘 줄은 앱 glyph cache로 tooltip foreground 색 12px 도형을 만들어 넘깁니다.
    - 그래서 400ms 지연, 0.3초 건너뛰기, 키보드 focus 즉시 열기, tooltip.background/border·12px 외형, 화살표·동작, 접근성이 셸에도 적용됩니다.
- `status-error-banner.rs`(새 모듈)
    - 로케일 배너(맨 위)와 테마 배너(로케일 배너가 있으면 32px 아래)를 창 너비 `Order::Middle` 영역으로 그립니다.
    - 원본 수치를 씁니다: 좌우 12px·상하 6px, 14px `TriangleAlert`, 간격 8px, 12px/16px 글자, `statusIndicator.error` 15% 바탕과 글자색, 밑줄 `common.retry` 버튼(접근성 Button).
    - 모달·팔레트로 셸이 비활성일 때는 같은 조건으로 비활성입니다.
- `presentation-refresh.rs`: `Refresh::retry`를 추가했습니다. 진행 중 요청이 없으면 같은 입력도 다시 요청하게 합니다.
- `application.rs`
    - `presentation_failures`를 추가했습니다.
    - Host의 테마·로케일 결과에 따라 실패 표시를 켜고 끕니다. 이전에는 status 라벨에 오류 문자열을 넣었습니다. 상세는 `log::warn`으로 남깁니다.
    - 실패해도 적용된 테마·로케일은 유지합니다.
    - 모든 창(`show_window`)에 배너를 그리고, 재시도는 refresh를 다시 요청합니다.

### 판단

- 전역 키 `git`(⌃⇧G)·`view.git`과 `search`(⌘⇧F)·`search-replace`·`find` 대체는 이번에 연결하지 않았습니다.
    - 기능표에서 각각 git-1·commands-ui-14·shell-21(배치 26)과 explorer-search-37·commands-ui-11(배치 25)에 배정돼 있습니다.
    - 검색 쪽은 seed·replace·scope·nonce 같은 검색 패널 요청 계약을 함께 넘깁니다.
    - 이번 단계는 보기 전환 대상(`View::Search`/`View::Git`)과 전환 UI만 만들었습니다. 같은 단계 앞선 작업의 "git 키 통과" 검사도 그대로 유지됩니다.
- 로케일 실패 문구는 카탈로그 키가 아니라 원본의 고정 영어 상수를 그대로 씁니다(`locale-provider.tsx` 8).
- 배너 높이는 CSS 계산대로 28px(상하 6px + 16px 줄)입니다. 쌓임 간격은 원본 상수 32px입니다.
- 한 번의 refresh가 테마와 로케일을 함께 다시 읽습니다. 그래서 한쪽 재시도도 같은 요청입니다. 결과는 각자 반영합니다.
- 테마 적용 단계(Appearances·터미널 팔레트) 실패도 테마 실패로 보고 배너에 넣었습니다.
- welcome 최근 행과 시스템 사용량 상세의 tooltip은 원본이 HTML `title`(브라우저 기본 tooltip)이라 egui 기본 hover를 유지했습니다. 편집기 내부 hover는 Monaco 대응 표면이라 이 채널에 넣지 않았습니다.

### 기존 검사 조정

- `symbol-sidebar-tests.rs`의 기존 클릭 검사는 버튼이 4개가 되어 아웃라인 index를 3으로 바꿨습니다. 단언 내용(선택색·focus·Outline 선택)은 같습니다.

## 검증

- 실패 재현 후 수정
    - drag 해제: UI `cargo test --features native-host,inspection --lib -- 해제_프레임` 2건 실패를 재현했습니다. 대상 밖 해제가 직전 대상(MoveTab/OpenInSlot Right)으로 drop됐습니다. 수정 뒤 drag 단위 14/14 통과입니다.
    - toast 층: App 실제 tooltip 검사가 "the toast layer paints over the tooltip"으로 실패했습니다. `move_to_top` 뒤 통과입니다.
    - 사이드바 클릭 tooltip: App 실제 보기 전환 검사가 Git 클릭 직후 본문에 "Git" tooltip이 보여 실패했습니다. 누름 프레임 focus로 바꾼 뒤 통과입니다.
    - TabList 접근성 bounds 누락도 같은 검사로 드러나 수정했습니다.
- 배너·tooltip 줄 채널·4개 보기 단위 검사는 구현과 함께 작성했습니다. 구현을 꺼서 재현하는 일은 작업 규칙상 하지 않았습니다.
- UI(`taide-native-ui`)
    - `cargo test --features native-host,inspection --lib`: 167/167 통과
    - `cargo test --features native-host,inspection --test workbench`: 45/45 통과. 새 검사 `셸_tooltip은_모두_공통_채널에_원본_방향과_줄을_매_프레임_넘긴다`가 7곳의 방향·줄(아이콘·opacity)·트리거 크기를 확인합니다. 기존 rail·slot 헤더·탭 에이전트 tooltip 검사도 기본 구현으로 통과합니다.
    - `cargo check --features native-host,inspection --all-targets`·기능 없는 `cargo check --lib` exit 0. 기능 없는 `--all-targets`는 workbench의 기존 `inject_panic`(inspection 전용) 사용으로 실패하며 이번 변경과 무관합니다.
    - `cargo fmt -- --check` exit 0
- App(`taide-native-app`)
    - 최종 `cargo test --lib -- explorer_views_tests symbol_sidebar presentation_refresh`: 13/13 통과(새 실제 App 3, sidebar 단위, refresh 단위 포함)
    - `실제_app_사이드바는_네_보기를_slot별로_전환하고_검색과_git은_빈_본문만_보인다`
        - 2 slot에서 왼쪽 Git·Search를 클릭하면 파일 행이 사라집니다. 탭은 선택 상태이고, 본문에 글자가 없습니다(tooltip·안내 없음).
        - 오른쪽 slot은 Files를 유지하고, Explorer로 돌아오면 행이 다시 보입니다.
    - `실제_app은_테마와_로케일_로드_실패를_상단_배너로_쌓고_재시도로_다시_불러온다`
        - 합성 테마·로케일 파일을 깨뜨리면 로케일 배너(y 0)와 테마 배너(y 32)가 창 너비로 보입니다. 바탕은 error 15%, 글자는 error 12px, 재시도는 밑줄 Button 2개입니다.
        - status 라벨에는 오류가 없습니다.
        - 파일을 고쳐도 재시도 전에는 그대로입니다.
        - 테마 재시도 뒤 테마 배너만 사라지고, 로케일 재시도 뒤 locale id `xx`가 적용됩니다.
    - `실제_app_rail_tooltip은_공통_채널로_400ms_지연과_focus로_열고_다줄_아이콘_외형과_toast_아래_층을_지킨다`
        - 0.3초에는 닫혀 있고 0.45초에 열립니다. 오른쪽에 붙습니다.
        - tooltip.background 바탕과 tooltip.border 테두리, 12px 글자, root 줄 opacity 0.7입니다.
        - 12px 경고 glyph가 경고 문구 앞에 있습니다. toast가 tooltip보다 나중에(위에) 그려집니다.
        - 포인터가 떠난 뒤 AccessKit focus 요청에는 지연 없이 열립니다.
    - 영향 회귀: `cargo test --lib -- tooltip toast project_rail tab_menu tab_drag shell_dialogs error_boundary auxiliary_window presentation_refresh explorer_views symbol_sidebar file_drop zen`
        - 117건 중 116 통과, 1 실패입니다. 실패는 `file_drop_tests::실제_app은_놓은_폴더를...`의 단계 시간 초과(`phase=folder then file`)로, 동시 실행 부하 때문입니다.
        - 같은 조건으로 `-- file_drop_tests`만 다시 실행하면 3/3 통과입니다(알려진 시간 초과 계열).
    - 아이콘 추가 영향: `cargo test --lib -- navigation symbol_outline explorer_tooltip` 11/11 통과
    - `cargo check --all-targets` exit 0
    - `cargo fmt -- --check`
        - 이번 변경 파일의 차이는 없습니다.
        - 남은 차이는 모두 기존 것입니다: `application.rs` 964 들여쓰기·테스트 모듈 순서, `lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike`.
- remote-web: `cargo check --target wasm32-unknown-unknown --lib` exit 0
- 앱 전체 lib 검사는 실행하지 않았습니다(배치 끝 메인 담당).

## 남은 범위

- shell-59
    - search·git 보기의 본체(검색 패널: 배치 25, Git 패널: 배치 26)
    - 그 보기로 가는 전역 키·명령: ⌘⇧F/⌘⇧H/`find` 대체(explorer-search-37·commands-ui-11), ⌃⇧G/`view.git`/`git.createTagOnHead`(git-1·commands-ui-14·shell-21)
- shell-94
    - 시작 시 테마·로케일 로드 실패는 여전히 앱 시작 오류입니다. 원본은 기본 CSS 변수와 배너로 계속 뜹니다. native에서 이를 재현하려면 시작용 대체 테마·빈 로케일을 정해야 합니다.
    - OS 파일을 끌어오는 동안의 drop overlay(`Order::Middle` 비영역 층)는 배너(`Order::Middle` 영역) 위에 그려집니다. 원본은 배너가 위입니다. 겹치는 동안만의 차이입니다.
- shell-98
    - 보조 창 자체 toast: 원본은 창마다 `AppToaster`가 있어 그 창에 toast를 띄웁니다(`src/app/app.tsx` 67-87). native toast는 메인 창 `finish_frame`에서만 그립니다.
    - 화면 픽셀·실제 tooltip 동작·지연의 실기 확인은 배치 31입니다.
- drag 해제 프레임: 포인터 위치가 아예 없는 해제(창 밖에서 위치를 잃은 경우)는 직전 대상을 씁니다. dnd-kit도 마지막 이동의 `over`를 씁니다.
