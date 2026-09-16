# TS 파일의 모든 import 가 "모듈 없음" 으로 표시됨 — 내장 ts worker 폴백 노이즈 + vtsls 세션 부재 (2026-09-16)

> 정본 계약: `docs/acknowledge/2026-09-16-d64-ts-fallback-lsp-observability-tree-contract.md`

## 증상

- gumba(`/Users/gkn/gumba`, Next.js, `node_modules/`·`.next/` 존재) 의 `src/shared/utils/vendor-utils.ts` 를 열면
  import 7줄 전부와 `process` 에 빨간 밑줄. 설치 안 된 프로젝트처럼 보임.
- 같은 프로젝트의 `.tsx` 파일에서는 나타나지 않음.

## 원인

1. 화면의 진단은 vtsls 가 아니라 **monaco 내장 ts worker** 가 낸 것 — IDE 서버 `getDiagnostics` 로 확인한
   메시지가 전부 TS2792("… 'moduleResolution' … 'paths' …")·TS2580(`process`/`@types/node`). 이 문구는 compilerOptions
   없이 돌아가는 worker 에서만 나온다.
2. 내장 worker 는 `src/shared/lib/monaco/setup.ts` 의 전체 `monaco-editor` import 로 자동 활성화되지만
   `monaco.languages.typescript.*Defaults` 설정은 저장소 어디에도 없다 → tsconfig·paths·node_modules 를 모르고
   `typescript`/`javascript` 모델에 semantic 진단을 낸다. `typescriptreact`(`.tsx`)는 TAIDE 별도 언어 id 라 내장
   worker 미적용 — `.tsx` 에서 안 보이는 이유.
3. vtsls 세션은 붙어 있지 않았다(`ps` 에 vtsls/node 없음). 왜 안 붙었는지는 **로그가 없어 미확정** — Rust LSP
   경로에 감지·spawn·종료 로그가 전무하고(`commands.rs` warn 1건뿐, stderr `Stdio::null()`), 프론트도 spawn/ready
   거부를 `.catch(() => undefined)` 로 삼킨다. vtsls 자체(`~/.bun/bin/vtsls`)·로그인 셸 PATH·`fix_path_env`
   프로브·`initialize` 응답은 모두 정상으로 실측됨.
4. 문서(`editor.md` §12, `lsp.md` §4)는 내장 baseline 설정과 세션 중 내장 기능 off 를 구현된 것처럼 기술하고
   있었으나 실물에는 없다.

## 수정 (d-64)

- 내장 worker 폴백을 구문 검사 전용으로(semantic 진단 off) — 컨텍스트 없는 semantic 진단은 오탐뿐.
- TS/JS LSP 세션이 붙어 있는 동안 내장 provider 정지(`setModeConfiguration`), 해제 시 복원.
- LSP 감지·spawn·종료(stderr tail, 마스킹) 로그 + 프론트 세션 실패 warn → 다음 실행에서 세션 부재 원인 확정.
- 문서 정정.

## 확인 방법 (사용자 실기)

설정 → LSP 서버의 vtsls 행, 상태바 `LSP n/m`, `~/Library/Logs/net.gumyo.taide/TAIDE.log` 의 `lsp detect` /
`lsp vtsls: spawn` / `exited` 줄, `vendor-utils.ts` 밑줄 소멸.
