# TS view 하위 feature 경계 inventory A

## 범위와 판정

비테스트 `.tsx` 모집단 212개 중 command palette 6개, editor 5개, explorer 2개, Git 8개의 직접 연결을 추가합니다. 각 행은 해당 경로가 맡는 상태·조작을 소스에서 확인한 범위이며 테스트 이름은 기존 자동 근거입니다. 이번 단계에서 GUI·키보드·VoiceOver·시각 검사를 새로 통과시킨 것은 아닙니다.

## Command palette

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/command-palette/command-palette-commands-group.tsx` | 명령별 실행 가능 여부·키바인딩을 표시하고 선택을 상위 실행 handler에 전달합니다. | `src/widgets/command-palette/command-palette.test.tsx`는 상위 palette 경계입니다. 비활성 명령·키보드 선택 실기 필요. |
| `src/features/command-palette/command-palette-files-group.tsx` | fuzzy 파일명·상위 경로를 강조하고, 새 검색 진행 중에도 기존 행을 열 수 있게 표시합니다. | `src/features/command-palette/command-palette-files-group.test.tsx` 직접 검사. 대규모 검색·삭제 직후 stale 행·IME 실기 필요. |
| `src/features/command-palette/command-palette-line-group.tsx` | 현재 파일·줄 이동 대상 행을 표시하고 선택을 상위 이동 handler에 전달합니다. | 직접 컴포넌트 테스트 미확인. 잘못된 줄 입력·키보드 선택 실기 필요. |
| `src/features/command-palette/command-palette-symbol-group.tsx` | 현재 문서 symbol 행과 위치를 표시하고 선택을 전달합니다. | 직접 컴포넌트 테스트 미확인. LSP 결과 갱신·동명이인·키보드 선택 실기 필요. |
| `src/features/command-palette/command-palette-workspace-symbol-group.tsx` | 워크스페이스 symbol의 파일·위치를 검색어와 함께 표시하고 선택을 전달합니다. | 직접 컴포넌트 테스트 미확인. 다중 파일·늦은 검색 결과 실기 필요. |
| `src/features/command-palette/highlighted-text.tsx` | fuzzy 일치 index를 텍스트 조각의 강조 span으로 렌더합니다. | 직접 컴포넌트 테스트 미확인. 합성 문자·고대비·스크린리더 읽기 순서 실기 필요. |

## Editor

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/editor/blame-footer-bar.tsx` | 외부에서 채우는 blame 문구를 editor 하단에 한 줄로 표시합니다. | 직접 컴포넌트 테스트 미확인. 긴 작성자·좁은 폭·발화 실기 필요. |
| `src/features/editor/breadcrumb-segment.tsx` | 경로 segment의 강조·구분자를 표시하고, 선택 가능한 항목은 dropdown으로 엽니다. | 직접 컴포넌트 테스트 미확인. 키보드·focus·긴 경로 실기 필요. |
| `src/features/editor/code-editor.tsx` | Monaco model attach·값 변경·저장·커서·설정·AI inline completion과 에디터 focus를 props 경계로 조립합니다. | `src/features/editor/code-editor.test.tsx` 직접 검사. 실제 IME·다중 split focus·undo·large file 실기 필요. |
| `src/features/editor/conflict-banner.tsx` | disk 변경·mirror 복원 충돌 variant에 따라 디스크 보기·내 초안 유지·닫기 조작을 보여줍니다. | 직접 컴포넌트 테스트 미확인. 실제 동시 외부 변경·읽기 순서·안내 발화 실기 필요. |
| `src/features/editor/markdown-preview.tsx` | 상위 계층에서 받은 HTML을 Markdown 스타일과 양축 스크롤로 표시합니다. | 직접 컴포넌트 테스트 미확인. HTML 생성/정화 경계·이미지·링크·키보드 스크롤 실기 필요. |

## Explorer

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/explorer/file-tree-draft-row.tsx` | 새 파일/폴더·이름 변경 입력에서 IME 조합을 구분하고 Enter 확정, Escape/blur 취소, 오류 tooltip을 표시합니다. | `src/features/explorer/file-tree.test.tsx`는 상위 트리 검사. IME·중복 이름·오류 tooltip focus 실기 필요. |
| `src/features/explorer/file-tree-toolbar.tsx` | 새 파일·새 폴더·새로고침·모두 접기 버튼과 접근성 이름을 제공합니다. | `src/features/explorer/file-tree.test.tsx`는 상위 트리 검사. hover 외 키보드 focus에서 버튼 노출·실제 파일 반영 확인 필요. |

## Git

| 경로 | 상태·동작 | 자동 근거와 남은 실기 |
| --- | --- | --- |
| `src/features/git/branch-group.tsx` | 로컬·원격 branch 목록 행을 검색 필터와 함께 구성하고 선택을 전달합니다. | 직접 컴포넌트 테스트 미확인. 동명 branch·키보드 목록 실기 필요. |
| `src/features/git/branch-switcher.tsx` | branch popover의 현재 branch·검색·로컬/원격 checkout·새 branch 만들기 선택을 관리합니다. | 직접 컴포넌트 테스트 미확인. checkout 오류·열림 focus·원격 branch 실기 필요. |
| `src/features/git/commit-box.tsx` | 메시지 입력·자동 생성 진행·충돌 차단 사유와 커밋 버튼/수정키+Enter 실행 가능 여부를 표시합니다. | 직접 컴포넌트 테스트 미확인. IME·충돌·생성 실패·키보드 실행 실기 필요. |
| `src/features/git/git-change-group.tsx` | 독립 DOM 컴포넌트가 아니라 merge/staged/unstaged 그룹의 행·행동·context menu 구성을 만듭니다. | `src/widgets/git-panel/git-panel.test.tsx`는 상위 패널 경계. 그룹별 실제 stage·diff·discard 실기 필요. |
| `src/features/git/git-section-count-badge.tsx` | 변경 그룹 건수를 compact badge로 표시합니다. | 직접 컴포넌트 테스트 미확인. 큰 건수·색 대비·발화 확인 필요. |
| `src/features/git/git-section-header.tsx` | 그룹 접기·펼치기, 건수, 그룹 전체 action과 키보드 활성화 경계를 제공합니다. | `src/features/git/git-section-header.test.tsx` 직접 검사. 좁은 폭·focus·동작 실기 필요. |
| `src/features/git/stash-list.tsx` | stash 목록의 apply/drop 버튼과 비활성 상태를 표시합니다. | 직접 컴포넌트 테스트 미확인. 충돌·삭제 확인·키보드 실기 필요. |
| `src/features/git/status-row-item.tsx` | 파일 상태 행의 선택·키보드 활성화·행별 action을 표시합니다. | `src/widgets/git-panel/git-panel.test.tsx`는 상위 패널 경계. rename·긴 경로·context menu 실기 필요. |

이 문서의 21개 경로는 기존 메인·overlay·설정·provider inventory에 직접 적히지 않았습니다. 다섯 문서의 중복 제외 직접 연결은 139/212개이며 남은 73개는 역할·상태·자동 근거 연결이 필요합니다. 이 수치는 GUI·시각·접근성 통과율이 아닙니다.
