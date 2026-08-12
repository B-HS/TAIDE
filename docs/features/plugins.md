# 기능 — 플러그인 (선언적 매니페스트)

> FR-E3·FR-J1. 결정 근거: ADR-0010(선언적, 코드 실행 없음). 1차 유스케이스 = LSP 언어 추가.

## 1. 구조

```
{app_data}/plugins/{plugin-id}/
├── taide-plugin.json     매니페스트 (정본 스키마는 이 문서)
├── grammars/             TextMate 문법(*.tmLanguage.json) — language 기여용
└── themes/               테마 파일 — theme 기여용
```

## 2. 매니페스트 스키마 (`taide-plugin.json`)

```jsonc
{
    "manifestVersion": 1,
    "id": "taide-plugin-go",              // kebab-case, 디렉토리명과 일치
    "name": "Go Language Support",
    "version": "1.0.0",
    "contributes": {
        "languages": [
            {
                "id": "go",
                "extensions": [".go"],
                "aliases": ["Go"],
                "grammar": "grammars/go.tmLanguage.json",  // 선택 — 없으면 plain text
                "embeddedLanguages": []                     // 선택 — embedded grammar(예: erb=html+ruby)
            }
        ],
        "lsp": [
            {
                "languageId": "go",
                "id": "gopls",
                "cmd": "gopls",                          // PATH 탐색 대상
                "args": [],
                "detect": ["$PATH", "~/go/bin/gopls"],  // 감지 순서 (경로 후보)
                "initializationOptions": {},
                "settingsNamespace": "gopls",
                "shareable": true,                       // workspaceFolders 공유 가능 여부
                "install": {                             // 미설치 시 안내
                    "instructions": "go install golang.org/x/tools/gopls@latest"
                }
            }
        ],
        "themes": [
            { "id": "my-theme", "path": "themes/my-theme.json" }
        ]
    }
}
```

- `lsp` 항목은 내장 서버와 동일한 `LanguageServerSpec`(`lsp.md` §1)으로 변환된다.
- **실행 커맨드는 사용자 시스템의 바이너리를 참조**한다(플러그인에 바이너리 동봉 금지 — ADR-0010).
  다운로드 URL 기여는 2차(SHA256 필수 스키마로 확장).
- `grammar` 항목은 TextMate 문법 JSON(`.tmLanguage.json`, `scopeName` 필수)이다. 로드 시 shiki
  `LanguageRegistration` 형태로 변환되어(TAIDE id 를 lang name 으로 재명명) highlighter 생성 시
  주입되고, `shikiToMonaco` 경유로 해당 언어 id 에 부착된다(`docs/theme-system.md` §4.2).

## 3. 로드·검증 (Rust plugin 도메인)

- 기동 시 `plugins/` 스캔 → 매니페스트 파싱·검증(스키마 버전, id/디렉토리 일치, 경로 탈출 금지 —
  기여 파일 경로는 플러그인 디렉토리 내부로 제한) → 레지스트리 등록 → 각 도메인에 기여 전달
  (language→에디터, lsp→lsp 도메인, theme→테마 목록).
- `grammar` 기여는 추가로 다음을 검증한다: 파일 실재(미존재 시 `grammarMissing`), JSON 파싱 +
  `scopeName` 필드 존재(실패 시 `grammarInvalid`), 크기 상한(초과 시 거부), 다른 플러그인과의
  `scopeName`/`languageId` 충돌(선착순 등록 승 + 후순위는 경고만 표시 — `grammarConflict`).
- 검증 실패 시 해당 플러그인만 비활성 + 사유를 설정 UI 에 표시(전체 기동은 계속).
- 설치/제거 = 디렉토리 추가/삭제(1차는 수동 + 설정 UI 에서 폴더 열기 버튼). 변경은 재시작 또는
  "플러그인 다시 읽기" 버튼으로 반영 — grammar 변경은 shiki highlighter 재생성(TAIDE 언어 재구성)이
  필요하므로 "플러그인 다시 읽기" 가 highlighter 를 다시 만든다(핫리로드, 재시작 불필요).

## 4. 보안 경계

- 플러그인은 코드를 실행하지 않는다. 유일한 프로세스 실행 지점은 `lsp.cmd` 이며,
  **사용자가 설정 UI 에서 해당 플러그인의 LSP 를 활성화할 때 명시 동의**를 받는다
  (커맨드·인자를 그대로 보여줌). 동의 전 자동 기동 금지.
- 매니페스트의 경로는 canonicalize 후 플러그인 루트 하위인지 검증(path traversal 차단).
- grammar 는 코드가 아니라 데이터지만, TextMate 문법의 정규식 패턴은 **ReDoS(catastrophic
  backtracking)** 로 렌더러를 멈출 수 있다. shiki 의 `tokenizeTimeLimit`(기본 500ms)에 의존해
  한 줄 토큰화 시간을 제한한다 — 플러그인 grammar 를 신뢰하지 않는 완전한 방어는 아니며, 한계로
  문서화한다.
- 프론트는 grammar 파일 본문을 매니페스트 스캔 결과에 인라인으로 받지 않는다. 온디맨드 IPC
  query `plugin_read_grammar(pluginId, languageId)` 로 필요한 시점(highlighter 생성)에만
  가져온다(`docs/ipc-contract.md` "plugin" 절).

## 5. capability(코어 확장)와의 구분

- 플러그인 = 선언적 리소스 추가(언어·테마). **remote-control 등 앱 기능 확장은 프로젝트
  capability**(`architecture.md` §3)로 코어에 컴파일된다. 두 축을 혼동하지 않는다(ADR-0010).
