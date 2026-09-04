# 기능 — 설정 화면 · 알림(toast)

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
- 섹션(실제 12종 — `settings-view.tsx` `SETTINGS_SECTION_ID` 정본): Appearance / Language /
  Interface / Editor / Snippets / Terminal / Keymap / LSP / AI / Plugins / Sync / Remote
- Interface 섹션에는 `explorerAutoReveal` 스위치가 있다(2026-09-04 추가, 기본 on — VS Code
  `explorer.autoReveal` 파리티). 라벨·설명 키는 `settings.explorerAutoReveal` /
  `settings.explorerAutoRevealDescription`, 동작 규약은 `explorer-sidebar.md` §2.2 "활성 파일 자동
  표시".

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
