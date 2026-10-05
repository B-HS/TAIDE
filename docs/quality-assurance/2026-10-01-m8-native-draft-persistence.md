# M8 native 초안 보존·자동 저장 연결

## 대상과 원본

- 원본은 `src/widgets/editor-pane/{use-editor-file-persistence.ts,untitled-pane.tsx}`와 `src/shared/constants/mirror.ts`입니다. 파일 mirror는 500ms debounce, 자동 저장은 설정값이 양수일 때만 수행하며 untitled는 자동 파일 저장을 하지 않습니다.
- 구현은 `native/taide-native-app/src/{persistence,application,host,file_sync}.rs`, 검사는 `native/taide-native-app/tests/persistence.rs`입니다.
- 앞선 충돌 처리 범위는 `2026-10-01-m8-native-conflict-and-file-observation.md`입니다. 기존 root/문서/화면 성공 결과를 다시 실행하지 않았습니다. 의존성·제품 manifest/MSRV·기존 실기 bundle·시스템 설정을 바꾸거나 GUI/OS 포트를 실행하지 않았습니다.

## 코드 연결

1. 실제 editor의 변경 문서를 ID별로 모아 문서별 deadline을 갱신합니다. 각 문서의 최신 Rope snapshot은 worker 제출 시 얻으며, 타이핑 때 전체 문자열을 만들지 않습니다. 탭/프로젝트 표시가 바뀌어도 현재 pane의 새 경로에 이전 초안을 쓰지 않습니다. 같은 canonical 문서의 mirror는 한 번만 in-flight로 유지합니다.
2. 파일/untitled mirror worker는 기존 root guard·operation lease·tracked blocking 작업을 재사용합니다. 현재 프로젝트/root/layout과 snapshot key를 다시 확인하며, source tab이 사라졌거나 저장 generation이 취소됐으면 쓰지 않습니다. 쓰기 직전에만 Rope를 문자열로 만들고 실제 mirror 서비스로 저장합니다. 손실 디코딩을 읽기 전용으로 관찰한 뒤에도 이미 존재하는 초안을 mirror로 보존할 수 있도록, mirror snapshot은 writable save snapshot과 구별합니다.
3. 파일 자동 저장은 기존 `auto_save_delay_ms`를 읽고, untitled/readonly는 제외합니다. 명시적 저장/닫기 저장도 문서별 in-flight 제어를 공유합니다. 실제 disk write와 mirror 삭제가 성공한 같은 root guard 안에서 이전 generation을 취소합니다. 그 뒤 큐에 남은 stale mirror는 파일에 초안을 다시 설치하지 않습니다.
4. 저장 중 새 편집은 live body를 유지합니다. 완료 후 dirty이면 새 generation의 mirror를 즉시 예약하고, 성공한 저장에 한해 다음 자동 저장을 다시 예약합니다. 실제 저장 실패는 disk/mirror를 성공 처리하지 않고 초안을 보존합니다. 큐 제출 실패는 기존 generation과 deadline을 유지하며 in-flight만 해제합니다. 완료된 동일 상태의 mirror 실패는 반복 재시도하지 않고 다음 편집/명시적 작업/종료 flush를 기다립니다.
5. View Disk·untitled 전환·최종 문서 회수·clean 관찰에서 generation을 취소합니다. mirror reply의 이전 generation은 새 in-flight 상태를 바꾸지 않습니다. 파일 삭제 관찰은 자동 저장을 해제해 기존 예약으로 원본을 다시 만들지 않습니다. 종료 시 기존 실제 root mirror flush와 엄격 layout 저장을 유지합니다.
6. 설치된 공식 eframe 0.36.2의 `epi::App::logic`, `native::epi_integration::{update,update_logic_only}`, `native::wgpu_integration` 호출 경계를 읽고 실제 App `logic`으로 파일 이벤트/관찰·reply·저장 deadline·종료 완료 처리를 옮겼습니다. 렌더 함수가 호출되지 않아도 이 비화면 경계가 실행됩니다. GUI 함수/painting은 그 콜백에서 호출하지 않습니다. 숨김 상태의 cached close request가 실패한 종료를 반복 실행하지 않게 latch를 두고, 종료 실패 후 host 재연결 때 버려진 in-flight generation을 취소한 뒤 현재 dirty 문서의 mirror를 다시 예약합니다.

## 검증 결과

공통 인자는 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test persistence`: 신규 2건 통과, 0.04초입니다. `Instant` 값을 직접 이동해 500ms 경계/문서별 debounce/untitled 제외/재편집/in-flight/저장 중 편집/새 generation/stale reply/단일 실패 중단을 확인했습니다. 실제 host의 file/untitled mirror, 성공 저장 직후 queued stale mirror 거절, late draft 보존, 실제 lossy readonly 저장 실패의 disk/mirror 보존, 취소된 untitled write와 tracked worker 0도 확인했습니다.
- [x] 큐 제출 실패의 deadline/generation 보존을 추가한 뒤 영향받은 `--test persistence 문서별_지연` 1건만 재실행해 0.00초 통과했습니다. 실제 worker의 앞선 동일 성공은 재사용했습니다.
- [x] 종료 실패의 재연결용 in-flight 취소/generation 갱신을 추가한 뒤 영향받은 결정적 검사 1건만 0.00초 재확인했습니다. 실제 worker 성공은 그대로 재사용했습니다. 같은 상태를 세 번 검사한 것이 아니며 각각 새 동작을 추가했습니다.
- [x] 앱 lib/bin/persistence strict clippy는 처음 수동 `Default` 구현을 지적했고 derive로 바꿨습니다. 해당 대상 재실행 exit 0, 0.64초입니다. 파일 삭제 관찰의 자동 저장 해제 분기를 포함한 최종 대상 정적 검사도 exit 0, 0.47초입니다.
- [x] 비화면 logic/종료 request latch·재연결 코드까지 포함한 최종 app lib/bin/persistence strict clippy: exit 0, 0.56초입니다. 변경 Rust 파일 rustfmt check와 tracked diff whitespace check도 exit 0입니다.
- [x] 후속 resource 작업에서 취소된 SaveTracked가 이전 경로를 재생성하는 실제 오류를 재현·수정했습니다. 저장 전 generation 검사와 신규 1건 0.01초의 근거는 `docs/bug/2026-10-01-native-cancelled-save-recreated-path.md`에 기록합니다. 최초 mirror 검사 성공을 저장 취소의 증거로 확대하지 않습니다.

## 아직 완료가 아닌 범위

- [ ] deadline은 실제 비화면 `App::logic`에서 제출하고 repaint를 예약하도록 연결했습니다. 공식 backend의 hidden/minimized logic-only 호출은 코드에서 확인했지만 실제 OS에서 500ms mirror/자동 저장과 종료 실패 뒤 재연결을 실행하지 않았습니다. 결정적 deadline/정적 검사를 실기 시간 보장으로 쓰지 않습니다.
- [x] 이 문서의 초기 구현 이후 format/code actions와 `.editorconfig` 우선 trim/final newline·auto cursor 보호를 연결했습니다. 후속 `2026-10-01-m8-native-{save-cleanup,save-actions,lsp-application}.md`의 직접 근거·미완료 범위를 따르며 전체 저장 파이프라인 완료로 세지 않습니다.
- [ ] 이미 열린 파일 삭제 시 missing-source 읽기 전용/Save As 전환과 모든 close/project/window 진입점, 실제 unmount 즉시 flush의 동일 경계는 남습니다. 현재 삭제 관찰은 자동 저장만 해제하고 live body를 유지합니다.
- [x] 초기 core의 undo 뒤 dirty 차이는 후속 save-cleanup 단계에서 명시적 settle까지의 dirty latch로 수정·검사했습니다. 모든 표시/닫기/종료의 실제 GUI 검증은 여전히 남습니다.
- [ ] root guard 밖의 mirror writer/외부 filesystem 변경, byte-budget 기반 큐 메모리/전체 mirror 파싱 전 상한, 실제 OS/UI/접근성/성능·terminal/전체 화면·TS 제거/서명/배포는 남습니다.

N2-A2c5의 최초 비화면 logic 기반 코드 연결과 좁은 검사만 기록합니다. N2-A2c/N2/M8 전체는 미완료이며, IME/VoiceOver와 fallback은 코드 구현 뒤 마지막 순서입니다.
