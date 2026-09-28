# M6 격리 release 앱 기동·종료 관찰

## 대상 파일

- `scripts/tauri.ts`, `src-tauri/tauri.bundle.conf.json`, `src-tauri/tauri.conf.json`
- `src-tauri/src/lib.rs`의 앱 시작·종료 조립
- `target/release/bundle/macos/TAIDE.app` 로컬 검사 산출물

## 리포트

`CARGO_NET_OFFLINE=true bun run tauri build --bundles app --no-sign --config '{"identifier":"net.gumyo.taide.m6release.0ee5f27e64f94daa82e4789556697b3c"}'`가 sidecar·Vite·Rust release profile·macOS `.app`까지 exit 0으로 완료됐습니다. `Info.plist`의 identifier를 확인했습니다. 실행을 위해 산출물에만 ad hoc 서명을 적용했고 `codesign --verify --deep --strict`가 exit 0이었습니다. 이는 배포 서명·공증 검사가 아닙니다.

## 상세

빈 `ZDOTDIR`, 별도 `/private/tmp/taide-m6-isolated.byLYaMWs/claude-release` IDE 경로, `TAIDE_PERF=1`을 환경으로 지정하고 release `.app`을 Launch Services에서 실행했습니다. 접근성 트리에 TAIDE 기본 창과 `Open Folder` 등 첫 화면이 나타났습니다. 화면 연결 도구가 장시간 지연돼 성능 snapshot이나 프로젝트 회귀는 측정하지 않았습니다.

`⌘Q` 뒤 화면 제어 도구가 `App quit`을 반환했고, IDE listener 34792와 격리 경로의 `34792.lock`이 없어졌습니다. 원래 사용자 홈의 `45059.lock`도 재생성되지 않았습니다. 앱 창 기동·기본 종료 외에 `RunEvent::Exit` 내부 순서, 동시 자원 drain, 3회 반복 성능 중앙값, 실제 배포 패키징 적합성은 검증하지 않았습니다.
