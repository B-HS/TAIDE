# Zen 안내 배너와 fullscreen opt-in

## 대상

- shell-20: Zen 진입 안내 배너(3초). 원본은 `src/features/window/zen-mode-hint.tsx`입니다.
- shell-19: Zen 풀스크린 opt-in과 첫 렌더 보호. 원본은 `src/widgets/app-shell/use-window-chrome.ts` 74-97입니다.

## 원본 동작

- 메인 `AppShell`만 `ZenModeHint`를 그립니다. Zen이 켜질 때마다 새로 mount되어 3000ms 동안 보이고, Zen이 꺼지면 즉시 사라집니다.
- 스타일:
    - 위치: `absolute top-3 left-1/2 -translate-x-1/2`, `pointer-events-none`
    - 외형: `bg-panel-background border-app-border rounded-md border px-3 py-1.5 text-xs shadow-lg`
    - 텍스트: `zen.hint`(medium)와 `ml-2`의 `zen.hintExit`(sidebar icon 색)
- fullscreen은 `zen && zenFullscreen`이 바뀔 때만 보냅니다. 첫 렌더는 현재 값으로 시작해서 아무것도 보내지 않습니다.

## 구현

- `zen::Hint`는 Zen 전환마다 시작 시각을 다시 잡습니다. 3초 안에는 foreground layer에 배너를 그리고 남은 시간 뒤 repaint를 예약합니다. 입력은 받지 않습니다.
    - 색: `panel.background`, `app.border`, `app.foreground`, `appSidebar.iconDefault`
    - 크기: 12px/16px 글꼴, 12×6 padding, 8px 간격, 6px 반경, shadow-lg 근사(offset 0,10·blur 15)
    - 메인 창에서만 그립니다.
    - 테마가 바뀌면 다른 appearance와 같은 경로로 갱신합니다.
- shell-19는 기존 `zen::Fullscreen`이 이미 구현하고 있습니다. 생성 시 `zen && enabled`로 seed하고, 값이 바뀔 때만 `ViewportCommand::Fullscreen`을 보냅니다. 기능표가 missing으로 남아 있던 것은 갱신 누락이었습니다.

## 검증

`cargo test --manifest-path native/taide-native-app/Cargo.toml --target-dir experiments/native-shell-spike/target --locked --offline --lib zen::`는 4통과입니다.

- 새 검사의 최초 실행은 textures_delta를 비우지 않은 fixture 오류로 실패했습니다. 이를 고친 뒤 통과했습니다. 새 검사는 다음을 확인합니다.
    - 비 Zen에서는 보이지 않습니다.
    - 진입 직후 두 텍스트가 보이고 가로 중앙, top 12에 놓입니다.
    - 2999ms에는 보이고 3000ms부터 사라집니다.
    - Zen을 나갔다 다시 들어오면 다시 보이고, 나가면 즉시 사라집니다.
- 기존 `zen_keymap…optin_fullscreen전환` 검사가 다음을 확인합니다.
    - 설정과 Zen 조합 전환 때만 fullscreen 명령을 보냅니다.
    - 복원 시 첫 프레임에는 명령을 보내지 않습니다.

App `--all-targets` check와 rustfmt는 exit 0입니다. 경고 2건은 이전부터 있던 것입니다.

## 남은 범위

실제 macOS fullscreen 전환 실기와 픽셀 대조는 배치 31입니다.
