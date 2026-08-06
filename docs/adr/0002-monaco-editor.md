# ADR-0002 — 코드 에디터 코어: Monaco Editor

- 상태: 승인 (사용자 확정, 2026-08-06)
- 관련: `docs/PRD.md` FR-D, `docs/features/editor.md`, `docs/research/monaco.md`

## 맥락

요구사항: VSCode 기본 단축키 전부 지원(FR-D4), split diff 뷰(FR-F3), git gutter·인라인 blame(FR-D2/D3),
5개 언어 LSP(FR-E). 후보는 Monaco Editor 와 CodeMirror 6.

## 결정

**Monaco Editor** 를 사용한다.

## 근거

- Monaco 는 VSCode 의 실제 에디터다. VSCode 단축키·멀티커서·찾기 위젯·명령 체계가 기본 내장이라
  "VSCode 기본 단축키 전부" 요구를 사실상 무비용으로 충족한다.
- DiffEditor(side-by-side) 내장 — Git split diff 를 별도 구현 없이 사용.
- decorations / content widget API 로 git gutter·인라인 blame 을 구현하는 패턴이 확립되어 있다.
- TS/JS intellisense worker 내장 — LSP 연동 전에도 기본 편집 경험 확보.
- monaco-languageclient 로 LSP 연동 경로가 성숙하다.

## 기각한 대안

- **CodeMirror 6**: 훨씬 가볍고 테마 자유도가 높으나, VSCode 키맵·diff·LSP 를 전부 조합/구현해야 해
  요구 충족 비용이 크다. 번들 경량화보다 기능 재현 정확도를 우선했다.

## 결과 (트레이드오프)

- 번들 자산 수 MB 증가 → Tauri 로컬 자산이라 네트워크 비용 없음. lazy load 로 초기 기동 영향 최소화.
- 테마 커스텀 단위가 CodeMirror 보다 거침 → defineTheme + semantic token 규칙을 테마 시스템에서
  일괄 생성하는 방식으로 흡수(`docs/theme-system.md`).
- 모바일 미지원, 번들러 worker 설정 필요 → 데스크톱 전용이므로 무관, Vite worker 설정은
  `docs/research/monaco.md` 확정안을 따른다.
