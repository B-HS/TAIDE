# TS view 하위 feature 경계 inventory C

## 범위와 판정

Project 4개, shell slot 1개, snippet 2개, split 2개, tab 2개, theme 5개, welcome 1개, window 5개의 비테스트 `.tsx` 경로를 소스의 상태·조작에 연결했습니다. 직접 테스트와 상위/유틸리티 테스트를 구분하며 실제 다중 창·drag/drop·테마·접근성 결과는 별도입니다.

## Project·shell slot·snippet

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/project/agent-status-badge.tsx` | agent 활동 상태를 작은 상태 표시로 렌더합니다. | 직접 컴포넌트 테스트 미확인. 작업·입력 대기 전환의 색 대비와 발화 필요. |
| `src/features/project/project-display-glyph.tsx` | 프로젝트 표시 설정에 따라 문자·아이콘 glyph를 선택해 그립니다. | `src/features/project/project-display-dialog.test.tsx`는 편집 dialog 검사. 테마별 glyph 크기·발화 필요. |
| `src/features/project/project-drag-preview.tsx` | 프로젝트 재정렬 중 표시 glyph를 drag preview에 제공합니다. | 직접 컴포넌트 테스트 미확인. 실제 pointer DnD·고대비·창 경계 필요. |
| `src/features/project/project-icon-button.tsx` | 프로젝트 활성화 버튼의 선택·상태 표시와 설명을 제공합니다. | `src/widgets/app-sidebar/app-sidebar.test.tsx`는 상위 rail 검사. 다수 프로젝트·키보드 활성화·tooltip 필요. |
| `src/features/shell-slot/shell-slot-header.tsx` | shell slot 이름·root 누락 경고·닫기 가능 상태와 close 버튼을 표시합니다. | `src/features/shell-slot/shell-slot-header.test.tsx` 직접 검사. split 상태·root 삭제·보조 기술 필요. |
| `src/features/snippet/snippet-entry-editor.tsx` | snippet draft의 입력 필드·scope·삭제 요청을 렌더합니다. | `src/shared/lib/snippet-draft.test.ts`는 draft 계산 검사. 편집 오류·IME·삭제 focus 필요. |
| `src/features/snippet/snippet-file-list.tsx` | 언어별 snippet 파일 목록과 현재 선택·파일 전환을 표시합니다. | 직접 컴포넌트 테스트 미확인. 빈/다수 언어·미저장 전환·키보드 필요. |

## Split·tab

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/split/pane-separator.tsx` | 가로/세로 panel resize 경계와 숨김 상태를 제공합니다. | `src/widgets/app-shell/shell-slot-tree-view.test.tsx`는 상위 slot tree 검사. 실제 drag·키보드 resize·Zen 모드 필요. |
| `src/features/split/split-drop-zones.tsx` | left/right/top/bottom/center drop target과 활성 edge preview를 렌더합니다. | 직접 컴포넌트 테스트 미확인. 실제 탭 drag·다중 창·drop 취소 필요. |
| `src/features/tab/sortable-tab.tsx` | tab의 drag 정렬·활성·닫기 이벤트를 내부 tab item과 연결합니다. | `src/features/tab/tab-bar-context-menu.test.tsx`는 메뉴 경계. pointer/keyboard DnD·고정 tab 필요. |
| `src/features/tab/tab-item.tsx` | tab title·icon·dirty·pinned·preview·agent 안내와 활성/닫기/고정 조작을 표시합니다. | 직접 컴포넌트 테스트 미확인. 긴 제목·미저장·스크린리더 tab 상태 필요. |

## Theme·welcome·window

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/theme/color-picker.tsx` | 색상 선택의 picker·slider·hex 입력과 잘못된 hex 상태를 관리합니다. | `src/features/theme/color-picker-drag.test.ts`는 drag 계산 검사. 실제 포인터·키보드 slider·색 대비 필요. |
| `src/features/theme/color-token-row.tsx` | theme 색 token의 현재 값·변경 표시·초기화와 color picker를 연결합니다. | `src/shared/lib/theme-draft.test.ts`는 draft 계산 검사. token 변경/초기화·발화 필요. |
| `src/features/theme/custom-theme-list.tsx` | 사용자 theme 목록과 edit/duplicate 행동을 표시합니다. | 직접 컴포넌트 테스트 미확인. 빈 목록·중복 이름·키보드 필요. |
| `src/features/theme/syntax-token-row.tsx` | syntax 색·bold/italic·변경·초기화 조작을 표시합니다. | `src/shared/lib/theme-draft.test.ts`는 draft 계산 검사. 실제 editor syntax 반영·대비 필요. |
| `src/features/theme/theme-live-preview.tsx` | 편집 중 theme token·ANSI 색의 샘플을 즉시 렌더합니다. | 직접 컴포넌트 테스트 미확인. 라이트/다크 전환·실제 editor와 시각 비교 필요. |
| `src/features/welcome/welcome-screen.tsx` | 빈 세션에서 폴더·파일·terminal 열기와 최근 프로젝트/root 누락 상태를 표시합니다. | `src/features/welcome/welcome-screen.test.tsx` 직접 검사. cold start·누락 root·키보드/발화 필요. |
| `src/features/window/drag-drop-overlay.tsx` | OS 파일/폴더 drop 중 안내 label·overlay를 표시합니다. | 직접 컴포넌트 테스트 미확인. 실제 Finder drag·잘못된 대상·발화 필요. |
| `src/features/window/font-size-stepper.tsx` | editor/terminal 글꼴 크기 감소·초기화·증가 버튼에 접근성 이름을 제공합니다. | 직접 컴포넌트 테스트 미확인. 최소/최대 크기·키보드·보조 창 동기화 필요. |
| `src/features/window/status-bar.tsx` | problems·LSP·사용량·글꼴 조절 등 창 하단 상태와 동작을 props로 표시합니다. | `src/widgets/window-chrome/status-bar-content.test.tsx`는 상위 조립 검사. 실제 상태 변동·좁은 폭·발화 필요. |
| `src/features/window/title-bar.tsx` | 활성 tab·프로젝트·branch의 창 제목을 표시합니다. | 직접 컴포넌트 테스트 미확인. native title과 동기화·긴 이름·보조 창 필요. |
| `src/features/window/zen-mode-hint.tsx` | Zen 모드 진입 시 일시적 해제 안내를 표시합니다. | 직접 컴포넌트 테스트 미확인. 안내 지속 시간·키보드 탈출·발화 필요. |

이 문서의 22개 경로를 더하면 일곱 문서의 중복 제외 직접 연결은 183/212개, 남은 29개는 앱 진입점과 shared·widgets 계층입니다. 경로 coverage는 화면/접근성 parity가 아닙니다.
