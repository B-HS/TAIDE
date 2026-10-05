# M8 native terminal 파일 링크 승인·탭 열기·좌표 이동 기본 연결

## 범위와 대상

N4-C의 파일 후보를 native hover/click→승인 worker→현재 프로젝트/창의 focused pane→shared open_tab의 실제 반환 tab ID→5초 reveal→canonical editor view에 연결했습니다. 새 기본 검사 6건과 변경된 URL 승인 경계의 회귀 1건이 통과했습니다. 전체 파일/GUI/IME 수명, OSC133, N4-C와 M8 전체 완료는 아닙니다.

대상은 `native/taide-native-app/src/{terminal_file_links,editor_reveal,terminal_surface,terminal_tabs,host,application,lib}.rs`, `tests/{terminal-file-links,editor-reveal,terminal-host}.rs`, `native/taide-native-editor/{src,tests}/editing.rs`, `native/taide-native-ui/{src,tests}/editor_surface.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. 보호 실기 bundle·제품 TS·root/MSRV·manifest/lock은 이 단위에서 변경하지 않았습니다. 서브에이전트·OS browser/clipboard·사용자 파일·입력기·VoiceOver를 사용하지 않았고 합성 디렉터리는 해당 테스트의 소유 범위에서 회수했습니다.

## 원본과 API 근거

1. `src/features/terminal/terminal-file-link.ts`의 물리 행·trimEnd·live cwd·256행 FIFO/null cache·실패 미캐시와 shared `taide-terminal::service::resolve_link_candidates`의 첫 16개 canonical/open-project 승인을 확인했습니다. Resolver가 디렉터리를 별도로 제거하지 않는 기존 동작을 유지하고 실제 open 시 existing_file을 검사합니다.
2. `terminal-session.tsx`→`editor-opener-bridge.ts`→`editor-area.tsx`의 실제 subscriber는 파일 소유 프로젝트가 아니라 현재 focused editor의 project/window/pane에 preview open을 요청합니다. `layout.query.ts`의 실제 open 반환 tab ID에만 `reveal-registry.ts`의 5000ms 요청을 적용합니다. 같은 경로의 첫 view를 골라 이동하지 않습니다.
3. 설치된 Monaco `textModel.js::_validatePosition`은 1-based line/UTF-16 column, NaN→1, floor, line 범위 밖이면 첫/마지막 위치, column 상한, surrogate pair 내부를 앞 경계로 보정합니다. Ropey 1.6.1 `RopeSlice::utf16_cu_to_char`의 내부 code unit floor와 egui/epaint 0.36.2 `Galley::pos_from_cursor`, 실제 readonly caret/IME 출력 조건을 고정 source에서 확인했습니다. 기존 LSP `position_to_byte`의 surrogate 내부 거절은 변경하지 않았습니다.

## 구현 계약

1. View마다 `(cwd, trimmed row text)`를 key로 256개 FIFO cache를 둡니다. null도 hit이며 조회는 FIFO 순서를 바꾸지 않습니다. 한 View의 승인 요청은 하나만 pending이고, 실패·잘못된 응답 개수·host queue 제출 실패는 cache로 저장하지 않고 retry를 허용합니다. ID/key가 다른 reply는 pending을 지우지 않습니다. 동일 행 text를 다른 grid 행에서 재사용할 때 현재 cell range를 다시 계산합니다. URL/유효 OSC8 우선순위는 유지합니다.
2. 승인 요청은 opaque Weak viewport/pane/tab/session owner와 typed source를 같은 64개 bounded host queue에 전달합니다. tracked blocking worker가 owner·열린 project·Hub의 Running session·active source tab을 검사한 뒤 shared resolver를 실행합니다. 결과는 같은 mount/session/pending 요청에만 저장합니다. 숨김·취소·session 교체는 owner와 cache를 해제합니다.
3. Alt/플랫폼 Mod gate와 press/release의 range/value 동등성을 유지한 File command는 owned mutation guard와 blocking worker에서 canonical open-project/CLI guard·existing_file을 재검사합니다. 현재 source project의 해당 창 focused pane에 open하고 preview 설정을 반영하며 shared open_tab이 반환한 실제 tab ID를 reply로 전달합니다. 파일 소유 프로젝트는 문서 admission에만 쓰고 UI 목적지 프로젝트로 바꾸지 않습니다.
4. Reveal은 destination tab/path/viewport·열린 layout에 묶이고 같은 tab의 최신 요청이 이전 요청을 대체합니다. 5초 만료·닫힌 tab/project·shutdown을 제거하며 App은 늦은 reply의 과거 layout 대신 현재 state layout을 controller에 전달합니다. 성공적인 open 뒤 terminal이 unmount되어도 destination reveal은 source Weak에 묶어 거절하지 않습니다. Disabled AppSurfaces는 요청을 소비하지 않으며 view attach 뒤 editor 이동·focus에 연결합니다.
5. Core 좌표 보정은 해당 view의 단일 collapsed selection과 composition만 바꾸고 문서 revision/dirty/undo·다른 view를 바꾸지 않습니다. NativeEditor는 동일 gutter/galley를 사용해 수평 caret을 표시하고 행을 중앙으로 이동하며 readonly 이동을 허용합니다. UI focus는 reveal을 소비한 실제 view에 요청합니다.

## 실제 검사와 정정

Cargo 공통은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 직렬 실행했고 동일 변경 상태의 성공은 반복하지 않았습니다.

- [x] Editor `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test editing reveal은` — 1 PASS, suite 0.00초·compile 2.23초. CRLF/astral/NFD/CJK·NaN/floor/infinity/line bounds·surrogate 내부 보정, 다른 view composition/selection·문서·undo 보존입니다.
- [x] UI `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test editor_surface reveal은` — 수정 후 1 PASS, suite 0.03초·compile 0.58초. 5만 행/4만 번째 대상·긴 행의 수평 caret·중앙 표시·readonly focus·다른 view/문서 보존입니다. 첫 fixture는 readonly surface에서 IMEOutput을 unwrap해 실패했고, texture delta를 지우기 전에 panic하여 destructor에서도 abort했습니다. 실제 source의 readonly IME 미출력 계약에 맞춰 delta를 먼저 회수하고 실제 painted caret을 검사했으며 제품 IME 정책을 검사에 맞춰 바꾸지 않았습니다.
- [x] App `--test terminal-file-links` — 2 PASS, suite 0.09초·compile 8.70초. null/FIFO/hit 미갱신·cwd 분리·단일 pending/실패 retry/cancel·stale ID/key·현재 행 범위 재계산·OSC8 우선입니다.
- [x] App `--test editor-reveal` — 1 PASS, suite 0.00초·compile 5.26초. 실제 tab ID/path/viewport·최신 대체·5초 정확한 경계·소비 1회·tab/project close·clear입니다. 시각은 Instant 인수로 결정적으로 검사하고 5초를 실제로 기다리지 않았습니다.
- [x] App `--test terminal-host file_link는` — 1 PASS, suite 0.47초·compile 4.66초. 실제 합성 PTY의 wide/astral/NFD prefix와 파일 suffix 출력→실제 surface hover/Alt click→worker 승인/nonexistent null→생성 후 null cache 유지→현재 project의 다른 focused pane/기존 tab ID dedup→preview 설정 false/true→다른 소유 프로젝트의 문서 admission→해당 canonical view UTF-16 이동/focus·다른 view/본문/dirty 보존입니다. 외부 root 거절/layout 불변·hidden Weak의 open/resolve/late cache reply 거절·성공 open 이후 source unmount와 destination reveal 분리·continue/exit Some(0)·host/Hub join·task 0을 확인했습니다. AppSurfaces 자체를 실제 제품 창에서 실행한 증거는 아닙니다.
- [x] 공통 승인/파일 cache 배선 영향 App `--test terminal-host terminal_links는` — 1 PASS, suite 0.26초·compile 0.21초. 원래 URL/OSC8의 actual PTY/egui/mock platform·scheme/userinfo/control·active/hidden gate를 유지했습니다. 이전 상태의 동일 성공을 반복한 것이 아니라 공통 handler 변경의 회귀입니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml ... --lib --test terminal-file-links --test editor-reveal --test terminal-host -- -D warnings` exit 0, 3.10초. UI `--lib --test editor_surface` strict exit 0, 4.36초·editor `--lib --test editing` strict exit 0, 0.43초입니다. Wry의 기존 dependency warning 17개는 새 authored warning이 아닙니다.

처음 App check는 metadata.cwd()가 Option이 아니라 String인 실제 API를 확인하고 empty-filtered Some으로 fallback을 수정한 뒤 exit 0(1.70초)였습니다. PTY fixture의 Exited 값은 i32가 아니라 Option<i32>인 실제 계약으로 정정한 뒤 위 신규 검사가 통과했습니다. Regex/OSC8/기존 메뉴/입력의 불변 성공은 기존 QA에서 재사용합니다.

Authored Rust 15개 파일의 exact `rustfmt --edition 2024 --config skip_children=true --check`와 tracked `git diff --check`는 exit 0입니다. 관련 QA 2개 Prettier는 exit 0·unchanged이며 untracked Rust/신규 QA 16개의 `git diff --no-index --check /dev/null <파일>`은 모두 whitespace 출력 없이 exit 1(내용 차이)입니다. 검사 범위를 기존 전체 vendor나 다른 완료 기능으로 넓히지 않았습니다.

## 남은 경계

- [ ] 실제 OSC7 변경→hover의 live cwd, active/project/session 변경과 remount/보조 창, host queue-full 후 App submit retry, 느린 worker 중 shutdown/close/root 변경, 사용자 파일 삭제/rename와 preview 대체의 전체 matrix를 직접 연결 검증합니다. 지금 cwd 교체는 cache unit으로만 확인했고 headless host는 초기 실제 cwd를 확인했습니다.
- [ ] 실제 AppSurfaces의 loading→attach→reveal consume→focus frame과 persistent Monaco view-state/fold 복원 이후의 순서, focus 중 IME composition interruption/late commit을 확인합니다. Core composition clear와 readonly caret 검사는 실제 OS CJK IME/AX 완료 근거가 아닙니다. 원본 view-state 복원 전체가 이미 구현됐다고 주장하지 않습니다.
- [ ] 행/URI/cwd 64KiB와 URL window 256KiB·256행 cache의 초대형 TS 호환성, 1 pending/View의 hover 응답성, cloned row/path/regex의 aggregate byte/peak/CPU/RSS·다중 창을 확인합니다. 구조적 개수 상한을 실제 memory/performance 합격으로 계산하지 않습니다.
- [ ] 전체 URL/file provider cell/wrap/button/mouse/keyboard·locale/theme/DPI/픽셀/실제 OS window를 마무리합니다. 다음 구현은 남은 file lifecycle·같은 parser의 OSC133 marker입니다. N4-C 및 M8 N1~N8은 0/8 완료 상태를 유지하며 213 view/full cutover/TS 제거·최종 Git은 아직입니다.
