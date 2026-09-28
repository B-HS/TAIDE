# TS view 앱 provider 경계 inventory

## 범위와 판정

`src-tauri/tests/fixtures/rust-native/ts-view-components-v1.json`의 비테스트 `.tsx` 212개 중 앱 provider 11개를 실제 소스의 책임·상태·이벤트 경계에 연결했습니다. 이 목록은 화면을 추가로 11개 찾았다는 뜻이 아닙니다. provider는 메인·보조 창의 상태 동기화와 사용자 조작을 연결하므로 native UI 전환 때 화면별 동작 보존 항목으로 추적합니다. 자동 근거는 테스트 파일의 존재와 코드 경계만 뜻하며, 이번 기록에서 테스트를 새로 실행하거나 GUI·접근성을 통과 판정하지 않았습니다.

| 경계와 경로 | 현행 상태·동작 | 자동 근거 | 남은 실기·접근성 확인 |
| --- | --- | --- | --- |
| 외부 agent 열기 `src/app/providers/agent-external-open-provider.tsx` | 시작 시와 `agentExternalOpen` 이벤트에 대기 요청을 비우고 프로젝트·파일 탭을 열며 `--wait` marker를 pinned tab 닫기와 연결합니다. 대상 부재·IPC 오류는 안내/marker 회수로 처리합니다. | `src/entities/agent/external-open-target.test.ts`, `src/entities/agent/agent-wait-marker-registry.test.ts`; provider 직접 GUI 검사는 미확인 | 실제 CLI 요청 전달·파일 닫기·프로젝트 미일치·보조 창 복귀, 토스트 발화 |
| agent 상태 `src/app/providers/agent-state-sync-provider.tsx` | 메인·보조 창에서 프로젝트별 agent 상태 push를 query cache에 동기화하며 sidebar가 숨은 Zen 모드에도 유지합니다. | `src/app/providers/agent-state-sync-provider.test.ts` | Zen·보조 창 전환 중 배지 최신성·발화 |
| 공통 provider `src/app/providers/app-providers.tsx` | 창별 QueryClient·Tooltip·toast 계층을 조립하고 QueryClient를 창에 연결합니다. | 직접 provider 테스트 미확인; `src/shared/constants/toast.test.ts`는 toast 설정만 확인 | 첫 로딩·오류 toast·tooltip 포커스와 보조 창 표시 |
| Emmet `src/app/providers/emmet-provider.tsx` | 설정의 `emmetEnabled`에 따라 Monaco 등록을 설치·해제합니다. 설정 미도착 시 기본 활성입니다. | provider 직접 테스트 미확인 | 설정 전환 직후 약어 확장·해제, 다중 창 editor 결과 |
| 외부 링크 `src/app/providers/external-link-provider.tsx` | 모든 anchor의 primary click을 검사해 앱 밖 URL이면 웹뷰 이동을 막고 외부 열기를 요청합니다. 실패는 toast로 알립니다. | `src/shared/lib/external-anchor.test.ts`는 URL 판정만 확인 | Markdown 링크·nested target·보조 창·실패 toast, 키보드 링크 활성화 |
| hot-exit flush `src/app/providers/hot-exit-flush-provider.tsx` | Rust의 all/window/project flush 요청을 창별로 계획하고 editor mirror·LSP 세션을 마감 예산 내 flush한 뒤 완료를 응답합니다. | `src/app/providers/hot-exit-flush-provider.test.ts` | 실제 dirty tab·프로젝트 닫기·보조 창 닫기·앱 종료의 파일 보존과 시간 초과 |
| IDE 동기화 `src/app/providers/ide-sync-provider.tsx` | IDE diff/save/close 요청, 상태와 focused project의 Monaco 진단을 연결합니다. 읽기 전용·모델 부재 저장은 실패로 응답합니다. | `src/app/providers/ide-sync-provider.test.tsx` | 실제 IDE WebSocket 도구 왕복·보조 창 dirty 파일·진단 갱신·오류 안내 |
| IPC 동기화 `src/app/providers/ipc-sync-provider.tsx` | 프로젝트·layout·설정·Git·terminal·remote·파일시스템 push를 query cache 및 창 상태에 반영합니다. `fsRescanRequired`는 재조회 경계입니다. | `src/app/providers/ipc-sync-provider.test.ts`, `src/app/providers/ipc-settings-sync.test.tsx` | 대량 FS 변경·다중 창 echo·누락 이벤트·오류/재연결 시 실제 갱신 |
| 키바인딩 runtime `src/app/providers/keybindings-runtime-provider.tsx` | 모든 창에서 keymap override와 글꼴 크기 shortcut을 적용하고 요청 시 keybindings dialog를 지연 로드합니다. 첫 닫기 뒤에도 dialog를 유지합니다. | `src/app/providers/keybindings-runtime-provider.test.tsx` | 메인·보조 창 단축키, 충돌·IME·dialog focus와 닫기 애니메이션 |
| native 알림 `src/app/providers/native-notification-provider.tsx` | agent 완료/입력 대기, terminal 명령 완료, LSP 설치 결과를 창 외 알림으로 내보냅니다. 프로젝트별 작업 시작 시각과 첫 전달 안내를 보유합니다. | `src/app/providers/native-notification-provider.test.ts`, `src/entities/notification/notify.test.ts` | OS 알림 허용·차단, 배경 창, 프로젝트 닫힘 뒤 중복·개인정보 표시 |
| shell slot 포커스 `src/app/providers/shell-slot-provider.tsx` | pointer/focus 이벤트로 현재 slot을 고르고 서버 응답 전 local override를 적용합니다. slot 밖 dialog·sidebar 조작은 기존 포커스를 유지합니다. | `src/app/providers/shell-slot-provider.test.tsx`, `src/shared/lib/shell-slot.test.ts` | 마우스·키보드·보조 기술 포커스, split·dialog·다중 창에서 대상 프로젝트 일치 |

이 문서의 11개 경로는 기존 [메인 inventory](2026-09-28-ts-view-inventory.md)·[overlay inventory](2026-09-28-ts-overlay-inventory.md)·[설정 inventory](2026-09-28-ts-settings-inventory.md)에 직접 적히지 않았던 경로입니다. 네 문서의 중복 제외 직접 연결 경로는 118개이고 남은 94개는 역할·상태·자동 근거 연결이 필요합니다. 경로 연결과 실제 동작·시각·접근성 검증은 별도입니다.
