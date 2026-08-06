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
- 섹션: 일반 / 모양(테마·폰트) / 에디터 / 터미널 / Git / LSP / 플러그인 / 키보드 / 알림 / 고급

### 1.2 컴포넌트 규칙 (전 OS 일관 — acknowledge §3.1)

**native 폼 위젯을 직접 쓰지 않는다.** 전부 `shared/ui` 의 자체 컴포넌트로 만든다.

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
