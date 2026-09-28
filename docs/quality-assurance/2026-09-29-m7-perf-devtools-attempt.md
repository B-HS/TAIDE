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

## 후속 권한 경계 확인

같은 debug 번들도 기본 sandbox에서 앱 등록 중 exit 134와 동일한 macOS 프레임을 보였고, 권한 허용 직접 실행에서는 창을 열었습니다. 같은 방식으로 계측용 release 실행 파일을 임시 설정 경로와 `TAIDE_PERF=1`로 직접 실행하자 창과 WebKit Inspector가 열렸으며 `perf_snapshot.enabled`가 `true`였습니다. 기본 sandbox의 창 전 중단을 `devtools` 기능 자체의 결함으로 판정하지 않습니다. 권한 허용 `open -n -a`에서 창이 없던 이유는 별도로 확인하지 않았습니다.

## 프로젝트를 열지 않은 격리 프로필의 부분 측정

동일 기기·동일 계측용 release 번들·프로젝트를 열지 않은 `claude-release` 격리 프로필에서 완전 종료 후 세 번 기동했습니다. Rust 값은 Inspector의 `perf_snapshot.entries`, 프론트 값은 앱 계측이 남긴 `performance` measure에서 읽었습니다. 세 실행 모두 `⌘Q` 뒤 exit 0이고 IDE listener·격리 lockfile이 사라졌습니다.

| 지표 (ms)               | 1회차      | 2회차     | 3회차     | 중앙값     |
| ----------------------- | ---------- | --------- | --------- | ---------- |
| `setup.main_window`     | 102.114125 | 93.489833 | 88.162042 | 93.489833  |
| `setup.locale_warm`     | 1.345833   | 0.4545    | 0.451834  | 0.4545     |
| `setup.state_restore`  | 0.940083   | 0.083458  | 0.081666  | 0.083458   |
| `setup.deferred_restore` | 2.582834   | 2.206292  | 2.259875  | 2.259875   |
| `boot.reveal`           | 43         | 35        | 35        | 35         |

세 번째 프로세스에서 팔레트를 닫힌 상태로 열고 `perf` 네 글자를 입력하는 조작을 세 번 반복했습니다. `palette.open`은 9·2·1ms로 중앙값 2ms였고 네 번째 글자 뒤 `palette.filter`는 2·2·2ms로 중앙값 2ms였습니다. 이는 한 프로세스 안의 반복이며 프로젝트를 열지 않은 결과입니다.

## 판정

부팅·팔레트의 부분 수치를 얻었지만 기준 체크리스트의 5,000개 이상 파일·1,000개 이상 커밋 fixture가 아니고 프로젝트 전환·파일 열기·트리·Git·검색·터미널·메모리 지표도 측정하지 않았습니다. 따라서 `docs/quality-assurance/2026-09-04-perf-baseline.md` 전체 기준선 및 M7-C4c-2는 미완료로 둡니다. 이 수치로 제품 성능 회귀 여부를 판정하지 않습니다.
