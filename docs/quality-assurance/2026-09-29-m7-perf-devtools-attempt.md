# M7 계측용 release 앱 실행 진단

## 대상

- `target/release/bundle/macos/TAIDE.app`: 별도 identifier의 로컬 계측 산출물
- `target/debug/bundle/macos/TAIDE.app`: 같은 코드의 격리 debug 대조 실행
- `docs/quality-assurance/2026-09-04-perf-baseline.md`: release 3회 중앙값 절차

## 관찰

- Tauri의 `devtools` Cargo 기능을 지정한 `CARGO_NET_OFFLINE=true bun run tauri build --bundles app --no-sign --features tauri/devtools --config '{"identifier":"net.gumyo.taide.m7perf.0ee5f27e64f94daa82e4789556697b3c"}'`가 exit 0으로 `.app`을 만들었습니다. `Info.plist` identifier가 지정값과 일치했고, 로컬 ad hoc 서명 뒤 `codesign --verify --deep --strict`도 exit 0입니다. 이 산출물은 배포 서명·공증을 받은 제품 빌드가 아닙니다.
- 앞서 격리 debug 앱에서 `TAIDE_PERF=1`로 WebKit Inspector를 열었고 `await window.__TAURI_INTERNALS__.invoke('perf_snapshot')`가 `enabled: true`와 Rust entries/counters를 반환했습니다. 앱 `⌘Q`는 exit 0이었고 IDE listener 57040과 격리 lockfile이 사라졌습니다. debug 수치는 release 기준선에 넣지 않았습니다.
- 계측용 release 실행 파일 직접 기동은 두 번 모두 창 표시 전 exit 134였습니다. 해당 macOS crash report의 예외는 `SIGABRT`, 프레임은 `___RegisterApplication_block_invoke`와 `NSApplication init`입니다. 앱 코드의 특정 원인이라고 판정하지 않습니다.
- 기본 sandbox의 `open -n`은 번들 안에 실행 파일이 있음에도 Launch Services `kLSNoExecutableErr`를 반환했습니다. 새 임시 경로로 복사한 같은 번들도 기본 sandbox에서 동일했습니다. 화면 제어 도구로 새 경로 앱을 선택한 호출은 타임아웃됐고, 당시 프로세스는 IDE listener를 열었지만 화면 제어 목록에 창이 없었습니다.
- 권한 허용 `open -n -a`는 exit 0으로 임시 번들을 기동했지만 다시 창이 나타나지 않았습니다. 정확한 산출물 경로·PID를 확인하고 창 없이 남은 두 테스트 프로세스에 각각 `SIGTERM`을 보냈으며 IDE listener 31053·58798이 사라졌습니다. 첫 프로세스의 격리 lockfile도 없었습니다.

## 판정

동일한 release Inspector 실행 가정으로 반복하지 않습니다. debug Inspector가 작동한다는 사실과 release 프로세스가 정상 GUI·성능 스냅샷에 도달했다는 사실은 다릅니다. `docs/quality-assurance/2026-09-04-perf-baseline.md`의 동일 fixture·3회 중앙값 및 M7-C4c-2는 미완료입니다. 제품 코드 변경이나 성능 회귀 판단의 근거로 이 시도를 사용하지 않습니다.
