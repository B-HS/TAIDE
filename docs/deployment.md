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
  `MACOS_CERTIFICATE_P12` 등록값이 한 줄 base64 가 아닌 경우다(웹 UI 붙여넣기 중 개행·CR·76자
  래핑 혼입). 2026-08-28 v0.1.0 1차 런이 이걸로 실패했다. 워크플로가 디코드 전 공백·개행을
  제거하도록 내성 패치됐지만, 값 자체가 잘못됐으면 `openssl base64 -A -in cert.p12 | pbcopy` 로
  재등록해야 한다(등록 절차 정본: `acknowledge/2026-08-19-phase8-signing-secrets.md`).
- 태그 재시도: 실패한 태그 런의 재실행(rerun)은 **태그 시점의 워크플로**로 돈다 — 워크플로
  수정을 반영하려면 태그를 새 커밋으로 다시 발행해야 한다(미발행 draft 상태라면 태그
  삭제·재푸시 가능 — 사용자 승인 필수).

## 8. 릴리스 이력

| 태그 | 일자 | 비고 |
|------|------|------|
| `v0.1.0` | 2026-08-28 | 1차 실행(사용자 결정 — qa6 완주 전 조기 태깅). 가드 2종·bun/cargo 테스트 통과 후 **서명 스텝 base64 디코드 실패**(§7) — 재시도 대기. Prompt Spark 아이콘·번들 테마 38·커맨드 181 시점 |
