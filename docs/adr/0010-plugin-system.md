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
   - `language`: 언어 id, 파일 확장자, TextMate 문법(`.tmLanguage.json`) 자산
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

## 7.10-W7 보강 (2026-08-12)

- `language` 기여의 문법 자산은 **TextMate(`.tmLanguage.json`) 단일**로 확정한다. 위 "결정" 2항의
  "textmate/monarch" 양다리 표현은 이 문서 작성 당시의 미확정 상태를 반영한 것이었고, 실제 구현은
  한 번도 Monarch 경로가 동작한 적이 없다(프론트 소비자 0건 실증 — `docs/features/plugins.md` 참고).
- 렌더링 엔진은 **shiki 4.4.3**(JS RegExp 엔진, `createHighlighterCore` + `createJavaScriptRegexEngine`)
  로 확정한다. 현행 CSP(`script-src 'self'`)는 변경하지 않는다 — WASM 엔진(oniguruma)은 채택하지 않는다.
- 매니페스트 `grammar` 필드의 **의미를 TextMate 로 재정의**했다(값 형태는 그대로 상대 경로 문자열).
  `manifestVersion` 은 1을 유지한다 — 재정의로 깨질 기존 소비자가 실증적으로 0건이기 때문이다.
  선택 필드 `embeddedLanguages` 를 신설한다(§ 상세는 `docs/features/plugins.md` §2).
- VS Code 확장(`.vsix`)의 `contributes.grammars` 임포트는 W7 범위 밖이다(`docs/backlog.md`) —
  언어 id 충돌 정책과 `LANGUAGE_ID_BY_EXTENSION` 런타임화가 선행돼야 하는 별개 축.
