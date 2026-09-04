# 기능 — 설정 화면 · 알림(앱 toast · OS 알림)

> Phase 7.5. 사용자 지적 11·12·13번. 설정 데이터 모델은 `settings` 도메인,
> 테마 편집기는 `theme-system.md` §7.3.

## 1. 레이아웃 (13번)

**현재 문제**: `max-w` 로 폭이 제한돼 넓은 화면에서 좌우가 비고, native `<input>`/`<select>` 를 써서
OS 마다 생김새가 다르다.

### 1.1 구조

```
[좌: TOC 탭 (고정폭, sticky)] | [우: 컨텐츠 (가용 폭 전부 사용)]
```

- 좌측 TOC 는 **스크롤에 따라 현재 섹션이 하이라이트**되고, 클릭 시 해당 섹션으로 스크롤.
- 우측 컨텐츠는 **`max-w` 를 걸지 않는다.** 대신 카드 그리드가 폭에 따라 열 수를 늘린다
  (좁으면 1열, 넓으면 2~3열). 한눈에 많이 보이는 것이 목적.
- 섹션(실제 13종 — `settings-view.tsx` `SETTINGS_SECTION_ID` 정본): Appearance / Language /
  Interface / Notifications / Editor / Snippets / Terminal / Keymap / LSP / AI / Plugins / Sync / Remote
- Interface 섹션에는 `explorerAutoReveal` 스위치가 있다(2026-09-04 추가, 기본 on — VS Code
  `explorer.autoReveal` 파리티). 라벨·설명 키는 `settings.explorerAutoReveal` /
  `settings.explorerAutoRevealDescription`, 동작 규약은 `explorer-sidebar.md` §2.2 "활성 파일 자동
  표시".
- 같은 섹션의 `welcomeOnEmptyEditor` 스위치(사용성 배치 4 B, 기본 on)는 **메인 창의 탭 0 편집
  영역에 Welcome 화면을 띄울지**를 정한다. 라벨·설명 키는 `settings.welcomeOnEmptyEditor` /
  `settings.welcomeOnEmptyEditorDescription`, 동작 규약과 표면 3종 대조는 `layout-shell.md` §1.1
  "빈 에디터 영역 = Welcome". 끄면 기존 `editor.noFileOpen` 문구로 돌아간다(보조 창은 설정과
  무관하게 항상 문구 — 보조 창은 트리가 비면 스스로 닫히기 때문).

### 1.2 컴포넌트 규칙 (전 OS 일관 — acknowledge §3.1)

**native 폼 위젯을 직접 쓰지 않는다.** 자체 컴포넌트로 만든다 — 실제 배치는 `shared/ui` 가
아니라 대부분 `features/settings/`(`text-field`·`font-picker`·`numeric-field`·`option-picker`·
`switch-field`·`keybinding-row`)와 `features/theme/`(`color-picker`)에 있고, `shared/ui` 에는
`switch` 등 shadcn 계열 primitive 만 둔다. (아래 표의 개념 이름 ↔ 실제 파일명 대응:
NumberInput→`numeric-field.tsx`, Select→`option-picker.tsx`, KeybindingInput→`keybindings-editor`
내장 캡처 UI.)

| 용도 | 컴포넌트 |
|------|----------|
| on/off | `Switch` |
| 선택 1 | `Select`(Radix) — native `<select>` 금지 |
| 숫자 | `NumberInput`(스테퍼 포함) |
| 문자 | `TextField` |
| 색 | `ColorPicker`(자체 — `<input type="color">` 금지) |
| 폰트 | `FontPicker`(시스템 폰트 열거 + 미리보기) |
| 키 | `KeybindingInput`(키 조합 캡처) |

- 각 설정 항목은 **카드**로 감싸고 제목·설명·컨트롤·(있으면) 기본값 복원 버튼을 담는다.
- 변경은 즉시 반영(`settings_update` 부분 patch) — 저장 버튼을 두지 않는다.

## 2. 알림 toast (11번)

- 성공/확인/에러는 **toast 로 통일**한다. `sonner` 를 계속 쓴다.
- **위치는 설정에서 9분할 중 선택**: `{top,middle,bottom} × {left,center,right}`.
  - 설정 키: `toast_position`. 값은 `as const` union (enum 금지).
  - sonner 의 `position` 은 6종(`top-left|top-center|top-right|bottom-*`)만 지원한다 —
    **`middle-*` 3종은 컨테이너 CSS 로 직접 수직 중앙 정렬**해야 한다. 구현 시 확인 필요.
- 기본값은 `bottom-right`.
- 에러 toast 는 **자동으로 사라지지 않게**(중요 정보 유실 방지) 하고, 성공은 짧게 자동 소멸.

## 2.1 Notifications 섹션 — OS 네이티브 알림 (사용성 배치 4 A)

> 정본: `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §A.
> 앱 안 toast(§2)는 그대로 두고, **완료성 이벤트만** OS 알림 센터에 추가로 보낸다.

### 2.1.1 스위치 8개 (`settings-notification-section.tsx`)

| 설정 키 | 라벨 키 | 기본값 | 의미 |
|---------|---------|:------:|------|
| `notifications_enabled` | `settings.notificationsEnabled` | true | 마스터 스위치 |
| `notifications_only_when_unfocused` | `settings.notificationsOnlyWhenUnfocused` | true | TAIDE 창이 하나라도 포커스면 보내지 않음 |
| `notify_agent_completed` | `settings.notificationsAgentCompleted` | true | 에이전트 작업 완료 |
| `notify_task_completed` | `settings.notificationsTaskCompleted` | true | 오래 걸린 터미널 명령 완료 |
| `notify_git_remote` | `settings.notificationsGitRemote` | true | git push·pull(+sync) 성공 |
| `notify_search_replace` | `settings.notificationsSearchReplace` | true | 파일에서 바꾸기 완료 |
| `notify_lsp_install` | `settings.notificationsLspInstall` | true | 언어 서버 설치 완료 |
| `notify_error` | `settings.notificationsError` | true | 위 카테고리의 **실패** 전부 |

- **실패는 전부 `error` 카테고리**로 보낸다(제목은 각자의 실패 키 — `notification.gitPushFailed` ·
  `notification.taskCompletedFailed` 등). "Failures" 스위치 하나로 모든 실패 알림을 끌 수 있게 하기
  위해서다. 성공/실패 판정 지점은 §2.1.3 표.
- 판정은 전부 Rust(`domain::notification::service::decide_delivery`)가 한다 — "어느 창도 포커스가
  없다"는 앱 전체 사실이라 창 하나가 자기 포커스만 보고 판단하면 다른 창을 보고 있는 사용자에게
  알림이 간다. FE 는 "이 창이 보낼 자격이 있는가"(메인 창 · 원격 미러 아님)만 본다
  (`shared/lib/native-notification-gate.ts` `shouldForwardNativeNotification`).

### 2.1.2 버튼 2개와 제약 (권한 상태를 알 수 없음)

- `tauri-plugin-notification` 2.4.0 데스크톱 백엔드는 `permission_state()` 가 항상 `Granted` 스텁이고
  `show()` 실패도 삼킨다 → **"사용자가 TAIDE 알림을 꺼 두었다"를 앱이 감지할 방법이 없다.** 그래서
  권한 거부를 감지해 안내하는 대신 **아래 두 버튼을 상시 노출**한다.
  - **알림 설정 열기** — `notification_open_system_settings`(macOS 전용, 실패 시 `describeIpcError`
    토스트).
  - **테스트 알림 보내기** — `notification_notify` 를 `error` 카테고리로 직접 호출한다(파사드를
    거치지 않는다 — 보조 창에서 눌러도 결과를 그대로 보여줘야 하므로). 전달되면
    `notification.enableHint` 를 성공 토스트로, 억제되면 **그 알림을 삼킨 스위치의 라벨**을 경고
    토스트로 보여준다(`SUPPRESSION_REASON_LABEL_KEY`). 기본 설정에서 설정 화면을 보며 누르면
    `windowFocused` 로 억제되는 것이 정상 동작이다.
- **첫 전달 1회 안내 토스트**: 세션 중 처음으로 알림이 실제로 전달되면
  `notification.enableHint` 를 sonner `action`("알림 설정 열기")과 함께 1회만 띄운다. 발신처는 여러
  레이어에 흩어져 있고 "이미 안내했다" 상태는 한 곳이어야 하므로, 전달 사실은
  `entities/notification/notify.ts` 의 브리지가 알리고 `NativeNotificationProvider` 가 `useRef` 로
  1회를 보장한다.
- **dev 실행 제약**: dev 빌드(`import.meta.env.DEV`)에서만 `settings.notificationsDevHint` 를 섹션
  하단에 보여준다 — 플러그인이 dev 에서 `com.apple.Terminal` 로 스푸핑하므로 알림이 Terminal 의
  이름·아이콘으로 뜨고 Terminal 의 권한을 따른다. 패키징 빌드는 정상이다.

### 2.1.3 발화 지점 (5카테고리)

| 카테고리 | 발화 지점 | 제목 / 본문 |
|----------|-----------|-------------|
| `agentCompleted` | `native-notification-provider.tsx` — `agent:state-changed` 의 Working→Idle\|AwaitingInput 전이, **Working 10초 이상**(`AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS`) | `notification.agentCompleted` / 에이전트 이름 |
| `taskCompleted` | `native-notification-provider.tsx` — Rust 가 발행하는 `terminal:command-finished`(OSC 133 `C`→`D`), **실행 10초 이상**(`TASK_COMPLETION_NOTIFY_MIN_DURATION_MS`) | `notification.taskCompleted{Succeeded,Failed}` / 세션 cwd |
| `gitRemote` | `entities/git/git.query.ts` `usePushGit`·`usePullGit`(sync 는 이 둘의 조합) | `notification.git{Push,Pull}{Succeeded,Failed}` / 브랜치명 또는 실패 사유 |
| `searchReplace` | `search-panel-container.tsx` 치환 성공 | `notification.searchReplaceDone` / `search.replaceDone` 요약 |
| `lspInstall` | `native-notification-provider.tsx` — `lsp:install-progress` 의 `done`·`failed` | `notification.lspInstall{Succeeded,Failed}` / 서버 id 또는 메시지 |

- **에이전트·터미널의 10초 임계**: 짧은 턴·짧은 명령까지 알리면 알림 채널 자체를 무시하게 된다.
  "사용자가 화면을 떠났을 만한 길이"를 기준으로 둘 다 10초를 쓴다(상수는 별개 —
  `shared/constants/notification.ts`).
- **터미널 실행 시간은 OSC 133 `C`→`D`** 로 잰다. `A`(프롬프트 시작)부터 재면 타이핑·대기 시간이
  명령 시간에 포함된다. `C` 없이 `D` 만 오는 셸은 이벤트 자체가 발행되지 않아 **알리지 않는다**.
  측정 지점은 프론트 트래커가 아니라 **pty reader 스레드**다(웨이브 1 리뷰 F-1) — 탭이 배경이면
  xterm 이 언마운트돼 프론트에는 알림이 필요한 바로 그 명령이 보이지 않는다(`terminal.md` §5).
- 알림 텍스트는 전부 FE 가 `t()` 로 만들어 Rust 에 넘긴다. Rust 는 "보낼지"만 결정하고 문구는 모른다.
  제목·본문은 `NOTIFICATION_TEXT_MAX_CODE_POINTS`(200 코드포인트)로 잘라 보낸다 — LSP 설치 stderr·git
  원격 거절 메시지처럼 길이 상한이 없는 외부 텍스트가 그대로 알림 센터에 실리기 때문이다.
- **보조 에디터 창·원격 미러는 발신하지 않는다**(`notifyNative` 파사드에서 즉시 반환). 백엔드가
  모든 창에 브로드캐스트하는 이벤트(에이전트·LSP·터미널 명령 완료)를 창마다 보내면 창 수만큼
  중복되기 때문이다. 터미널은 발화가 Rust 이벤트로 올라간 뒤로 **보조 창에서 끝난 명령도 메인 창이
  대신 알린다**(git·검색 패널은 메인 창 전용이라 무관).
- 검색 치환 실패·에이전트 실패는 알리지 않는다(대응 로케일 키가 없고, 실패 알림은 위 4종으로 한정).

## 3. 리사이저 두께 (12번)

**현재 문제**: 사이드바 ↔ codeview/터미널 사이 구분선이 굵다.

- 설정 키: `resizer_thickness`(기본 1px). 시각적 두께만 바꾼다.
- **잡는 영역은 시각적 두께와 분리**한다 — react-resizable-panels 의 `resizeTargetMinimumSize`
  로 히트 영역을 넉넉히(예: 6~8px) 두면 "얇아 보이지만 잡기 쉬운" 상태가 된다.
  이 분리를 안 하면 얇게 만든 순간 드래그가 어려워진다.
- 상세는 `tabs.md` §5.

## 4. 수명주기

- 설정 화면은 `TabKind::Settings` 탭으로 열린다(별도 OS 창 아님).
- 설정 변경은 `settings_update` → `settings` 쿼리 갱신 → 소비자(에디터·터미널·toast)가 반응.
  **각 소비자가 설정을 폴링하지 않는다.**

## 5. 저장 실패·미선택 표시 (d-51 F6 · 감사 §4-B B15·C12)

- **저장 실패는 무조건 toast 로 알린다.** 실패 보고는 호출부가 아니라
  `entities/settings/settings.query.ts` 의 두 뮤테이션(`useUpdateSettings`·`useSetThemeId`)이
  공통 `onError` 로 한다 — 컨트롤이 캐시된 `Settings` 를 그대로 렌더하므로, 실패하면 값이 원래대로
  되돌아갈 뿐 아무 설명이 없었다. 문구는 `settings.saveFailed` + `describeIpcError(error)` 설명줄.
  호출부에서 같은 실패를 또 toast 하지 않는다(v5 는 훅·호출 양쪽 `onError` 를 모두 실행한다).
- **숫자 필드는 blur 시 실제 저장값으로 되돌려 쓴다**(`numeric-field.tsx` +
  `shared/lib/numeric-field-commit.ts`). 입력은 비제어라 blur 시점의 텍스트는 사용자가 남긴 그대로다
  — 범위를 넘긴 값을 clamp 해 보내면서 화면은 그대로 두면 "쓰여 있는 값 ≠ 적용된 값" 이 남고, 저장이
  실패한 경우에도 입력한 숫자가 저장된 것처럼 보인다. clamp 결과가 저장값과 같으면 IPC 를 보내지 않는다.
- **텍스트 필드도 정규화가 있는 항목이면 blur 시 되돌려 쓴다**(`text-field.tsx` 의 `normalize` prop,
  현재 사용처는 `editorRulers` — `shared/lib/editor-rulers.ts` 의 `normalizeEditorRulersText`).
  `TextField` 는 `key={value}` 로 저장값이 바뀔 때만 리마운트하는데, 정규화 결과가 기존 저장값과
  같으면(`[80]` 에 `80, 2000` 을 쓰거나 빈 목록에 `abc` 를 쓰는 경우) 값이 안 바뀌어 리마운트도 없다
  — 되돌려 쓰지 않으면 버려진 텍스트가 화면에 남는다. `normalize` 를 넘기지 않은 필드
  (`shellOverride`·`aiOmlxBaseUrl`)는 타이핑한 문자열이 곧 저장값이라 동작이 그대로다.
- **선택되지 않은 값은 placeholder 로 보인다**(`option-picker.tsx` 의 `placeholder`). 매칭되는
  옵션이 없을 때 첫 옵션을 대신 표시하던 폴백을 없앴다 — AI 프로바이더를 바꾸면 `aiModel` 이 비는데
  모델 피커가 첫 모델을 선택된 것처럼 광고했다.
