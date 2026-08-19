# Phase 8 서명·공증 — GitHub Actions secrets 사전 등록 목록 (2026-08-19)

> 사용자 요청(2026-08-19): T1 3차 진행 중 secrets 를 미리 등록해 두기로 함.
> **정본 선례: 사용자의 기존 프로젝트 raw-viewer** (`~/development/raw-viewer/.github/workflows/
> release.yml`) — 동일 5개 secret 으로 서명+공증을 이미 운용 중. TAIDE Phase 8 워크플로는 이
> 패턴(서명 아이덴티티를 .p12 에서 자동 추출·임시 키체인 비밀번호 인라인)을 이식한다.
> 변수명·값을 raw-viewer 와 동일하게 맞춰 프로젝트 간 일관성 유지(환경 일관성 원칙).
> 레포 실측: TAIDE 에 `.github/workflows` 없음(Phase 8 신설)·tauri-plugin-updater 미사용 →
> updater 서명 키 불필요. GITHUB_TOKEN 은 자동 제공.

## 1. 등록할 secrets — 5건 (raw-viewer 와 동일 이름·동일 값 재사용 가능)

Developer ID Application 인증서는 개발자 단위라 앱이 달라도 같은 .p12 를 재사용한다.
키체인 실측(2026-08-19): `Developer ID Application: HYUNSEOK BYUN (SN98P5V7J4)` 발급·설치 완료.

| Secret | 값 | 예시 형태 |
|---|---|---|
| `MACOS_CERTIFICATE_P12` | .p12 의 base64 한 줄(`openssl base64 -A -in cert.p12`). raw-viewer 등록값 재사용 가능 | `MIIMlQIBAzCCDF8GCSqG...` |
| `MACOS_CERTIFICATE_PASSWORD` | .p12 내보내기 비밀번호 | `p12-Export-Pw!` |
| `APPLE_ID` | Apple 계정 이메일 | `user@example.com` |
| `APPLE_APP_SPECIFIC_PASSWORD` | 앱 암호(account.apple.com 에서 생성 — 계정 비밀번호 아님) | `abcd-efgh-ijkl-mnop` |
| `APPLE_TEAM_ID` | 팀 ID | `SN98P5V7J4` (실제값) |

## 2. secret 으로 넣지 않는 것 (워크플로가 해결)

- `APPLE_SIGNING_IDENTITY` — raw-viewer release.yml:92-97 패턴: .p12 를 임시 프로브 키체인에
  import 후 `security find-identity` 로 자동 추출(선택적 override secret 만 지원). 참고 실측값:
  `Developer ID Application: HYUNSEOK BYUN (SN98P5V7J4)`.
- 키체인 비밀번호 — CI 임시 키체인용 인라인 값(secret 불필요).
- `GITHUB_TOKEN` — Actions 자동 제공.
- `TAURI_SIGNING_PRIVATE_KEY` 계열 — tauri-plugin-updater 미사용(도입 시 재론).
- 시크릿 값은 레포·문서 어디에도 적지 않는다(이름만 기록). `.p12` 원본은 레포 밖 보관.

## 3. 대안 (기록만 — 미채택)

- 공증을 App Store Connect API 키(`APPLE_API_ISSUER`/`APPLE_API_KEY`/`APPLE_API_KEY_CONTENT`)로
  하는 방식 — 앱 암호 만료 이슈에 강하나, raw-viewer 검증 선례(Apple ID 방식)와의 일관성을
  우선해 미채택. 앱 암호 만료가 실제 문제가 되면 그때 전환.
- 로컬 서명·공증(secret 0개, `xcrun notarytool store-credentials`) — Phase 8 을 CI 자동화로
  갈지 로컬 스크립트로 갈지는 착수 시 결정(현 추천: raw-viewer 패턴 CI).

## 4. Phase 8 착수 시 소비 지점 (참고)

- 신설 `.github/workflows/release.yml` 이 raw-viewer 의 서명·공증 스텝(프로브 키체인 아이덴티티
  추출 → tauri-action env 매핑 → notarize)을 이식, aarch64-apple-darwin 타깃.
- secret 미등록 시 서명/공증을 각각 건너뛰는 raw-viewer 의 graceful skip 분기도 승계.
- 공증 거부 시 `xcrun notarytool log` 확인 절차를 qa6 에 추가 예정.
