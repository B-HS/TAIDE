# 파일·Git 트리 선택, 탭 재사용, 포커스 복귀, 알림 CTA 결함

## 대상 파일

- `src/features/explorer/file-tree.tsx`, `src/widgets/git-panel/git-panel.tsx`
- `src/entities/layout/layout.query.ts`, `src/shared/lib/pane-tree.ts`
- `src/app/providers/theme-provider.tsx`, `src/shared/lib/window-appearance.ts`
- `src/app/providers/native-notification-provider.tsx`
- `src/app/providers/keybindings-runtime-provider.test.tsx`
- `src/widgets/editor-area/pane-node-view-focus.test.tsx`, `src/widgets/editor-area/use-request-close-tab.test.tsx`

## 리포트

파일 트리와 Git 변경 목록이 단일 선택만 보존해 Shift 범위 선택과 Command/Ctrl 추가 선택이 불가능했습니다. 트리에서 파일·diff를 열 때 preview 탭을 요청했고, 일반 파일 탭 재사용도 요청 대상 pane 안에서만 수행되어 같은 창의 다른 pane에 열린 파일로 이동하지 못했습니다. 창 포커스 복귀 시에는 테마 적용 guard를 무효화해 같은 native appearance를 다시 적용할 수 있었고, macOS의 app-wide appearance repass가 화면 리프레시처럼 보였습니다. OS 알림 전달 안내 toast는 전달 성공 뒤에도 시스템 설정 CTA를 항상 붙였습니다.

## 상세

- `src/shared/lib/list-selection.ts`에 순서·anchor 기반 선택 계산을 추가했습니다. 일반 클릭은 단일 선택, Shift는 연속 범위, Command/Ctrl은 추가·해제, Command/Ctrl+Shift는 범위 추가로 처리합니다.
- 파일 트리와 Git 변경 행은 modifier 클릭에서 파일·diff 열기를 실행하지 않고 선택과 ARIA 상태만 갱신합니다. 일반 클릭과 키보드 활성화 동작은 유지합니다.
- 파일 트리, Git diff, Git의 Open File 요청을 permanent 탭으로 변경했습니다. `useOpenFileTab`은 명시적 split target이 없을 때 현재 OS 창 안의 동일 `file` 탭 pane을 먼저 사용합니다. 같은 경로의 `diff` 탭은 별도 kind라 재사용하지 않습니다.
- 포커스 복귀 시 appearance guard를 초기화하지 않습니다. 대신 native `Window.theme()`을 확인하고 실제 타입이 다를 때만 `setTheme()`을 호출해 다른 창의 appearance 변경은 복구하되 같은 값의 app-wide repass는 막습니다.
- native 알림 전달 성공 안내 toast에서 시스템 설정 action을 제거했습니다. 설정 화면의 명시적 “알림 설정 열기” 버튼과 실제 권한 조회 이월 항목은 유지합니다.
- dev CI 런 `35693825184`의 FE 12건 실패는 제품 코드 회귀가 아니라 Bun의 테스트 파일 간 `mock.module` 누수였습니다. Linux 파일 순서에서는 앞선 테스트가 `layout.ipc`·`settings.ipc`를 교체한 뒤 대상 테스트가 더 아래의 generated `commands`를 감시해 호출을 보지 못했습니다. 세 테스트 모두 실제 소비 경계인 IPC namespace를 감시하도록 바꾸고, 설정 캐시는 `gcTime: Infinity` 테스트 쿼리로 유지해 실행 순서 의존성을 제거했습니다.

## 검증

- `bun test src/shared/lib/list-selection.test.ts src/shared/lib/pane-tree.test.ts src/entities/layout/use-open-file-tab.test.tsx src/shared/lib/window-appearance.test.ts src/app/providers/native-notification-provider.test.ts src/features/explorer/file-tree.test.tsx src/widgets/git-panel/git-panel.test.tsx`
- `bun run typecheck`
- `bun run lint` — 오류 0, 기존 warning 11
- `bun run format:check`
- `bunx --bun bun@1.3.14 test --randomize --seed=1 src/entities/file/file.query.test.ts src/entities/settings/settings.query.test.ts src/entities/layout/use-open-file-tab.test.tsx src/widgets/search-panel/search-panel-container.test.tsx src/widgets/editor-area/pane-node-view-focus.test.tsx src/widgets/editor-area/use-request-close-tab.test.tsx src/app/providers/keybindings-runtime-provider.test.tsx` — layout/file IPC 오염 순서 63 pass
- `bunx --bun bun@1.3.14 test --randomize --seed=2 src/entities/settings/settings.query.test.ts src/app/providers/keybindings-runtime-provider.test.tsx` — settings IPC 오염 순서 13 pass

## 남은 위험

- macOS 실기에서 백그라운드 복귀 시 native appearance repass가 사라졌는지는 별도 확인이 필요합니다.
- `tauri-plugin-notification` desktop backend가 실제 권한 상태를 반환하지 않는 제약은 남아 있습니다. 권한별 UI가 필요하면 `UNUserNotificationCenter` 직접 연동이 선행되어야 합니다.
