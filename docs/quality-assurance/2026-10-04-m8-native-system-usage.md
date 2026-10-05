# M8 실제 시스템 사용량과 상세 목록

## 대상과 원본

대상은 `native/taide-native-app/src/system-usage{,-tests,-view,-view-tests}.rs`, `remote-utilities.rs`, `application.rs`, `presentation-refresh.rs`, `lib.rs`, `lsp.rs`, `lsp-status.rs`, `tests/lsp-recovery.rs`, `resources/status/activity.svg`입니다. 원본은 `src/entities/system/system.query.ts`, `src/features/window/status-bar.tsx`, `src/widgets/{window-chrome/status-bar-content,system-usage-modal/system-usage-modal,app-shell/app-shell}.tsx`, `src/shared/ui/dialog.tsx`, `src/app/query-client.ts`와 설치된 TanStack Query의 queryObserver입니다.

App 소유 Sampler가 같은 SystemUsageStore와 기존 root PID/CPU 정규화/descendant/라벨/정렬을 사용하는 typed 수집 함수를 호출합니다. remote JSON dispatch도 이 함수를 재사용합니다. 실제 OS 수집은 기존 TaskSupervisor blocking worker에서 실행되고 UI 프레임은 oneshot.try_recv만 처리합니다. 요약/상세 각각 한 요청만 유지하며 관측 폐기나 설정 off/on에도 진행 중인 OS 작업을 복제하지 않습니다. 루트 종료에서 새 작업을 받지 않으며 이미 시작한 blocking worker는 기존 감독자의 종료 회수 대상입니다. UI가 OS 작업을 동기 실행하거나 취소를 실제 thread 종료로 주장하지 않습니다.

설정 enabled와 모달 open은 독립적입니다. 최초 enable 조회는 비활성 창에서도 실행하고 반복 조회만 창 포커스로 제한합니다. 원본 전역 retry0/refetchOnWindowFocus=false와 3초 주기를 사용합니다. 실패 시 마지막 정상 표시를 유지하고 즉시 재시도하지 않으며 로그는 error.kind만 기록합니다. 설정만 끄면 숨긴 요약 cache를 보존하고, 모달 닫기/상태바 Zen unmount는 해당 cache와 모달을 비웁니다. generation identity는 폐기 요청의 늦은 완료를 다시 표시하지 않으며 종료 후 신규 요청은 없습니다.

Native LspBridge가 실제 registry의 spec.name/PID를 별도 watch로 발행해 기존 terminal/agent/legacy LSP 라벨 뒤에 병합합니다. 이름은 `서버명 · 프로젝트명`이며 프로젝트가 없으면 서버명만 사용합니다. 마지막 문서 회수와 publisher 종료는 라벨을 비우고 provider가 services를 소유하지 않습니다. 같은 서버의 문서마다 PID 행을 복제하지 않습니다.

상태바에 locale·CPU null의 `--`·MB 반올림·11px/12px Activity icon·top tooltip·클릭을 연결했습니다. 상세 모달은 70vh/최대672px·원본 종류 App/Terminal/LSP/Agent/Other·기존 종류 내 memory 정렬·열64/80·전체 label tooltip/축약·빈 목록·닫기 버튼/Escape/배경 dismiss를 구현했습니다. 모달 동안 shell/global keymap은 비활성입니다. 초기/정상/preview 테마 갱신을 같은 Appearance 경로에 연결했습니다. Activity의 공식 Lucide SVG는 white/tint·기존 resvg/usvg·외부 image resolver 없음·DPI cache/1024 raster 상한으로 처리하며 기존 LICENSE-LUCIDE를 유지합니다.

## 실행 근거

공통 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`, flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬입니다.

- [x] 최초 `test --lib system_usage -- --nocapture` compile FAIL(handle39332): 기존 log 대신 tracing 이름을 사용한 코드와 새 LspBridge fixture field·설치된 TexturesDelta의 SmallVec 형식을 정정했습니다. 새 의존성이나 검사 비활성화는 없습니다.
- [x] 해당 필터 재실행(handle28855) compile10.76초/suite0.04초 중 실제감독수집기 검사1 PASS입니다. 초기 비포커스 요청·UI 반환/blocked worker·단일 in-flight·off/on/obsolete 완료·설정 off 중 독립 상세 조회·CPU 최초 null/CPU 정규화/라벨/descendant/정렬·닫기 cache·포커스 반복 게이트·실패 last-good/retry0·종료 후 pending 회수/task0/owner 해제를 확인했습니다. OS process/font enumeration은 합성 provider로 대체했으며 실제 사용자 프로세스 실측이 아닙니다.
- [x] 같은 실행의 UI 검사는 Frame의 중첩 Shape를 놓쳐 FAIL입니다. 중첩을 펼친 UI만 실행(handle32104) compile7.65초/suite0.04초도 아직 fade-in 중인 색상 동일성 fixture로 FAIL입니다. 설치된 Area.fade_in과 기존 keybindings settled.time fixture를 확인해 시간을 명시했으며 제품 모달 애니메이션을 끄지 않았습니다.
- [x] 실패 UI만 `test --lib system_usage_view -- --nocapture` 1 PASS(handle64401), compile3.57초/suite0.05초, filtered222입니다. 실제 pointer press/release 상태 버튼·null/반올림·아이콘 upload·숨김·종류/행 순서·숫자열·672px/70vh frame·empty·닫기 버튼 pointer·Escape를 확인했습니다. 헤더의 번역문은 close 영역을 침범하지 않도록 제한하고 X의 원본 경로 비율도 보존했습니다.
- [x] 변경된 `test --test lsp-recovery -- --nocapture` 1 PASS(handle85058), compile12.91초/suite0.82초입니다. 실제 합성 child의 최초 PID 라벨1행/spec.name·두 문서/서버1개·crash/replay/restarted PID·마지막 문서/종료 라벨 없음/task0/owner 해제를 확인했습니다. 기존 physical recovery가 실제 caller 변경으로 영향받아 해당 검사만 1회 실행했습니다.
- [x] source 대조에서 별도로 확인한 Zen status-bar unmount 경계를 반영한 `test --lib system_usage_zen -- --nocapture` 1 PASS(handle34577), compile7.31초/suite0.00초, filtered223입니다. cache/modal 정리·진행 중 요청 유지·remount의 obsolete 완료 거절을 확인했습니다. 앞선 성공 검사는 반복하지 않았습니다.
- [x] 최종 변경된 native `clippy --lib --bin taide-native-app --tests -- -D warnings` exit0·14.49초(handle5865), authored Rust11 exact rustfmt입니다. 선행 strict는 exit0·14.79초(handle92623)이며 Zen caller 변경의 strict만 재실행했습니다. 기존 Wry dependency17 warnings는 authored strict와 별도입니다.

## 공식 API와 미완료

후속 chord App 통합에서 사용량 모달이 열릴 때 toast와 keybinding editor 입력 gate도 닫았습니다. 이 연결은 `native-status-chord`의 최종 strict 근거로 추적하며 앞선 수집/UI/LSP/Zen 성공은 재사용합니다. 실제 겹친 GUI 입력은 최종 화면 gate에 남습니다.

설치된 egui0.36.2 Modal/ModalResponse/Frame/Area.fade_in/Ui/Texture 및 Tokio oneshot.try_recv 공식 소스를 확인했습니다. [Modal 공식 문서](https://docs.rs/egui/latest/egui/containers/modal/struct.Modal.html), [oneshot Receiver 공식 문서](https://docs.rs/tokio/latest/tokio/sync/oneshot/struct.Receiver.html), [Lucide Activity SVG](https://github.com/lucide-icons/lucide/blob/main/icons/activity.svg)도 확인했습니다. 공식 main SVG를 읽었으며 pinned 기존 binary와 픽셀 동등성을 주장하지 않습니다.

- [ ] 실제 GUI/DPI/다중 창/좁은 창·툴팁/접근성·닫기 opacity/위치·font tabular/uppercase tracking·실제 OS CPU/메모리/foreground 주기는 최종 화면/성능 gate에서 확인합니다. headless geometry를 픽셀 실측으로 계산하지 않습니다.
- [ ] Problems/chord·전체 diagnostics/provider·same-generation handshake/dispose/reacquire·actual App assets/effects/ports owner/Settings/AppFile/IDE 화면·Rust remote UI·keybinding RED/PTY remount는 미완료입니다.
- [ ] N1~N8 0/8·전체 M8은 active이고 최종 TS 제거/배포/commit/push는 미완료입니다. 이 범위는 전체 상태바/앱 완성이 아닙니다.

이번 구현에서 manifest/lock/dependency/root/Tauri/MSRV/제품TS/Git은 변경하지 않았으며 사용자 home·자격 증명·clipboard·실제 앱·보호 bundle·OS 설정/IME/VoiceOver에 접근하지 않았습니다. 합성 테스트는 실행되지 않는 UUID 임시 경로 또는 fixture가 소유한 child/파일만 사용합니다.
