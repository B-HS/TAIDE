# 기능 — 플러그인 (선언적 매니페스트)

> FR-E3·FR-J1. 결정 근거: ADR-0010(선언적, 코드 실행 없음). 1차 유스케이스 = LSP 언어 추가.

## 1. 구조

```
{app_data}/plugins/{plugin-id}/
├── taide-plugin.json     매니페스트 (정본 스키마는 이 문서)
├── grammars/             Monaco Monarch 문법(.json) — language 기여용
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
                "grammar": "grammars/go.monarch.json"   // 선택 — 없으면 plain text
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

## 3. 로드·검증 (Rust plugin 도메인)

- 기동 시 `plugins/` 스캔 → 매니페스트 파싱·검증(스키마 버전, id/디렉토리 일치, 경로 탈출 금지 —
  기여 파일 경로는 플러그인 디렉토리 내부로 제한) → 레지스트리 등록 → 각 도메인에 기여 전달
  (language→에디터, lsp→lsp 도메인, theme→테마 목록).
- 검증 실패 시 해당 플러그인만 비활성 + 사유를 설정 UI 에 표시(전체 기동은 계속).
- 설치/제거 = 디렉토리 추가/삭제(1차는 수동 + 설정 UI 에서 폴더 열기 버튼). 변경은 재시작 또는
  "플러그인 다시 읽기" 버튼으로 반영(핫리로드는 2차).

## 4. 보안 경계

- 플러그인은 코드를 실행하지 않는다. 유일한 프로세스 실행 지점은 `lsp.cmd` 이며,
  **사용자가 설정 UI 에서 해당 플러그인의 LSP 를 활성화할 때 명시 동의**를 받는다
  (커맨드·인자를 그대로 보여줌). 동의 전 자동 기동 금지.
- 매니페스트의 경로는 canonicalize 후 플러그인 루트 하위인지 검증(path traversal 차단).

## 5. capability(코어 확장)와의 구분

- 플러그인 = 선언적 리소스 추가(언어·테마). **remote-control 등 앱 기능 확장은 프로젝트
  capability**(`architecture.md` §3)로 코어에 컴파일된다. 두 축을 혼동하지 않는다(ADR-0010).
