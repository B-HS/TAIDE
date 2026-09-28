# TS view 앱 진입점·shared·widget 경계 inventory

## 범위와 판정

기존 일곱 문서에서 역할 연결이 남았던 비테스트 `.tsx` 29개를 앱 진입점 1개, shared 19개, widget 9개로 나누어 실제 소스와 테스트 경계에 연결했습니다. 여기에는 화면이 아닌 context·hook·테스트 helper도 포함됩니다. 직접 테스트와 상위 화면 테스트를 구분하며, 경로를 기록한 사실은 실제 시각·키보드·접근성·다중 창 동작의 통과 증거가 아닙니다.

## 앱 진입점과 shared 경계

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/main.tsx` | 렌더러의 bootstrap·원격 shim·workspace 연산 연결 후 `StrictMode`와 root `ErrorBoundary`로 앱을 시작합니다. 독립 화면은 아닙니다. | `src/shared/ui/error-boundary.test.tsx`는 boundary 자체 검사. 앱 최초 진입·root 오류 뒤 창 표시·복구 실기 필요. |
| `src/shared/icons/file-type-icon.tsx` | 파일명에 맞는 아이콘과 색 class를 선택합니다. | `src/shared/icons/file-icon-registry.test.tsx`는 아이콘 규칙 검사. 실제 테마별 색 대비·발화 필요. |
| `src/shared/icons/folder-type-icon.tsx` | 폴더명과 확장 여부에 맞는 아이콘·색을 선택합니다. | `src/shared/icons/file-icon-registry.test.tsx`는 아이콘 규칙 검사. 실제 접힘/펼침·테마·발화 필요. |
| `src/shared/lib/shell-slot-context.tsx` | 현재 focus project/slot과 하위 slot scope를 context로 전달합니다. 비시각 상태 경계입니다. | `src/shared/lib/shell-slot.test.ts`는 slot 모델 검사. 다중 slot·보조 창의 실제 focus 연동 필요. |
| `src/shared/scroll/overlay-scrollbar.tsx` | viewport 스크롤 위치에 맞춘 수직/수평 overlay thumb를 그립니다. | 직접 컴포넌트 테스트 미확인. 긴 목록·트랙 클릭·포인터/키보드·고대비 확인 필요. |
| `src/shared/scroll/scroll-container.tsx` | 방향별 scroll viewport와 overlay scrollbar를 조립합니다. | 직접 컴포넌트 테스트 미확인. 큰 내용·중첩 스크롤·확대 표시 필요. |
| `src/shared/testing/render.tsx` | 테스트용 QueryClient·i18n provider의 render/hook helper입니다. 제품 화면이 아닙니다. | `src/shared/testing/render.test.tsx` 직접 검사. 제품 GUI 실기 대상에서는 제외하되 테스트 환경 재사용 근거로 유지합니다. |

## Shared UI primitive

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/shared/ui/button.tsx` | button/slot 변형과 크기 class를 제공합니다. | 직접 컴포넌트 테스트 미확인. 소비 화면의 disabled·focus·키보드·대비 필요. |
| `src/shared/ui/card.tsx` | card/header/title/description/action/content/footer의 시각 구조를 제공합니다. | 직접 컴포넌트 테스트 미확인. 실제 콘텐츠·좁은 폭·읽기 순서 필요. |
| `src/shared/ui/checkbox.tsx` | Radix checkbox의 checked/disabled 상태와 indicator를 감쌉니다. | 직접 컴포넌트 테스트 미확인. 소비 폼의 label·mixed 상태·키보드 필요. |
| `src/shared/ui/command.tsx` | cmdk의 dialog/input/list/empty/group/item/shortcut 구조와 빈 목록 상태를 제공합니다. | `src/widgets/command-palette/command-palette.test.tsx`는 상위 palette 검사. 실제 검색·선택·IME·focus trap 필요. |
| `src/shared/ui/error-boundary.tsx` | 렌더 오류를 fallback·재시도 UI로 전환하고 root 창 reveal callback을 받을 수 있습니다. | `src/shared/ui/error-boundary.test.tsx` 직접 검사. 실제 앱 root 오류·복구·발화 필요. |
| `src/shared/ui/file-group-header.tsx` | 파일 그룹 count·확장 상태와 선택 toggle을 표시합니다. | 직접 컴포넌트 테스트 미확인. 그룹 선택·접힘·키보드·발화 필요. |
| `src/shared/ui/icon-button.tsx` | 접근성 이름을 가진 icon button과 설명 tooltip을 조립합니다. | 직접 컴포넌트 테스트 미확인. 소비 화면에서 focus·tooltip·disabled 상태 필요. |
| `src/shared/ui/progress.tsx` | Radix progress 상태와 indicator를 제공합니다. | 직접 컴포넌트 테스트 미확인. 실제 진행·완료 전환과 값 발화 필요. |
| `src/shared/ui/scroll-area.tsx` | Radix scroll area와 방향별 scrollbar를 감쌉니다. | 직접 컴포넌트 테스트 미확인. 긴 콘텐츠의 포인터·키보드·고대비 필요. |
| `src/shared/ui/separator.tsx` | Radix 구분선의 방향과 장식 여부를 제공합니다. | 직접 컴포넌트 테스트 미확인. 화면별 구획·읽기 순서 필요. |
| `src/shared/ui/status-error-banner.tsx` | 상태 오류 메시지와 retry 버튼을 고정 높이 배너에 표시합니다. | 직접 컴포넌트 테스트 미확인. 오류 발생·재시도·중첩 배너·발화 필요. |
| `src/shared/ui/switch.tsx` | Radix switch의 checked/disabled 상태를 제공합니다. | 직접 컴포넌트 테스트 미확인. 설정의 label·저장 오류·키보드 필요. |
| `src/shared/ui/tooltip.tsx` | Radix provider/root/trigger/content와 지연 표시를 감쌉니다. | 직접 컴포넌트 테스트 미확인. hover·focus·Escape·보조 창 portal 필요. |

## Widget 조립 경계

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/widgets/app-sidebar/sortable-project-group-header.tsx` | 그룹의 접힘·이름 변경·삭제 메뉴와 drag 재정렬 header를 조립합니다. | `src/widgets/app-sidebar/sortable-project-group-header.test.tsx` 직접 검사. 실제 pointer/keyboard DnD·메뉴 focus 필요. |
| `src/widgets/app-sidebar/sortable-project-icon.tsx` | 프로젝트 활성화·표시 편집·닫기·slot 열기·그룹/경로 메뉴와 agent 상태를 조립합니다. | `src/widgets/app-sidebar/sortable-project-icon.test.tsx` 직접 검사. 다수 프로젝트 DnD·보조 창·메뉴 focus 필요. |
| `src/widgets/editor-area/use-request-close-tab.tsx` | dirty tab과 mirror의 닫기 요청·확인/취소·정리 순서를 조정하는 비시각 hook입니다. | `src/widgets/editor-area/use-request-close-tab.test.tsx` 직접 검사. 미저장 파일의 실제 dialog·다중 창·CLI wait 연동 필요. |
| `src/widgets/editor-pane/breadcrumbs-bar.tsx` | 파일 경로·프로젝트 트리·문서 symbol breadcrumb를 조회하고 경로/심볼 이동을 제공합니다. | `src/widgets/editor-pane/breadcrumb-path.test.ts`는 경로 계산 검사. 실제 LSP symbol 변화·긴 경로·키보드 필요. |
| `src/widgets/file-history/file-history-panel.tsx` | 선택 파일의 Git 이력 dialog에서 로딩·오류·빈 목록·commit 선택을 제공합니다. | 직접 컴포넌트 테스트 미확인. 실제 Git 이력/오류·dialog focus·키보드 필요. |
| `src/widgets/git-panel/commit-graph.tsx` | commit lane·tag·선택과 revert/tag/delete 메뉴를 가상화된 그래프에 조립합니다. | `src/widgets/git-panel/git-panel.test.tsx`는 상위 panel 검사. 대량 이력·가상 스크롤·선택/메뉴 키보드 필요. |
| `src/widgets/git-panel/git-panel-container.tsx` | Git status/log/remote/branch/stash 조회와 commit draft·동작을 panel props로 조립하고 저장소 초기화 상태를 분기합니다. | `src/widgets/git-panel/git-panel-container.test.tsx` 직접 검사. 실제 저장소 오류·충돌·다중 프로젝트 필요. |
| `src/widgets/plugin-manager/plugin-install-button.tsx` | zip 필터의 native 파일 선택 뒤 plugin 설치 mutation·진행/오류 toast를 제공합니다. | 직접 컴포넌트 테스트 미확인. 승인된 시험 plugin의 선택·설치 오류·focus가 필요하며 임의 plugin 설치는 수행하지 않습니다. |
| `src/widgets/welcome/welcome-container.tsx` | 최근/열린 프로젝트·settings·layout 조회, 폴더/파일/terminal 열기와 최근 프로젝트 활성화를 welcome props로 조립합니다. | `src/features/welcome/welcome-screen.test.tsx`는 표시 컴포넌트 검사. 빈 세션·최근 목록 오류·root 밖 파일 거부·키보드 필요. |

이 문서의 29개 경로를 더하면 여덟 문서의 직접 연결 경로는 중복 제외 212/212개입니다. 경로 모집단 연결만 완료했으며 각 화면의 빈/오류/진행 상태, 실제 시각·접근성·다중 창 동등성은 계속 미완료입니다.
