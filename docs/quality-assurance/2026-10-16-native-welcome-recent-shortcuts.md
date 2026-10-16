# Welcome 최근 프로젝트·단축키 카드

## 대상

- shell-27: 최근 프로젝트 목록. 상한, rootMissing 비활성, 조회 실패 안내를 포함합니다.
- shell-28: 단축키 카드. effective keymap을 반영합니다.
- 원본: `src/widgets/welcome/welcome-container.tsx`, `src/features/welcome/welcome-screen.tsx`

## 원본 동작

- 최근 목록은 `list_recent_projects`입니다. `last_opened_at` 내림차순이고, 매 조회 때 root 존재 여부를 다시 계산합니다.
    - 현재 Welcome의 프로젝트는 목록에서 뺍니다.
    - `RECENT_PROJECT_DISPLAY_LIMIT` 8개까지 보입니다.
    - 조회 실패면 목록 대신 `app.recentProjectsUnavailable`만 보입니다.
    - root가 없는 항목은 `aria-disabled`로 클릭되지 않습니다. 이름 아래에 경고 문구와 root를 보이고, hover하면 root를 보입니다.
    - 열린 프로젝트를 고르면 activate하고, 아니면 root로 open합니다.
- 단축키 카드는 `quick-open, command-palette, search, toggle-terminal, toggle-sidebar, save` 순서입니다.
    - `applyKeymapOverrides`의 첫 override를 적용하고 `descriptionKey`를 번역합니다.
    - 해제된 키는 빈 문자열로 표시합니다.
    - 항목이 없으면 카드를 숨깁니다.

## 구현

- UI:
    - `ShellSurfaces::welcome_content`를 추가했습니다. 기본 구현이 있어 기존 구현체와 동결 remote-web은 그대로입니다.
    - `WelcomeContent`/`WelcomeRecent`/`WelcomeShortcutRow`, 최근 목록·조회 실패·2열 단축키 카드 렌더링과 `ShellIntent::SelectRecentProject`를 추가했습니다.
    - keymap catalog에 `welcome_shortcuts`를 추가했습니다(기본 JSON·override·Monaco 표기 재사용).
- App:
    - `welcome.rs`가 Welcome이 보인 동안 shell snapshot이 바뀔 때마다 기존 runtime `project_list_recent`를 transient 작업으로 다시 조회합니다. 원본 query의 mount 조회와 프로젝트 변경 invalidate에 대응합니다.
    - 단축키 행은 overrides 문자열 단위로 캐시합니다.
    - 선택은 이미 열린 프로젝트면 `ActivateProject`, 아니면 `HostCommand::OpenProject(root)`로 처리합니다.

## 검증

- `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib welcome::`는 2통과입니다.
    - mac 기본 표기(⌘P/⇧⌘P/⇧⌘F/⌃`/⌘B/⌘S), 비 mac override(Ctrl+Alt+O), 해제(빈 표기)와 캐시를 확인했습니다.
    - 실제 AppState와 임시 project 기록 10개로 다음을 확인했습니다.
        - 최신순 정렬과 자기 제외
        - 8개 상한
        - root 누락 재계산
        - 같은 snapshot의 재조회 없음
        - 실패 시 None
        - 작업 회수
- `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --features native-host,inspection --test workbench --lib`는 13/136통과입니다.
    - 새 실제 egui 검사로 목록·경고 문구·카드 텍스트, root 누락 행 클릭 무시, 정상 행 클릭의 intent, 조회 실패 안내와 카드 숨김을 확인했습니다.
- App `--all-targets` check, 동결 remote-web Wasm lib check, 소유 Rust rustfmt는 exit 0입니다.

## 남은 범위

열기 실패 toast 문구는 기존 `HostCommand::OpenProject` 실패 보고 경로를 따릅니다. 화면 픽셀과 실기는 배치 31입니다.
