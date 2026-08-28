# 배포 (Deployment) — 정본

> TAIDE 의 릴리스는 GitHub Actions `.github/workflows/release.yml` 단일 파이프라인으로 완결된다.
> macOS(Apple Silicon) dmg 가 산출물이며, 서명·공증까지 CI 가 수행한다.
> 서명 secrets 의 등재 내역·근거는 `docs/acknowledge/2026-08-19-phase8-signing-secrets.md` 정본.

## 1. 트리거

| 방식 | 조건 | 비고 |
|------|------|------|
| 태그 푸시 | `v*` 패턴 (예: `v0.1.0`) | 정식 릴리스 경로 — draft GitHub Release 생성까지 수행 |
| 수동 실행 | Actions 탭 workflow_dispatch | 빌드·검증만 필요할 때. 태그 가드 스텝은 스킵되고 Release 도 만들지 않음 |

- main/dev 브랜치의 일반 푸시는 릴리스를 트리거하지 않는다.

## 2. 절대 규칙 — 태그에 숫자 4 금지

- **릴리스 태그와 앱 버전에 숫자 `4` 를 절대 포함하지 않는다.** 자리 무관 전면 금지이며,
  4 를 포함하는 버전은 통째로 건너뛴다(`0.3.x` 다음은 `0.5.0`, patch 도 `x.y.3` 다음 `x.y.5`).
- CI 가드 "Verify tag contains no digit 4" 가 버전 일치 검증보다 먼저 실행되어 위반 태그를
  즉시 실패시킨다. 정본: `docs/acknowledge/2026-08-28-release-tag-no-digit-4.md` (재론 금지).

## 3. 릴리스 절차 (사람이 하는 일)

1. 버전 결정 — 숫자 4 미포함 확인(§2).
2. `src-tauri/tauri.conf.json` 의 `version` 을 새 버전으로 갱신·커밋 (CI 가드가 태그와의
   일치를 강제하므로 태그보다 먼저).
3. 검증 사다리 통과 확인 — `bun run verify` + `bunx vite build`.
4. 태그 생성·푸시: `git tag v<version> && git push origin v<version>`.
5. Actions 에서 Release 런 완주 확인(최대 90분 타임아웃) → draft Release 검토 후 수동 공개.

## 4. CI 파이프라인 (release.yml 실측 순서)

| 단계 | 내용 | 실패 조건 |
|------|------|-----------|
| 태그 가드 | 숫자 4 포함 여부(§2) → `tauri.conf.json` version 과 태그 일치 | 4 포함·버전 불일치 |
| 테스트 | `bun install --frozen-lockfile` → `bun run test` → `cargo test --release --workspace` | 테스트 실패 |
| 서명 구성 | .p12 를 임시 키체인에 import, `security find-identity` 로 아이덴티티 자동 추출 | 인증서 결함 |
| 공증 구성 | `APPLE_ID`/`APPLE_APP_SPECIFIC_PASSWORD`/`APPLE_TEAM_ID` 3종이 모두 있으면 활성, 아니면 명시적 스킵 로그 | (스킵 허용) |
| 빌드 | `bun run tauri build` — 아이콘·리소스(테마 38종·로케일 3종) 임베드 포함 | 빌드 실패 |
| 자립성 검증 | `.app` 내 전 실행 파일을 `otool -L` 로 검사 — `/opt/homebrew`·`/usr/local` 링크 발견 시 실패 | 비자립 dylib |
| 산출물 수집 | dmg 를 `dist-release/` 로 복사, **파일명 공백→점 리네임**(GitHub 에셋 서빙 규칙과 SHA256SUMS 정합), `SHA256SUMS.txt` 생성 | - |
| 업로드 | actions artifact(`taide-dmg`) + 태그 푸시일 때만 **draft** GitHub Release 생성 | - |

## 5. Secrets (이름만 — 값은 어디에도 기록하지 않는다)

GitHub 레포 secrets 5건 등재 완료(2026-08-19, raw-viewer 선례 이식):
`MACOS_CERTIFICATE_P12` · `MACOS_CERTIFICATE_PASSWORD` · `APPLE_ID` ·
`APPLE_APP_SPECIFIC_PASSWORD` · `APPLE_TEAM_ID`(팀 `SN98P5V7J4`).

- `APPLE_SIGNING_IDENTITY` 는 secret 이 아니라 .p12 에서 자동 추출(선택적 override 만 지원).
- tauri-plugin-updater 미사용이므로 updater 서명 키는 없다(도입 시 재론).

## 6. 로컬 빌드·미서명 배포

- 로컬 프로덕션 빌드: `bun run tauri build` → `src-tauri/target/release/bundle/{macos,dmg}/`.
- 미서명 빌드를 배포받은 경우 최초 실행 전 격리 해제가 필요하다:
  `xattr -cr "/Applications/TAIDE.app"` (Release 본문에 자동 포함되는 안내와 동일).

## 7. 트러블슈팅

- **"Configure Apple code signing" 에서 `base64: error decoding base64 input stream`**:
  `MACOS_CERTIFICATE_P12` 등록값이 유효한 base64 가 아닌 경우다 — 2026-08-28 v0.1.0 1차 런이
  이걸로 실패했다. 붙여넣기 중 base64 밖 문자(따옴표·헤더 줄)·잘림·개행 혼입이 후보다.
  **로컬 선검증: `bash scripts/verify-signing-cert.sh <cert.p12 | base64.txt>`** — CI 와 동일한
  임시 키체인 임포트까지 재현하고, .p12 원본을 주면 secret 용 한 줄 base64 를 클립보드에
  만들어 준다(비밀번호는 `CERT_PASSWORD` 환경변수 또는 프롬프트). 워크플로는 디코드 전
  공백·개행 제거 내성이 있다. 등록 절차 정본: `acknowledge/2026-08-19-phase8-signing-secrets.md`.
- **공증 자격 3종(APPLE_ID·APPLE_APP_SPECIFIC_PASSWORD·APPLE_TEAM_ID) 선검증**:
  `bash scripts/verify-notary-credentials.sh` — `xcrun notarytool history` 로 Apple 공증 서버에
  실제 인증(제출 없음)해 3종을 한 번에 판정한다. 값은 프롬프트 또는 동명 환경변수로 입력,
  실패 시 Apple 원문 오류(401 등)와 원인 후보를 출력한다.
- **`cargo test --release` 의 output filename collision / dead_code 경고**(v0.1.0 1차 런 관측):
  근본 = `[profile.release] panic="abort"` 와 테스트 하네스(unwind 필수)가 lib 를 한 그래프에서
  2회 컴파일하는데, 모바일용 `crate-type`(staticlib·cdylib)의 비해시 산출물이 충돌 —
  데스크톱에 불요한 crate-type 제거로 해소(모바일 빌드 재개 시 `[lib] crate-type` 복원 필요).
  `BINDINGS_PATH` dead_code 는 사용처와 동일한 `#[cfg(any(debug_assertions, test))]` 게이트로 해소.
- **"no .app bundle found" (자립성 검증 스텝)**: 루트 워크스페이스라 cargo 산출물은
  `<repo>/target/` 이다 — `src-tauri/target/...` 을 보던 초기 경로 결함으로 v0.1.0 2차 런이
  실패했다(2026-08-28 수정: 검증·수집 경로와 rust-cache workspaces 를 루트 기준으로 정정,
  구식 `src-tauri/Cargo.lock` 중복 락 제거). 이 런에서 **서명·공증 포함 빌드 자체는 통과** —
  시크릿·빌드 경로는 검증 완료 상태다.
- 태그 재시도: 실패한 태그 런의 재실행(rerun)은 **태그 시점의 워크플로**로 돈다 — 워크플로
  수정을 반영하려면 태그를 새 커밋으로 다시 발행해야 한다(미발행 draft 상태라면 태그
  삭제·재푸시 가능 — 사용자 승인 필수).

## 8. 릴리스 이력

| 태그 | 일자 | 비고 |
|------|------|------|
| `v0.1.0` | 2026-08-28 | 1차 실행(사용자 결정 — qa6 완주 전 조기 태깅). 가드 2종·bun/cargo 테스트 통과 후 **서명 스텝 base64 디코드 실패**(§7) — 재시도 대기. Prompt Spark 아이콘·번들 테마 38·커맨드 181 시점 |
