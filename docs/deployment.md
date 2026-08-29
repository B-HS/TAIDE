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
2. **`src-tauri/tauri.conf.json` 과 `src-tauri/Cargo.toml` 의 `version` 을 동시 갱신**·커밋
   (CI 가드가 태그와 두 파일 모두의 일치를 강제 — 앱 자가 보고 버전 드리프트 방지) +
   `package.json` version 도 동기.
3. **릴리스 노트 작성 — `docs/release-notes/v<version>.md`** (CI release job 이 이 파일을
   Release 본문으로 사용하며, **없으면 릴리스가 실패**한다. 하이라이트·수정 사항·설치 안내 구성
   — v0.1.2 파일이 표본).
4. 검증 사다리 통과 확인 — `bun run verify` + `bunx vite build`.
5. 태그 생성·푸시: `git tag v<version> && git push origin v<version>`.
6. Actions 에서 Release 런 완주 확인(job 별 타임아웃: 테스트 20/45분·build 75분·release 15분)
   → draft Release 검토 후 수동 공개.

## 4. CI 파이프라인 (release.yml — 4-job 병렬, 2026-08-28 재편)

| job | 내용 | 비고 |
|-----|------|------|
| `test-frontend` (동시) | `bun install --frozen-lockfile` → `bun run test` | e2e 는 매처 밖이라 미포함 |
| `test-rust` (동시) | `cargo test --workspace` — **debug 프로필**(릴리스 프로필 검증은 build job 의 `tauri build` 컴파일 게이트가 담당. 최적화 민감 코드를 넣게 되면 `--release` job 복원 검토) | 자체 rust-cache(shared-key: tests) |
| `build` (동시) | 태그 가드 2종(숫자 4 금지 → `tauri.conf.json`+`Cargo.toml` 버전 일치) → 서명 구성(.p12 임시 키체인·아이덴티티 자동 추출) → 공증 구성(3종 secrets 전부 있으면 활성·아니면 명시 스킵) → `bun run tauri build` → 자립성 검증(`otool -L` — homebrew//usr/local 링크 발견 시 실패) → dmg 수집·공백→점 리네임·`SHA256SUMS.txt` → artifact(`taide-dmg`) 업로드 | rust-cache(shared-key: release) |
| `release` (needs: 위 3) | 태그 푸시일 때만 — artifact 다운로드 → **draft** GitHub Release 생성 | 테스트 실패 시 draft 미생성 |

- wall-clock ≈ max(build, 테스트) — 이전 단일 job 직렬 대비 테스트 시간만큼 단축.

## 5. Secrets (이름만 — 값은 어디에도 기록하지 않는다)

GitHub 레포 secrets 5건 등재 완료(2026-08-19, raw-viewer 선례 이식):
`MACOS_CERTIFICATE_P12` · `MACOS_CERTIFICATE_PASSWORD` · `APPLE_ID` ·
`APPLE_APP_SPECIFIC_PASSWORD` · `APPLE_TEAM_ID`(팀 `SN98P5V7J4`).

- `APPLE_SIGNING_IDENTITY` 는 secret 이 아니라 .p12 에서 자동 추출(선택적 override 만 지원).
- tauri-plugin-updater 미사용이므로 updater 서명 키는 없다(도입 시 재론).

## 6. 로컬 빌드·미서명 배포

- 로컬 프로덕션 빌드: `bun run tauri build` → **루트** `target/release/bundle/{macos,dmg}/`
  (워크스페이스라 src-tauri 하위가 아님 — §8 "no .app bundle found" 참조).
- 로컬 빌드 .app 을 기존 설치본 위에 덮어쓸 때 `cp` 가 `Operation not permitted` 로 막히면
  macOS 앱 관리 보호(TCC)다 — dmg 를 열어 Finder 드래그("대치")가 정공.
- 미서명 빌드를 배포받은 경우 최초 실행 전 격리 해제: `xattr -cr "/Applications/TAIDE.app"`
  (로컬에서 직접 빌드한 .app 은 격리 속성이 없어 불필요).

## 7. 도메인 `taide.gumyo.net` — 원격 접속 공개 서빙 준비물 (2026-08-28 문답 정본)

앱 도메인이 `taide.gumyo.net` 으로 확정(identifier `net.gumyo.taide` 개명의 근거 —
`acknowledge/2026-08-29-bundle-identifier-rename.md`). 이 도메인으로 원격 접속을 서빙하려면:

1. 앱 설정 → Remote → **허용 호스트에 `taide.gumyo.net` 등재** — 미등재 Host 는 서버가 전
   요청 거부(DNS 리바인딩 방어, `remote/server.rs`). `*.` 접두 와일드카드 지원.
2. **HTTPS 종단 리버스 프록시**(Caddy/nginx/Cloudflare Tunnel 등) + `X-Forwarded-Proto: https`
   전달 + **WebSocket 업그레이드** 지원 — 허용 호스트라도 이 헤더가 없으면 평문 취급.
3. DNS 를 해당 머신(고정 IP/터널)으로, TLS 인증서는 프록시가 담당(앱 서버는 127.0.0.1 평문).

- GitHub 동기화(sync)는 **도메인과 무관** — Gist + `gist` 스코프 PAT 방식(OAuth 콜백 없음).
  할 일은 설정 Sync 섹션에 PAT 연결뿐.
- 기록: 원격 페이지 자체는 CSP 헤더 미부착(d-48 계약 §4 후속 후보 — 비밀번호 게이트 뒤라 기록만).

## 8. 트러블슈팅

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
- **"non-system dylib linkage" (자립성 검증 스텝)**: 바이너리가 Homebrew dylib 를 동적
  링크한 경우다 — v0.1.0 3차 런이 `libssl/libcrypto`(git2 ssh/https 의 libssh2 계열)와
  `liblzma`(xz2)로 실패했다. 수정: git2 `vendored-openssl`·xz2 `static`(2026-08-28, 로컬
  otool 로 외부 링크 0 확증). **새 네이티브 의존을 추가할 때는 vendored/static 여부를 반드시
  확인**하라 — 이 게이트가 최종 방어선이다.
- 태그 재시도: 실패한 태그 런의 재실행(rerun)은 **태그 시점의 워크플로**로 돈다 — 워크플로
  수정을 반영하려면 태그를 새 커밋으로 다시 발행해야 한다(미발행 draft 상태라면 태그
  삭제·재푸시 가능 — 사용자 승인 필수).

## 9. 릴리스 이력

| 태그 | 일자 | 비고 |
|------|------|------|
| `v0.1.5` | 2026-08-29 | **완주(런 `33248319213`, wall 약 9m13s — 워밍 캐시 첫 적용, build 8m55s)** — 사용성 릴리스(파일트리 git 데코레이션·터미널 Shift+Enter LF·SCM 키보드 내비). dmg 14,891,912B. 숫자 4 금지로 0.1.4 건너뜀. draft 생성 — 사용자 공개 대기 |
| `v0.1.3` | 2026-08-29 | **완주(런 `33231960782`, wall 약 10m32s — 프로필 전환 직후 콜드)** — 전수조사 정비 릴리스(fix 41·성능·설정 10종·thin LTO+opt3). 워밍 캐시 효과는 다음 릴리스부터. draft 생성 — 사용자 공개 대기 |
| `v0.1.2` | 2026-08-29 | **완주(런 `33190360486`, wall-clock 9m35s)** — d-48 근본 수정(CSP style-src)·identifier `net.gumyo.taide` 개명·릴리스 노트 체계 첫 적용. 설치 실기 3건(d-47~49) 전부 종결된 첫 공개 후보. draft 생성 — 사용자 공개 대기 |
| `v0.1.1` | 2026-08-28 | ~~폐기~~(draft·태그 삭제 — d-48 잔존·identifier 개명 반영 위해). 4-job 병렬 CI 첫 실전(run `33166793025`, 약 10분 — 단일 job 16m54s 대비 단축). d-47 해소는 이 빌드에서 확인 |
| `v0.1.0` | 2026-08-28 | ~~폐기~~(draft·태그 삭제 — 설치 실기에서 d-47~49 발견). 4차 런에서 파이프라인 자체는 완주. 실패 이력: 1차 시크릿 base64 → 2차 번들 경로 → 3차 Homebrew dylib(§8 트러블슈팅 3건의 출처) |
