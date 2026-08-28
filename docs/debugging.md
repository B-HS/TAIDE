# 디버깅 가이드 — 정본

> TAIDE 를 진단할 때의 진입점·도구·이 프로젝트에서 실증된 계측 기법을 모은다.
> 원칙: **재현·계측이 먼저, 추측 수정 금지.** 가능하면 결함을 드러내는 실패 테스트를 먼저 만든다.

## 1. 로그

| 항목 | 값 |
|------|-----|
| 경로 | `~/Library/Logs/dev.taide.app/TAIDE.log` |
| 특성 | UTC 타임스탬프 · 40KB 회전(tauri-plugin-log) |
| 활용 예 | 원격 접속 서버 포트 발견(`원격 접속 서버 기동: port=` 라인 — e2e 하네스도 이 라인을 파싱), 부팅 시퀀스·에러 추적 |

## 2. 실행·검증 사다리

- dev 실행: `bun run tauri dev`. 프론트는 Vite HMR, **Rust 는 재컴파일·재시작 필요**.
- 검증: `bun run verify` = typecheck → eslint → prettier → bun test → cargo fmt/clippy/test.
  프론트 번들 회귀는 `bunx vite build` 로 추가 확인.
- 검증 없이 "통과했다"고 보고하지 않는다. 실패를 발견하면 고치고 재검증 후에만 완료 보고.

## 3. HMR 이 통하지 않는 것들 (실증된 함정)

| 변경 대상 | 반영 조건 | 근거 |
|-----------|-----------|------|
| 번들 테마·로케일(`src-tauri/resources/`) | 앱 재시작(Rust `include_str` 임베드) | d-46 실기 |
| 앱 아이콘(`src-tauri/icons/`) | 재컴파일 — `build.rs` 의 `rerun-if-changed=icons` 가 트리거(2026-08-28 신설). dev 도크 아이콘은 `generate_context!` 가 컴파일 타임에 icns 를 임베드해 `RunEvent::Ready` 에서 적용하는 구조라, 이 가드 이전엔 아이콘만 바꾸면 영원히 미반영이었다 | `docs/acknowledge/2026-08-28-app-icon-prompt-spark.md` § dev 미반영 |
| Rust 코드 전반 | tauri CLI 가 감지해 재빌드·재시작 | - |

## 4. 실증된 계측 기법 (선례 — 재사용 권장)

모두 실제 결함의 근본 확정에 사용됐다. **계측 코드는 확정 후 반드시 원복한다.**

- **원격 페이지 WS 몽키패치**: 원격 접속 페이지에서 WebSocket send/receive 를 몽키패치해
  IPC 트래픽을 관찰 — 저장 직후 구식 캐시 채택(d-43)의 근본을 이걸로 확정했다.
- **프레임 갭 계측**: rAF 델타를 기록해 롱스톨(예: 100ms+)을 검출 — 피커 드래그 프리징(d-45)
  에서 "웹 파이프라인 단독으로는 스톨 미재현(240무브 max 47ms)"을 실측해 네이티브 축을
  특정했다. 스크립트 드래그(pointer 이벤트 합성)와 조합하면 재현이 결정적이 된다.
- **레지스트리 소스 실물 확인**: 의존 크레이트의 실제 소스를 cargo 레지스트리에서 직접 연다
  (`~/development/rust/cargo/registry/src/index.crates.io-*/<crate>-<ver>/`). tao 0.35.3 의
  `set_theme` 무단락 전역 재적용(d-45), tauri 2.11.5 dev 도크 아이콘 임베드 경로(§3)를 이렇게
  확정했다. 문서·기억보다 소스 실물이 정본이다.
- **대조 환경 격리**: 같은 코드를 결함 축이 배제된 환경(예: 네이티브 IPC 가 실효 없는 원격
  페이지)에서 돌려 층을 분리한다 — "웹 무죄 → 네이티브 유죄" 식 소거법.

## 5. e2e 하네스

- 실행: `TAIDE_E2E_PASSWORD=<비밀번호> bun run e2e` (비밀번호는 셸 프로필 export 권장.
  로그인 무재시도·5회 실패 시 60초 잠금). 앱은 사전에 떠 있어야 하며 포트는 하네스가
  로그(§1)에서 자동 발견한다. 사용법 정본: `docs/quality-assurance/2026-08-18-e2e-harness.md`.
- **픽스처 루트는 `~/Library/Caches/dev.taide.app/e2e-fixtures`** — Vite watch 루트 밖이면서
  FSEvents 감시가 되는 위치여야 한다. `os.tmpdir()`(`/var/folders`)은 macOS FSEvents 미감시로
  기각(실측), 리포 안은 Vite full-reload 유발로 기각. 정본 JSDoc: `e2e/lib/paths.ts`.
- 기지 리스크: 앱 재시작 직후 첫 런은 spec 05 가 vtsls 콜드 인덱싱으로 실패할 수 있다(직후
  격리 재실행 통과). 스위트 2회차부터 후반 스펙 no-shim 연쇄(C8)가 가능하나 서버 무죄가
  라이브 프로브로 확정된 하네스 webkit 열화다(dev 전용·자가 회복 — 파일럿 보고서 §7-C8).
- 단언 약화 금지: 스펙이 앱 결함을 드러내면 스펙을 앱에 맞추지 말고 앱을 고친다.
- webkit `type()` 은 IME 손상 가능 — 텍스트 입력은 클립보드 붙여넣기 방식을 쓴다(하네스에
  이미 반영).

## 6. 흔한 결함 클래스 (이 코드베이스에서 실제 발생 — 수정 시 회귀 주의)

| 클래스 | 요지 | 정본 |
|--------|------|------|
| 캐시-버퍼 역행 | 저장 성공 후 FILE.CONTENT 캐시가 구식이면 렌더 채택 분기가 버퍼를 되돌림 — 저장 성공 지점에서 캐시 동기 패치 필수 | d-43 계약 |
| 외부 변경 무효화 누락 | `.git` 워처는 index/HEAD 만 본다 — 워크트리 파일 변경은 `fs:changed` 에서 git 쿼리 무효화로 이어야 함 | d-44 계약 |
| 네이티브 IPC 홍수 | 고빈도 UI 이벤트(드래그 move)가 네이티브 IPC 를 직결 호출하면 메인 스레드 포화 — 변화 가드·코얼레싱·커밋 시점 지연으로 상한 | d-45 계약 |
| 프리뷰/적용 경합 | monaco·shiki 재적용류는 `runExclusive` 큐 안에서만 — 큐 밖 호출은 홍수 시 경합 | d-45 계약 §3 |

## 7. 진단 팁

- 상태의 단일 출처는 Rust(ADR-0004) — view 이상은 먼저 Rust 상태·이벤트 흐름을 의심한다.
- 커맨드 dispatch 는 커스텀 테이블(`src-tauri/src/lib.rs`) — 커맨드 이름 불일치는 patch 대상
  테이블과 `collect_commands!` 패리티 테스트가 잡는다.
- cargo 환경: `CARGO_HOME=$HOME/development/rust/cargo`, `RUSTUP_HOME=$HOME/development/rust/rustup`
  를 PATH 에 선행 export(셸 프로필에 없으면 수동).
