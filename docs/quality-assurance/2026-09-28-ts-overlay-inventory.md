# TS 대화상자·메뉴 inventory 진행 기록

## 범위

`ts-view-components-v1.json`의 212개 경로 중 이름이 `dialog`·`menu`·`popover`·`modal`로 끝나는 비테스트 `.tsx` 26개를 대상으로 했습니다. 앱별 항목 21개와 공용 Radix wrapper 5개를 구분합니다. 이름이 다른 `command-palette.tsx`·`keybindings-editor.tsx` 등도 dialog를 그리므로 이 목록만으로 전체 overlay가 완성되지 않습니다. 아래의 자동 검사 열은 실제 파일이 있는 직접 테스트만 적었으며, 테스트가 없다고 제품 동작 결함을 단정하지 않습니다.

## 앱별 대화상자·메뉴

| 경로 | 현재 표시·상태·동작 경계 | 직접 자동 검사 | 남은 실기 |
| --- | --- | --- | --- |
| `src/features/explorer/entry-delete-dialog.tsx` | entryName이 있을 때 삭제 확인, 취소·파괴적 확인 | 없음 | 파일/폴더 대상명 발화, focus/취소 |
| `src/features/explorer/file-tree-context-menu.tsx` | 파일/폴더별 새 항목·열기 방식·비교·복사/붙여넣기·rename/delete, 조건별 비활성 | 없음 | 마우스·키보드 메뉴 진입, 하위 메뉴·focus |
| `src/features/git/conflict-compare-dialog.tsx` | sides가 있을 때 비교 화면, 닫기 | 없음 | 큰 diff·화면 크기·닫은 뒤 focus |
| `src/features/git/conflict-resolution-dialog.tsx` | 충돌 해결: compare/current/incoming/both, 취소 | 없음 | 선택 결과·파괴적 동작 전후 발화 |
| `src/features/git/create-tag-dialog.tsx` | tag 이름 입력, 공백/진행 중 확인 차단, 취소 | 없음 | Enter·IME·오류·focus 복귀 |
| `src/features/git/hunk-discard-dialog.tsx` | hunk 범위 폐기 확인, 취소·확인 | 없음 | 범위 설명·파괴적 버튼 구분 |
| `src/features/project/open-project-by-path-dialog.tsx` | 경로 입력, 빈 값/진행 중 확인 차단, Enter·취소 | `src/features/project/open-project-by-path-dialog.test.tsx` | OS 경로·IME·오류·focus |
| `src/features/project/project-display-dialog.tsx` | 프로젝트 icon·색·이름 선택, 저장 가능 상태·취소 | `src/features/project/project-display-dialog.test.tsx` | radio/색 토큰 발화·키보드·시각 대비 |
| `src/features/project/project-group-dialog.tsx` | 그룹 create/edit, 이름·색 선택, Enter·진행 중 차단·취소 | `src/features/project/project-group-dialog.test.tsx` | 색 그룹 키보드·오류·focus |
| `src/features/project/sidebar-add-project-menu.tsx` | 경로/Finder/최근 프로젝트 진입, 최근 목록 0개·존재 분기 | `src/features/project/sidebar-add-project-menu.test.tsx` | 메뉴 순서·키보드·발화 |
| `src/features/settings/sync-conflict-dialog.tsx` | 동기화 충돌에서 로컬 유지/원격 당김, 닫기 취소 | 없음 | 실제 충돌 데이터·선택 위험·focus |
| `src/features/tab/close-dirty-tab-dialog.tsx` | dirty tab이 있을 때 저장/버리기/취소 | 없음 | 다중 dirty 제목·기본 focus·키보드 |
| `src/features/tab/tab-bar-add-menu.tsx` | 새 파일/새 terminal tab 선택 | 없음 | 추가 버튼 라벨·키보드 |
| `src/features/tab/tab-bar-context-menu.tsx` | tab bar의 빈 공간에서 close/split 등 action 그룹·조건부 항목 | `src/features/tab/tab-bar-context-menu.test.tsx` | tab 0개/다중 pane 항목·하위 메뉴·focus |
| `src/features/tab/tab-context-menu.tsx` | tab close/pin/경로·open-with/split/보조 창 이동, 종류별 조건부 항목 | 없음 | pin·dirty·preview·보조 창별 항목·focus |
| `src/features/terminal/terminal-context-menu.tsx` | copy/paste/select-all/clear/split/new/kill, 가용성별 비활성 | `src/features/terminal/terminal-context-menu.test.tsx` | PTY focus 복귀·선택·clipboard 권한 |
| `src/widgets/plugin-manager/plugin-uninstall-dialog.tsx` | pending plugin 제거 확인, 진행 중 버튼 차단 | 없음 | 설치 상태·파괴적 버튼 발화·focus |
| `src/widgets/plugin-manager/vsix-import-dialog.tsx` | VSIX 결과/grammar/theme 저장, 기존 theme 덮어쓰기 중첩 확인 | 없음 | 실제 VSIX·중첩 dialog focus·오류 |
| `src/widgets/snippet-editor/new-snippet-file-dialog.tsx` | global/language 선택, 이름 검증·중복 차단·생성/취소 | 없음 | 언어 목록·IME·오류·focus |
| `src/widgets/system-usage-modal/system-usage-modal.tsx` | 시스템 사용량 상세 표시·닫기 | 없음 | 갱신 중 화면·스크린리더·focus |
| `src/widgets/task-runner/task-runner-dialog.tsx` | 프로젝트별 task query/list·선택 실행·닫기 | `src/widgets/task-runner/task-runner-dialog.test.tsx` | 메인/보조 창 target·IME·키보드 |

## 공용 overlay 경계

| 경로 | 현재 책임 | 남은 판정 |
| --- | --- | --- |
| `src/shared/ui/dialog.tsx` | Radix Dialog wrapper, IME 조합 중 Escape 닫기 억제, 제목/설명/닫기 버튼 | 모든 소비자의 실제 focus trap·Escape·발화·IME |
| `src/shared/ui/alert-dialog.tsx` | Radix AlertDialog wrapper, 확인/취소/설명·크기 변형 | 파괴적 작업의 기본 focus·스크린리더 |
| `src/shared/ui/context-menu.tsx` | Radix ContextMenu wrapper, item/submenu/선택/비활성 스타일 | 마우스·키보드 메뉴 진입/닫기·focus |
| `src/shared/ui/dropdown-menu.tsx` | Radix DropdownMenu wrapper, item/submenu/선택/비활성 스타일 | trigger·키보드·포커스 반환 |
| `src/shared/ui/popover.tsx` | Radix Popover wrapper | 실제 소비자별 열림/닫힘·focus |

## 판정

26개 경로의 역할과 앱별 기본 열림·선택 경계를 소스에 연결했습니다. 앱별 21개 중 직접 테스트 파일이 확인된 것은 7개입니다. 자동 테스트의 assertion 범위는 이 표만으로 증명하지 않으며, 실제 키보드·IME·VoiceOver·시각·다중 창 결과는 전부 미검증입니다. 이 파일을 TS view 전수 완료나 접근성 통과로 계산하지 않습니다.
