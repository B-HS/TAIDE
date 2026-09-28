# TS 설정 화면 inventory 진행 기록

## 범위

`src/widgets/settings-view/`의 비테스트 `.tsx` 17개와 `src/features/settings/`의 비테스트 `.tsx` 22개를 대상으로 합니다. `SettingsView`는 프로젝트에 소유된 settings tab이며, 13개 목차 section과 theme/snippet 편집 화면 분기를 가집니다. 아래는 경로·주요 상태·조작의 소스 연결입니다. 모든 설정 필드의 값·오류 표시와 실제 저장 결과를 검증했다는 뜻은 아닙니다.

## 화면과 section

| 영역 | 경로 | 확인한 상태·조작 | 직접 자동 검사·남은 실기 |
| --- | --- | --- | --- |
| root·목차 | `src/widgets/settings-view/settings-view.tsx`, `src/features/settings/settings-section.tsx`, `src/features/settings/settings-toc.tsx` | 로딩 placeholder, section 선택/스크롤, 설정 JSON tab, theme/snippet 화면 교체, remote link·sync conflict 상태 | 직접 화면 검사는 미확인; 목차 키보드·스크롤·화면 복귀·보조 창 실기 필요 |
| appearance | `src/widgets/settings-view/settings-appearance-section.tsx`, `src/features/settings/theme-picker.tsx` | theme 선택·복제·편집, system theme 추종, 사용자 theme 폴더 열기 | section 직접 검사는 미확인; 테마별 화면·색 대비·선택 발화 필요 |
| language | `src/widgets/settings-view/settings-language-section.tsx`, `src/features/settings/language-picker.tsx` | locale 로딩·선택·사용자 locale 폴더 열기, combobox 열림 상태 | 직접 검사는 미확인; 언어 전환·긴 번역·IME·발화 필요 |
| interface | `src/widgets/settings-view/settings-interface-section.tsx`, `src/widgets/settings-view/agent-cli-status-row.tsx`, `src/widgets/settings-view/agent-hooks-project-list.tsx`, `src/widgets/settings-view/agent-hooks-project-row.tsx`, `src/features/settings/agent-hooks-toggle.tsx`, `src/features/settings/toast-position-picker.tsx` | toast 위치·분할 두께·system usage·rail/preview/zen/search 설정, CLI install/uninstall, agent hook 동의/설치/해제 | `src/widgets/settings-view/agent-hooks-project-list.test.tsx`; 실제 CLI/agent hook·동의 dialog·focus 필요 |
| notifications | `src/widgets/settings-view/settings-notification-section.tsx` | 알림 전체/비활성 창/종류별 toggle, 시스템 알림 설정 열기·시험 알림 | section 직접 검사는 미확인; OS 권한·알림 전달·발화 필요 |
| editor | `src/widgets/settings-view/settings-editor-section.tsx`, `src/features/settings/font-picker.tsx`, `src/features/settings/numeric-field.tsx`, `src/features/settings/option-picker.tsx`, `src/features/settings/text-field.tsx` | 글꼴·크기·저장 시 동작·들여쓰기·ruler·cursor·diff·Emmet 등 숫자/선택/toggle/text 입력 | section 직접 검사는 미확인; Monaco 반영·유효범위·키보드·IME 필요 |
| snippets | `src/widgets/settings-view/settings-snippets-section.tsx` | snippet 관리 화면·사용자 폴더 열기 | section 직접 검사는 미확인; 관리 화면 왕복·파일 처리 필요 |
| terminal | `src/widgets/settings-view/settings-terminal-section.tsx`, `src/features/settings/shell-profile-list.tsx` | terminal 글꼴·크기·shell·scrollback·cursor 선택/깜박임, shell profile 버튼 선택 | 직접 검사는 미확인; 실제 PTY·프로필·글꼴·키보드 필요 |
| keymap | `src/widgets/settings-view/settings-keymap-section.tsx`, `src/features/settings/keybinding-row.tsx` | keybindings editor 열기, 단일 binding capture·충돌 해결·reset/unbind | section 직접 검사는 미확인; 키 조합·IME·OS shortcut 충돌 필요 |
| LSP | `src/widgets/settings-view/settings-lsp-section.tsx`, `src/features/settings/lsp-server-status-list.tsx` | server 상태·install/cancel·설치 명령 복사·진행 표시 | 직접 검사는 미확인; 실제 언어 서버 설치·오류·발화 필요 |
| AI | `src/widgets/settings-view/settings-ai-section.tsx`, `src/features/settings/ai-provider-token-row.tsx`, `src/features/settings/ai-omlx-row.tsx`, `src/features/settings/ai-auto-tab-toggle.tsx` | provider token/API key 저장·해제, provider/model 선택·로딩/오류, auto-tab toggle, prompt 파일 열기 | 직접 검사는 미확인; 시크릿 비노출·모델 오류·입력/발화 필요 |
| plugins | `src/widgets/settings-view/settings-plugins-section.tsx` | plugin manager 지연 렌더·목록/설치/가져오기 조작 | section 직접 검사는 미확인; 설치/VSIX/권한 실기 필요 |
| sync | `src/widgets/settings-view/settings-sync-section.tsx`, `src/features/settings/sync-section.tsx`, `src/features/settings/sync-conflict-dialog.tsx` | token 연결·해제, upload/download, 충돌 시 로컬 유지/원격 당김 | 직접 검사는 미확인; 실제 원격 충돌·동의·focus 필요 |
| remote | `src/widgets/settings-view/settings-remote-section.tsx`, `src/features/settings/remote-section.tsx`, `src/features/settings/remote-password-row.tsx`, `src/features/settings/remote-allowed-hosts-row.tsx` | server toggle/link/revoke, password 설정·해제, 허용 host 추가/삭제·검증 | `src/features/settings/remote-allowed-hosts-row.test.ts`; 실제 HTTP/WS·cookie·경고/복사·키보드 필요 |

## 공용 입력 경계

`src/features/settings/switch-field.tsx`는 section들의 Boolean 입력을, `src/features/settings/numeric-field.tsx`는 범위가 있는 숫자 commit을, `src/features/settings/option-picker.tsx`는 선택 combobox를, `src/features/settings/text-field.tsx`는 text commit을 표현합니다. `src/features/settings/font-picker.tsx`와 `src/features/settings/language-picker.tsx`도 combobox 상태를 가집니다. 이 공용 입력의 label·disabled·오류·키보드·IME 동작은 소비 section별 실제 확인이 필요합니다.

## 판정

대상 39개 경로의 화면 역할과 주요 입력/동작을 연결했습니다. 직접 테스트 파일은 2개만 확인했으며 assertion의 완전성은 판정하지 않았습니다. 설정 저장·재시작·테마/로케일·실제 원격/OS 자원과 접근성 실기가 없어 Phase 0 TS view 완료로 계산하지 않습니다.
