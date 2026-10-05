# M8 격리 native terminal core 기본 경계

## 대상·상태

`native/taide-native-terminal/{Cargo.toml,Cargo.lock,src/lib.rs,tests/core.rs}`입니다. 앞서 조사/검사한 Alacritty 0.26.0과 bounded OSC/StreamObserver를 가진 기존 vte 0.15.0을 격리 typed core에서 재사용했습니다. 기존 spike를 제품 구현으로 이름만 바꾸지 않고 가변 크기·borrowed grid/content·mode/damage·effect admission과 실패 수명을 연결했습니다. 실제 PTY/NativeApplication/renderer/normalized text/input·전체 terminal/M8는 미완료입니다.

메인이 workflow·서브에이전트 없이 수행했습니다. 제품 taide-terminal/scanner/PTY·root Cargo/MSRV·native app의 dependency graph·사용자 실기 bundle·앱·데이터·시스템 설정은 이 core 단위에서 변경하지 않았습니다. 기존 media/HTML 및 parser/PTY 성공은 재사용하고 반복 실행하지 않았습니다.

## source/API 근거와 기본 계약

Alacritty 설치 source `term/{mod,cell}.rs`, `grid/mod.rs`, `event.rs`의 Dimensions/Term::new/resize/renderable_content/grid/mode/damage/reset_damage/scroll_display와 EventListener/Osc52·Cell/Hyperlink를 직접 읽었습니다. 기존 vte `src/{lib,ansi}.rs`, `TAIDE-CHANGES.md`의 단일 ANSI parser·OSC 원형/StreamObserver·동기화 buffer/stop_sync를 확인했습니다. 기존 scanner의 공개 `classify_osc_payload`와 ScanEvent를 사용해 OSC 정책을 새로 복제하지 않습니다.

- Core는 Term·Processor를 하나씩 소유합니다. renderer는 수명이 묶인 read-only borrowed grid/RenderableContent를 사용하며 전체 history를 매 frame 복사한 snapshot을 만들지 않습니다. resize와 primary/alternate·history·mode·damage가 같은 Term에서 나옵니다. 두 창의 실제 공유 owner/view 정책은 아직 구현하지 않았습니다.
- 생성/resize 전에 두 화면+설정 history의 cell 기본 크기를 checked arithmetic으로 검사합니다. 기본 grid admission 128MiB, feed 64KiB, 논리 effect payload+inline 합산 64KiB/256개입니다. 작은 변경은 주입 Limits로 검사합니다. 이 값은 private cell extra/Arc/resize buffer/parser/title/allocator/RSS를 합한 전체 retained 메모리 상한이 아닙니다.
- feed oversize/잘못된 size는 파싱/변경 전에 명시적으로 거절합니다. effect overflow는 pending을 폐기하고 Term을 retire해 partial effects/화면을 성공으로 전달하지 않습니다. 이후 read/feed도 오류입니다. 실제 PTY consumer는 오류를 backpressure/종료 상태로 보여야 하며, 현재 consumer가 없어 무손실 전송 전체 완료를 주장하지 않습니다.
- OSC Cwd/command/notification/agent 효과는 같은 parser observer의 완결된 raw payload로만 분류합니다. 별도 raw scanner를 만들지 않았습니다. OSC52는 Disabled이고 Listener도 ClipboardLoad/Store를 밖으로 전달하지 않습니다. native OS clipboard port를 호출하지 않습니다.
- 제목은 Term의 Title 이벤트를 기존 크기/제어문자 정책으로 분류합니다. observer 제목은 중복하지 않습니다. 이 경계는 OSC 0/2뿐 아니라 CSI title stack pop의 Title과 ResetTitle도 처리합니다. 처음 단순히 Title 이벤트를 무시하는 초안은 source review에서 stack 복원 누락이 확인돼 수정했으며 별도 새 test로 확인했습니다. raw observer 중복 회피를 위해 stack title까지 버리지 않습니다.
- PtyWrite/Bell/색상·크기 query 등의 toolkit 이벤트는 typed Effect로 소유권을 전달합니다. renderer 깨움/마우스 커서/blink는 caller의 bytes/damage/render 주기에서 처리할 경계이며 현재 GUI 연결은 없습니다. 이벤트 formatter/색상·크기 회신의 실제 PTY 순서와 소비자 정책은 다음 gate입니다.

upstream dependency license는 Alacritty Apache-2.0, 기존 vte MIT/Apache-2.0입니다. native core 본문은 MIT이고 upstream code/license를 MIT-only로 바꾸지 않습니다. 기존 vte path patch를 이 독립 Cargo root에만 연결했습니다. 첫 offline resolution은 270 packages를 현재 Rust 1.95-compatible로 lock했고 이후 locked/offline입니다. transitive version이 root/native app과 모두 같다고 주장하지 않습니다. 예를 들어 이 독립 graph의 rustix는 1.1.5이며 media app의 1.1.4를 바꾸지 않았습니다. 제품 의존성 채택·MSRV/배포 license 확정은 N1/N7입니다.

## 실행 증거

Cargo 직렬, `--manifest-path native/taide-native-terminal/Cargo.toml --target-dir experiments/native-shell-spike/target`입니다. 첫 새 root lock resolution만 `--offline`, 이후 `--locked --offline`입니다.

- [x] 신규 `cargo test … --test core -- --nocapture`: 1 PASS, compile 0.41초/suite 0.00초입니다. whole/byte-split Unicode+combining/SGR/wide spacer/OSC8 cell과 title/cwd/DSR reply/bell 순서·OSC52 거절·alternate restore/모드·history/scroll/borrowed view/resize/damage·feed/size/산술 overflow·effect count/bytes 초과의 core retire·sync flush를 연속 확인했습니다. 최초 10.52초 compile 뒤 runtime FAIL은 alternate 진입이 cursor home이라는 fixture 가정이었습니다. upstream swap_alt가 현재 primary cursor를 복사함을 읽고 명시적 CSI H를 fixture에 넣었습니다. 구현을 임의 home으로 바꾸지 않았습니다. 첫 apply_patch는 rustfmt의 실제 한 줄과 달라 거절됐으며 파일/재검사 실행 없이 실제 줄을 확인해 수정했습니다.
- [x] 신규 `cargo test … --test core 제목은 -- --nocapture`: 1 PASS, compile 0.74초/suite 0.00초입니다. OSC title 1회·push/change/pop 복원·이후 cwd 순서·oversized title 거절·None 제목 ResetTitle을 확인했습니다. 일반 core의 이미 성공한 runtime body는 다시 실행하지 않았습니다. test 이름의 OSC 대문자 snake-case 경고는 이름만 소문자로 고친 뒤 정적으로 확인했고 body를 다시 실행하지 않았습니다.
- [x] 초기 core `cargo clippy … --lib --test core -- -D warnings` exit 0(5.11초), 제목 변경 후 같은 영향 target strict exit 0(0.39초)입니다. static 결과는 서로 다른 code 상태입니다.
- [x] 정확한 두 Rust 파일 `rustfmt --edition 2024 --check` exit 0입니다. source/fmt/구현/검증 근거만 기록하고 실제 OS shell·PTY/GUI 실행은 하지 않았습니다.

## 다음 필수 구현·검증

- [ ] 단일 parser normalized text/overlap의 기본 Core 소비는 후속 `2026-10-02-m8-native-terminal-outcome.md`에서 연결·검사했습니다. 실제 effects/text consumer와 기존 metadata/agent 이벤트 연계, writer ordering·resize/query reply·raw ring/snapshot/live attach·pause/admission/close·정상/오류 종료 회수는 미완료입니다. 아직 제품 OutputScanner를 제거하지 않습니다.
- [ ] 동적 cell extra/zerowidth/Hyperlink/Arc·inactive/history/resize/title/parser buffer·effect Vec capacity를 포함한 실제 retained 계산/aggregate·CPU budget입니다. 현재 grid admission은 기본 cell 산술일 뿐입니다. 악성 긴 결합 문자·title stack·flood의 무한 session/전체 RSS가 안전하다는 증거는 없습니다. private 그래프를 소비하기 전에 상한을 구현하고 검사합니다.
- [ ] NativeApplication의 Terminal placeholder를 실제 shell profile/PTY/core/native surface·설정/theme/font/input/IME·selection/search/link/OSC·CJK/AX·multi-window·다중 session과 연결합니다. 기존 TS view가 기준이고 사용자 담당 IME/VoiceOver·실제 OS/GPU/성능은 최후 순위입니다.
- [ ] upstream raw trace/title·unhandled payload log의 개인정보 경계, 외부 link/clipboard/notification/query 정책, codec가 아닌 terminal parser/display corpus·장기 안정성과 최종 license/package/MSRV·전체 N4/N1~N8입니다. core를 native app에 연결하기 전에 unresolved boundary를 덮습니다.
