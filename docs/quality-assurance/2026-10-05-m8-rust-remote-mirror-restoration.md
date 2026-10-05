# M8 실제 원격 미러 조회·초안 복원

## 결과·범위

BrowserEditor가 실제 ShellSnapshot의 ProjectRef를 받는 bind_project_file, 프로젝트별 typed file_list_mirrors, 같은 core의 늦은 초안 복원을 소유합니다. 파일 열기는 미러를 기다리지 않습니다. 복원은 첫 파일 admission의 revision/clean 상태가 유지된 경우에만 적용하며 이미 편집/저장/선택한 문서나 기존 공유 모델을 덮지 않습니다. 실제 복원은 Restored 사건으로 같은 dirty 소유자에 연결됩니다.

MirrorState4·실제 FileViews/core 복원2·새 Chrome-Wasm 연속1 고유7검사 PASS입니다. 성공한 검사는 반복하지 않았습니다. mirror write/epoch/지연·종료 flush와 조건부 정리, missing-source Save As UI, 전체 App/canvas/자산/최종 게이트는 미완료입니다. 후속47 2/4(50%)·전체363/433(83.83%, 비가중)·최종0/8·전체 ETA 산정 보류·goal active·전체 완료 전 Git 없음·main 직접입니다.

## 대상·원본

- `native/taide-remote-web/src/mirrors.rs`: 프로젝트별 DTO cache/dirty/pending validity·late read 폐기·해제 후 재바인딩·오류/Closed·명시 retry/읽기 reconnect를 구현했습니다. invalidated pending은 응답을 소비하지만 새 cache에 적용하지 않습니다. source_missing도 실제 MirrorEntry로 보존하며 빈 가짜 문서를 만들지 않습니다. 일반 fs:changed는 미러를 버리지 않고 fs:rescan-required만 전체 조회를 무효화합니다. 실제 layout의 auxiliary slot 소실은 같은 길이의 다른 창 추가와 함께 발생해도 미러를 재조회합니다.
- `src/mirrors.rs::is_within_root`: 원본 `src/shared/lib/path-root.ts`의 backslash/dot/dotdot·drive anchor/대소문자·prefix 경계를 그대로 옮겼습니다. 이 판정은 UI의 미러 적용 여부이지 권한 검사가 아닙니다. 기존 원본처럼 root `/`의 하위 경로 판정은 false이며 임의 기능 변경으로 고치지 않았습니다. 서버의 canonical/root 권한 검사는 선행 실제 remote backend 그대로입니다.
- `native/taide-native-editor/src/store.rs::restore_file_draft_if_unchanged`: 파일 identity/revision/clean·용량/overflow를 먼저 확인하고 baseline/observed disk/readonly metadata를 보존한 채 초안을 적용합니다. revision을 올리고 requires_save를 설정하며 모든 view의 선택/composition/fold와 history/syntax 상태를 정리합니다. 디스크 I/O나 browser 전역을 core에 반입하지 않았습니다.
- `native/taide-remote-web/src/{files.rs,browser-editor.rs,lib.rs}`: 첫 fresh admission의 revision을 보유합니다. 늦은 복원·saved/중간 pending choice/save 제한·restore notice/Restored 사건, 실제 scope/query seq 응답 소유권·Drop/dispose·조회/복원 오류·명시 재시도를 연결합니다. 실제 snapshot에서 닫힌 project가 확인되면 scope를 회수하며 layout 조회 중 snapshot이 None인 것만으로 scope를 폐기하지 않습니다. raw core/files_mut caller는 이 scope를 자동 추측하지 않으며 실제 App은 typed project binding을 사용해야 합니다.

원본 use-editor-file-persistence의 file-first/late restore·live model 우선·dirty/restoreNotice와 ipc-sync-provider의 일반 watcher 제외·rescan 재조회·auxiliary window 소실 비교를 읽었습니다. 새로운 제품 TS/의존성/manifest/lock/MSRV/OS/보호 앱은 변경하지 않았습니다. test-only FileProbe/HTML/tool은 제품 App이나 JS fallback이 아닙니다.

## 최소 검증

환경은 이전 QA와 같으며 Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 같은 경로의 bin/cargo, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`로 직렬 실행했습니다.

- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test mirrors`: 새3 첫 PASS(build2.89초/suite0.00초)입니다. 중복 request/coalesce·invalidated read·해제/재요청의 옛 응답·typed source_missing·Closed/reconnect·binary/malformed/remote 오류/명시 retry·원본 lexical path 경계를 확인했습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test files 늦은_미러_복원은_초기_revision_공유_view_저장_편집과_용량을_보호한다 -- --exact`: 시험의 undo→자동 clean 가정이 틀려 첫 실패(build0.44초/suite0.00초)였습니다. 기존 코어의 undo는 requires_save를 유지합니다. 기존 mark_saved로 실제 clean이지만 revision이 변한 상태를 만들어 시험만 정정한 뒤 해당1 PASS(build0.35초/suite0.00초)입니다. file-first/두 view·dirty/revision1/Restored 사건·재복원 없음·disk baseline·수정된 clean·save 완료·용량 실패·명시 retry/readonly metadata를 확인했습니다. patch context 불일치는 파일을 다시 읽어 정정했으며 제품 오류로 세지 않습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test files 미러_복원은_진행중_저장과_디스크_선택을_침범하지_않는다 -- --exact`: 새 RED(Ok(true)/기대 Ok(false), build0.41초/suite0.00초)→pending save/choice guard 추가→실패1 GREEN(build0.60초/suite0.00초)입니다. queued/in-flight choice·pending/완료 save 동안 초안 복원을 하지 않습니다.
- [x] `cargo test --manifest-path native/taide-remote-web/Cargo.toml --test mirrors 미러_조회는_보조창_실제_소실과_rescan만_재조회하고_일반_watcher는_보존한다 -- --exact`: 새1 첫 PASS(build0.99초/suite0.00초)입니다. same-length slot 교체에서도 실제 소실을 인식하고, 추가 소실 중의 옛 query 오류를 버리며 일반 watcher 보존/rescan 무효화를 확인했습니다. 앞서 통과한3은 실행하지 않았습니다.
- [x] production Wasm strict 최종0.19초·remote lib/mirrors/files strict0.36초·core 직접 lib strict2.58초·probe Wasm strict0.45초(`-D warnings`) exit0입니다. 시험 slice 생성의 불필요한 clone3곳은 from_ref로 정정했으며 성공한 동작 검사는 반복하지 않았습니다. probe build3.47초/CLI0.2.129로 file-built 재생성·tool strict TypeScript·Rust8 exactfmt/tool·HTML Prettier를 확인했습니다.

## 실제 Chrome-Wasm 연속 결과

`bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built mirror-restore` 새1이 첫 실행 PASS했습니다. 승인된 격리 synthetic localhost·새 headless Chrome/mock keychain·SW 차단/download off 환경이며 사용자 앱/설정/키체인에 접근하지 않았습니다. 정본은 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/mirror-file-result.json`입니다.

실제 project_list/layout_get→ShellSnapshot의 project로 두 file scope를 바인딩했습니다. 미러 응답이 지연돼도 file_open의 disk/clean/document1/view2가 먼저 실제 renderer에 표시됩니다. 응답을 풀자 같은 core가 recovered draft/dirty=true·복원1/실제 restored conflict 문구를 표시하고 같은 소유자의 dirty=true 요청/ack가 완료됐습니다. 수동 dirtyFlushes는0입니다.

실제 Text 입력 뒤 typed recovered draft를 만든 다음 미러 query를 지연하고 두 view를 해제·재바인딩했습니다. 옛 obsolete draft 응답은 폐기되고 새 latest draft cache만 들어왔습니다. 실제 문서는 여전히 typed recovered draft/dirty=true/document1/view2·복원 횟수1입니다. 재attach한 dirty 공유 모델에는 새 restore notice를 붙이지 않습니다.

서버 seq1~12 고유·file_open1/mirror query3/dirty1/save0·socket0/connected=false·quiet1.1초 wake13/요청/추가 연결 불변·page error/response 누출/복원 오류0입니다. 실제 egui Text shape이지 canvas 픽셀/전체 GUI 검사라고 주장하지 않습니다.

이 runtime source/artifact는 pending guard·auxiliary/rescan 후속 추가 전입니다. 해당 후속은 각 새 pure 검사와 최종 Wasm/core/probe strict로 확인했고 이 초기 조회·복원의 동일 성공 시나리오는 재실행하지 않았습니다. 선행 result 파일은 보존했으며 같은 file-built의 초기 mirror runtime binding과 이후 source를 구분합니다. 서버/Cargo/browser live handle은 남지 않았습니다.

## 남은 경계

- [ ] 실제 mirror write/epoch·500ms debounce/강제 flush·성공/실패/Closed·같은 최신 draft·저장/선택 이후 조건부 정리를 연결합니다. 현재 root mirror_dirty 응답은 baseline Option<f64>뿐이므로 이전 쓰기만 정확하게 취소하려면 서버의 해당 쓰기 receipt가 필요합니다. 클라이언트의 추정 savedAtMs로 CAS를 만들지 않습니다.
- [ ] missing source의 실제 복구/Save As UI·문서 및 project scope 종료·전체 query cache 수명/원본 rescan throttle·실제 보조 창 닫힘/closed-project browser 시나리오를 전체 App 게이트에서 확인합니다. pure auxiliary 비교를 창 실기로 세지 않습니다.
- [ ] actual App/canvas/모든 소비자·자동 저장/LSP/포맷 참여자·Rust public assets/package·성능/메모리/beta/cutover/Rust99%를 완료합니다. CJK·VoiceOver 실기는 사용자-last입니다.
