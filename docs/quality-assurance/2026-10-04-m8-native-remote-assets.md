# M8 원격 공개 자산 로더

## 2026-10-05 Rust 브라우저 통신 기반

공유 wire/client·실제 WebSocket adapter·Wasm target/공식 CLI 준비와 격리 Chrome의 단일 연속 검증은 `2026-10-05-m8-rust-remote-browser-transport.md`가 정본입니다. core4/server2/browser1 PASS와 실제 Wasm-target compile/strict를 확인했지만 renderer·모든 UI/consumer·제품 bundle manifest 생성/패키징은 여전히 미완료입니다. test-only browser-probe/generated binding/Wasm은 Catalog의 제품 remote-public에 넣지 않았습니다. 아래 test8바이트 module과 이번 실제 transport probe 모두 제품 UI/fallback이 아닙니다.

## 2026-10-05 실행 파일 번들 계약

Catalog.packaged는 실제 executable 기준 macOS `.app/Contents/Resources/remote-public`, 비번들 executable 인접 `remote-public`을 사용합니다. 실행 인자나 사용자 data-dir로 자산 root를 바꾸지 않으며 legacy TS dist fallback은 없습니다. manifest/wasm 이름을 실제 빌드가 제공해야 하고 미존재·불일치 시 로딩 실패로 원격 listen을 거절합니다.

`bundle-manifest.json`은 `format: taide-rust-remote-v1`, `entryWasm`, `files`만 받습니다. manifest64KiB·공개 payload64MiB·파일1024개를 각각 제한하고 기존 root/entry path·NOFOLLOW·정규 파일·Stamp·원자적 frozen resolver를 재사용합니다. index.html 필수·중복/미지원 파일/자기 metadata 공개·entry 미등록/비-wasm·unknown field·format 불일치를 거절합니다. metadata와 공개 파일을 같은 descriptor Anchor로 읽고 entry의 WebAssembly v1 header를 확인합니다. metadata는 resolver에 포함하지 않습니다.

header와 manifest 표기는 **실제 Rust 구현/전체 wasm 검증/GUI 기능의 증명이 아닙니다.** 합성8바이트 module은 loader/수명 검사용이며 제품 UI나 fallback 자산으로 배포하지 않습니다. 실제 Rust 원격 UI 생성/컴파일/패키징·전체 frontend 기능과 CSP/browser 실행은 아직 미완료입니다. wasm32-unknown-unknown target lib는 현재 설치되지 않은 상태를 readonly test로 확인했습니다. 기존 frontend/toolchain을 조용히 대신 사용하지 않습니다.

1. `cargo test --lib remote_assets::tests::`: compile12.68초/suite0.02초·새 bundle1/변경된 fixed loader 영향1 PASS입니다. packaged 경로/잘못된 executable root·미존재 manifest/unknown/legacy/entry/path/중복/metadata 공개·metadata sparse 과대/비-wasm/symlink 거절·정상 manifest exact bytes·metadata 비공개/task0을 확인했습니다. 기존 fixed manifest의 사전 거절·HTTP/frozen/보안 범위도 변경 영향을 확인했습니다.
2. 현재 lib binary의 prepare_tests2 exact: 2 PASS/suite0.02초입니다. Source 구분 변경 뒤 취소된 waiter/직렬 cache·startup/settings 자산 실패 시 listen 없음/성공 후 시작을 확인했습니다. loopback 권한만 승격했으며 이전 App 생성/Exit 성공을 반복하지 않았습니다.
3. 최종 native lib/bin/tests clippy `-- -D warnings`: exit0/3.94초·authored5 exact rustfmt/check·tracked diff/check exit0입니다. 미추적 파일 whitespace 검사는 별도로 기록합니다. Wry17 dependency warnings와 큰 debug lib-test binary의 ld `__eh_frame` 경고는 authored clippy와 구분하며 build flag/검사기를 끄지 않았습니다.

Cargo는 locked/offline·CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo·target=/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬입니다. 프로덕션 원격 byte queue 정책은 그대로256-frame이며 새64MiB는 정적 자산 payload quota이지 그 queue 상한이나 RSS 보증이 아닙니다. [Serde의 strict container 계약](https://serde.rs/attributes.html)·[Rust Path의 parent 계약](https://doc.rust-lang.org/std/path/struct.Path.html#method.parent)과 실제 toolkit source를 확인했습니다.

## 대상과 결과

대상은 `native/taide-native-app/src/remote-assets.rs`, `remote-assets-tests.rs`, `lib.rs`입니다. 명시적 공개 manifest를 감독된 blocking 작업에서 모두 읽은 후 immutable resolver로 반환합니다. 요청 시 filesystem 접근·임의 경로 fallback은 없습니다. 기존 Anchor/Stamp의 descriptor 기반 regular-file·NOFOLLOW·root identity 검사를 재사용하며 변경 감지나 일부 파일 실패 시 전체 catalog가 실패합니다.

caller가 정한 count·payload byte quota를 할당 전에 검사합니다. 이 상한은 payload 합계이며 allocator/RSS·manifest 문자열·HTTP 응답 복제·WebSocket 바이트 상한을 의미하지 않습니다. 기존 256-frame WebSocket 정책은 변경하지 않았습니다. index.html을 요구하고 MIME whitelist·중복·비공개 namespace·비정상 경로를 거절합니다. legacy TS dist를 읽거나 생산 fallback으로 추가하지 않았습니다.

## 최소 검증

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib remote_assets::tests`: 고유1 PASS, compile9.17초/suite0.01초, filtered210입니다. 최초 fixture의 EventSink closure 타입 오류를 명시적 Sink 구현으로 수정한 뒤 실행했습니다.
- [x] 같은 합성 검사에서 exact manifest/MIME·quota·중복·index 필수·invalid manifest의 파일 읽기 이전 거절·symlink/missing file 실패·원본 변경 및 삭제 후 고정된 bytes·기존 HTTP static handler의 index fallback/CSP/no-store·owner 해제·shutdown/task0을 확인했습니다. 실제 인증 router/WS 검사는 앞선 성공을 재사용하고 반복하지 않았습니다.
- [x] native lib/bin/tests clippy `-- -D warnings`: exit0·13.77초입니다. Wry dependency17 warnings는 authored strict와 구분합니다. Cargo handles86122/53939/92385는 모두 종료했습니다.
- [x] authored3 exact rustfmt와 대상 문서 Prettier 완료, tracked diff check 출력 없음(exit0), 새 Rust2개 no-index whitespace check 출력 없음(exit1은 신규 diff)입니다.

UUID 임시 합성 자산만 생성했고 사용자 파일·시크릿·OS 앱·보호 bundle에 접근하지 않았습니다. 추가 의존성·manifest/lock·제품TS·Git 변경은 없습니다. [Rust Read](https://doc.rust-lang.org/std/io/trait.Read.html#method.read_exact), 기존 preview_web_file Anchor/Stamp와 range_file MIME을 근거로 구현했습니다.

## 미완료

- [ ] Rust 원격 UI 자체·자산 생성/패키징·실제 App caller는 아직 미완료입니다. 이 catalog가 기능 동등한 화면을 제공한다고 주장하지 않습니다.
- [ ] nonUnix Anchor는 기존 fail-closed이며 Windows 구현/실기와 root 교체의 동시 경합·allocation failure 검증은 해당 플랫폼/최종 보안 gate에서 필요합니다.
- [ ] 전체 N1~N8 0/8·App Settings/AppFile·startup/Exit·keybinding RED/PTY remount·실기/성능/배포 gate는 남습니다. 전체 M8 완료 전 commit/push하지 않습니다.
