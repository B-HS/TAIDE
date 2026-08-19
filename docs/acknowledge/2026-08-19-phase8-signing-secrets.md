# Phase 8 서명·공증 — GitHub Actions secrets 사전 등록 목록 (2026-08-19)

> 사용자 요청(2026-08-19): T1 3차 진행 중 secrets 를 미리 등록해 두기로 함. 변수명은 Tauri v2
> 공식 문서(`distribute/Sign/macos.mdx`·`distribute/Pipelines/github.mdx`) 실조회로 검증
> (tauri-action 이 이 이름들을 그대로 읽음 — 임의 개명 금지).
> 레포 실측: `.github/workflows` 없음(Phase 8 에서 신설)·tauri-plugin-updater 미사용 →
> updater 서명 키(TAURI_SIGNING_PRIVATE_KEY 계열)는 **불필요**. GITHUB_TOKEN 은 Actions 자동
> 제공이라 등록 불필요.

## 0. 전제 (secret 발급 전 필요)

- [ ] Apple Developer Program 멤버십(연 99 USD) 활성.
- [ ] **Developer ID Application** 인증서 발급: 키체인 접근 → 인증서 지원 → CSR 생성 →
      developer.apple.com Certificates 에서 Developer ID Application 생성·다운로드 → 키체인
      등록 → 개인키 포함 **.p12 로 내보내기**(내보내기 비밀번호 설정).
      (App Store 배포용 "Apple Distribution" 이 아니라 앱 외부 배포용 Developer ID 인증서다)

## 1. 서명(codesign) — 4건 전부 필수

| Secret 이름 | 값 |
|---|---|
| `APPLE_CERTIFICATE` | .p12 를 base64 인코딩한 문자열: `openssl base64 -A -in certificate.p12 -out certificate-base64.txt` 의 파일 내용 |
| `APPLE_CERTIFICATE_PASSWORD` | .p12 내보내기 시 설정한 비밀번호 |
| `APPLE_SIGNING_IDENTITY` | 서명 아이덴티티 전체 문자열, 형식 `Developer ID Application: <이름> (<TEAM_ID>)` — 키체인 등록 후 `security find-identity -v -p codesigning` 출력의 따옴표 안 값 |
| `KEYCHAIN_PASSWORD` | CI 임시 키체인용 임의 비밀번호(아무 강한 랜덤 문자열 — 다른 곳에서 안 쓰는 값 신규 생성) |

## 2. 공증(notarization) — 두 방식 중 택1

**추천: 방식 B(App Store Connect API 키)** — 앱 암호(방식 A)는 계정 보안 이벤트 시 만료·재발급
이슈가 있어 CI 에는 API 키가 안정적.

### 방식 B — App Store Connect API 키 (추천, 3건)

App Store Connect → Users and Access → **Integrations 탭** → 키 추가(**Developer** 권한).

| Secret 이름 | 값 |
|---|---|
| `APPLE_API_ISSUER` | 키 테이블 상단에 표시되는 Issuer ID |
| `APPLE_API_KEY` | 키 테이블 Key ID 열 값 |
| `APPLE_API_KEY_CONTENT` | 다운로드한 `.p8` 개인키 파일의 **원문 내용**(다운로드는 1회만 가능 — 파일 보관). tauri-action 은 파일 경로(`APPLE_API_KEY_PATH`)를 요구하므로 Phase 8 워크플로가 이 내용을 러너 임시 파일로 쓰고 경로를 주입한다 |

### 방식 A — Apple ID (대안, 3건)

| Secret 이름 | 값 |
|---|---|
| `APPLE_ID` | Apple 계정 이메일 |
| `APPLE_PASSWORD` | **앱 암호**(app-specific password — account.apple.com 로그인·보안에서 생성. 계정 비밀번호 아님) |
| `APPLE_TEAM_ID` | 팀 ID(developer.apple.com → Membership 페이지) |

## 3. 등록하지 않는 것

- `GITHUB_TOKEN` — Actions 자동 제공.
- `TAURI_SIGNING_PRIVATE_KEY`·`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — tauri-plugin-updater
  전용. 현재 미사용(도입 시 재론).
- 시크릿 값은 이 레포·문서 어디에도 적지 않는다(이름만 기록). `.p12`·`.p8` 원본 파일은
  레포 밖에 보관.

## 4. Phase 8 착수 시 소비 지점 (참고)

- 신설 `.github/workflows/release.yml`(가칭)에서 tauri-action 이 §1·§2 변수를 그대로 읽어
  aarch64-apple-darwin 빌드 → codesign → notarize → staple 수행.
- 공증 소요는 Apple 서버 대기 포함 수 분 단위. 첫 실행에서 `xcrun notarytool log` 로 거부
  사유 확인 절차를 qa6 에 추가 예정.
