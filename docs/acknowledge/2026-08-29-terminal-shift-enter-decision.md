# 터미널 Shift+Enter 지원 방식 결정 (2026-08-29)

> 배경: 내장 터미널에서 Claude Code 실행 시 Shift+Enter 가 줄바꿈 삽입으로 동작하지 않는다는
> 사용자 보고. 사용자 질문 2건(Shift+Enter 지원 방식·네이티브 터미널 임베드 가능 여부)에 대한
> 조사와 결정 기록.

## 조사 요지

- xterm.js 6.0 은 Shift+Enter 를 Enter 와 동일한 CR(`\r`)로 인코딩한다. kitty keyboard
  protocol·xterm modifyOtherKeys 를 모두 미지원해(번들 실측 grep — 관련 심볼 0) 조합 구분
  시퀀스를 낼 수 없다. VS Code 가 `/terminal-setup` 으로 키바인딩을 별도 주입하는 것도 같은 이유.
- Claude Code 공식 문서(code.claude.com/docs terminal-config·keybindings) 기준:
  - Shift+Enter 네이티브 지원 터미널: Ghostty·kitty·WezTerm·iTerm2·Warp·Apple Terminal·
    Windows Terminal. VS Code·Cursor·Alacritty·Zed 는 `/terminal-setup` 필요.
  - **Ctrl+J(LF, `0x0a`)는 터미널 불문 "줄바꿈 삽입"** 으로 문서화되어 있다. `\` + Enter 도 동일.
  - Claude Code 가 stdin 에서 읽는 Shift+Enter 의 정확한 CSI 시퀀스는 공개 문서에 명시돼 있지
    않다 (kitty protocol·modifyOtherKeys 지원 언급만 존재).

## 결정

1. **Shift+Enter → LF(`\n`) 변환 채택** (추천안). xterm 커스텀 키 핸들러에서 조합 키 없는
   Shift+Enter keydown 만 가로채 LF 를 PTY 로 보낸다. Claude Code 는 Ctrl+J 와 동일하게
   줄바꿈을 삽입하고, 일반 셸(zsh/bash ZLE)은 CR·LF 모두 accept-line 이라 프롬프트 동작이
   보존된다. IME 조합 중(`isComposing`)이거나 alt/ctrl/meta 가 섞이면 기존 파이프라인 유지.
   - 알려진 한계: LF 를 Enter 와 다르게 취급하는 극소수 TUI(예: nano 의 Ctrl+J=justify)에서는
     Shift+Enter 가 그 기능으로 동작한다. 수용.
2. **kitty keyboard protocol 구현은 이번 범위에서 제외**. 키 인코딩 전반(Esc·Ctrl 조합 등)이
   바뀌는 큰 작업 + 회귀 리스크. 필요 시 별도 작업으로 재검토.
3. **네이티브 터미널(iTerm/Terminal.app) 임베드는 불가** — macOS 에 외부 앱 창을 웹뷰에 넣는
   수단이 없고, 웹뷰 임베드 가능한 성숙 에뮬레이터는 xterm.js 가 사실상 유일
   (`docs/research/terminal-reevaluation.md` §0.1, ADR-0005 존속 결론 유지). 셸 자체는 이미
   실제 PTY 위의 네이티브 zsh/bash 로 동작하며 xterm.js 는 렌더러다.
4. **"외부 터미널에서 열기" 커맨드는 이번엔 추가하지 않음** (사용자 결정 — 필요 시 재요청).

## 반영

- `src/features/terminal/terminal-view.tsx` — `shouldTranslateShiftEnterToLineFeed` 판정 +
  `attachCustomKeyEventHandler` 변환 (keydown 에서 `preventDefault` 로 textarea 개행 삽입 차단).
- `src/features/terminal/terminal-view.test.ts` — 판정 함수 단위 테스트 6케이스.
