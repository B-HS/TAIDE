# ADR-0010 — 플러그인 체계: 선언적 매니페스트 (1차 유스케이스 = LSP 추가)

- 상태: 승인 (2026-08-06)
- 관련: `docs/PRD.md` FR-E3·FR-J, `docs/features/plugins.md`

## 맥락

기본 내장 언어(JS/TS·Rust·Python·Markdown) 외 언어는 플러그인 설치로 LSP 를 추가할 수 있어야 한다.
VSCode 식 코드 실행형 확장(임의 JS 실행)은 보안 표면·API 안정화 부담이 크고, 현 단계 실수요는
"언어 지원 추가"에 집중되어 있다.

## 결정

1. 플러그인은 **선언적 매니페스트(JSON) + 정적 자산** 으로 정의한다. 임의 코드 실행은 하지 않는다.
2. 1차 지원 기여 지점(contribution point):
   - `language`: 언어 id, 파일 확장자, Monaco 문법(textmate/monarch) 자산
   - `lsp`: 서버 실행 커맨드·args·initializationOptions, 서버 설치 안내(또는 다운로드 URL)
   - `theme`: 테마 파일 기여
3. 설치 = `plugins/{id}/` 폴더에 배치(수동 설치 우선, 설치 UI 는 추후). Rust plugin 도메인이
   기동 시 매니페스트를 스캔·검증·등록한다.
4. LSP 서버 바이너리 자체는 플러그인에 동봉하지 않고 **사용자 시스템 설치를 참조**하는 것을 기본으로 한다
   (경로 자동 탐지 + 매니페스트의 설치 안내 표시). 내장 4개 언어의 배포 전략은 ADR 별도
   (`docs/research/lsp-servers.md` 결과에 따라 `features/lsp.md` 에서 확정).

## 기각한 대안

- **코드 실행형 확장(JS 런타임 내장)**: API 안정화·샌드박싱 비용이 현 단계 가치를 초과. 추후 재검토.
- **WASM 플러그인**: 매력적이나 1차 유스케이스(LSP 등록)에 과잉.

## 결과

- 매니페스트 스키마가 공개 계약이 된다 — `features/plugins.md` 에 스키마·검증 규칙을 정본으로 명세.
- 미래 remote-control 등 앱 수준 기능은 플러그인이 아니라 **프로젝트 capability**(ADR-0004,
  `architecture.md` §3)로 부착한다. 두 확장 축을 혼동하지 않는다: 플러그인 = 선언적 리소스 추가,
  capability = 코어에 컴파일되는 기능 모듈.
