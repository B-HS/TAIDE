# monaco `ICommandService` 딥 임포트 — 채택 사유

## 대상 파일

- `src/shared/lib/lsp/command-relay.ts` (`loadMonacoCommandService`, `executeMonacoCommand`)
- `src/shared/lib/lsp/monaco-internal.d.ts` (앰비언트 타입 shim)

## 리포트

`editor.action.showReferences(uri, position, locations)` 처럼 monaco 에 이미 등록된 커맨드를 **여러 개의
위치 인자**로 실행해야 하는 지점(rust-analyzer/tsls 의 CodeLens·`rust-analyzer.gotoLocation` 클라 커맨드
중계)에 공개 `monaco.d.ts` API 가 없다.

- `monaco.editor.registerCommand` 는 **등록 전용**이다 — 실행 API 가 아니다.
- `editor.trigger(source, id, payload)` 는 payload **1개**만 전달한다. 다인자 커맨드를 호출할 수 없다.
- monaco 자신도 내부적으로 `editor.action.showReferences` → `editor.action.peekLocations` 별칭을
  `ICommandService`/`StandaloneServices.get` 로 실행한다(`registerCommandAlias` 내부 구현) — 즉 이
  경로가 monaco 자체가 다인자 커맨드를 실행하는 유일한 메커니즘이다.

**대안 기각**

- IBulkEditService 내부 교체: 계약 문서(`2026-08-14-wave-a-lsp-intelligence-contract.md` §4)에서
  별개 사유로 이미 기각됨(비공개 esm 내부 API 우회) — 같은 사유가 이 딥 임포트에도 적용되므로,
  "왜 그럼에도 이 경로를 택했는가"를 명시할 필요가 있어 이 문서를 남긴다.
- 신규 monaco 공개 API 요청/패치: 별도 fork 유지 비용이 이 프로젝트 규모에 맞지 않아 기각.
- 인자를 1개 객체로 합쳐 `editor.trigger` 로 우회: 대상 커맨드(monaco 내장 alias 포함)가 이미
  포지셔널 시그니처로 고정돼 있어 클라이언트 쪽에서 바꿀 수 없음.

## 상세

- 의존 경로는 정확히 2개, 심볼도 2개로 최소화되어 있다: `monaco-editor/platform/commands/common/commands`
  의 `ICommandService`, `monaco-editor/editor/standalone/browser/standaloneServices` 의
  `StandaloneServices`.
- `monaco-editor` 의 `package.json` `exports` 맵이 `"./*": "./esm/vs/*.js"` 와일드카드로 이 서브패스를
  이미 공개 표면으로 노출하고 있다(타입 선언만 없음) — 즉 "비공개 API 우회"가 아니라 "타입 미제공
  공개 표면"에 가깝다. 다만 monaco 팀이 내부 리팩터링 시 이 경로를 아무 통보 없이 바꿀 수 있다는
  리스크는 남는다.
- `command-relay.ts` 내부 1파일로 접근을 한정했고, 동적 `import()`(정적 top-level import 아님)로
  로드해 이 모듈의 다른 순수 함수를 DOM 없는 환경(`bun:test`)에서도 그대로 쓸 수 있게 격리했다.
- **monaco 업그레이드 시 확인 절차**: `node_modules/monaco-editor/esm/vs/platform/commands/common/commands.js`
  와 `.../editor/standalone/browser/standaloneServices.js` 경로가 그대로 존재하는지, `ICommandService`
  심볼명이 유지되는지 확인한다. 이 두 파일이 삭제/개명되면 `command-relay.test.ts` 의
  `registerSessionExecuteCommands`/`createShowReferencesHandler` 관련 테스트가 (동적 import 실패로)
  먼저 깨진다.
