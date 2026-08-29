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

## 6. 설치 UI · VSIX grammar 임포트 · 동적 언어 등록 (Wave I)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.4. §2 확정 사실 8이
> 지적한 결함(신규 언어가 monaco/shiki 게이트에 막혀 화면에 도달하지 않음·확장자→언어 테이블
> 4벌 중복·vsix 원격 노출·zip 하드닝 부재)을 전부 이 웨이브에서 해소했다.

### 6.1 설치·제거 UI

- 설정 화면 PLUGINS 섹션(`widgets/plugin-manager`)에 목록(활성/비활성·오류 표면화)·설치·제거·
  "플러그인 폴더 열기"가 있다. 목록 항목의 오류(`grammarMissing`·`grammarInvalid`·`grammarConflict`
  등, §3)는 플러그인별로 인라인 표시된다.
- `plugin_install(sourcePath)` — 디렉토리 또는 `.zip`/`.vsix` 아카이브를 받는다. **설치 UI 는
  아카이브 전용 파일 피커만 제공한다** — Tauri `@tauri-apps/plugin-dialog` 의 `open()` 은 파일
  선택과 폴더 선택을 한 네이티브 호출에서 동시에 지원하지 않기 때문이다. 디렉토리 형태 플러그인은
  "플러그인 폴더 열기"로 수동 배치한 뒤 "다시 읽기"로 인식시킨다(별도 폴더 피커는 후속 개선 여지).
  이미 설치된 `id` 는 거부한다(업그레이드는 제거 후 재설치만 가능 — 계약이 upgrade 시맨틱을 정하지
  않아 가장 안전한 기본값을 택했다).
- `plugin_uninstall(pluginId)` — 빌트인 보호는 없다(사용자 디렉토리만 다루므로 애초에 빌트인을
  지울 수 없다). 확인 다이얼로그를 거친다.
- 비활성 토글(`disabledPlugins`)은 이번에도 도입하지 않았다 — 삭제로 갈음한다(1차 방침 유지, E1).

### 6.2 VSIX → 플러그인 (grammar/languages 임포트)

- `vsix_import_plugin(vsixPath)` — 실제 VS Code 확장의 `contributes.languages`/
  `contributes.grammars` 를 읽어 `taide-plugin.json` 을 **합성**한 뒤, `plugin_install` 과 완전히
  동일한 검증·등록 경로(`plugins/{publisher}-{name}/`)로 착지시킨다. 부분 선택 없이 "그 확장이 기여한
  언어 전부"를 한 번에 들여오는 all-or-nothing 동작이다.
  - `AppError` 는 실패 사유를 세분화된 코드로 주지 않으므로(Io/NotFound/InvalidArgument/Forbidden/
    Internal 뿐), 설치 UI(`VsixImportGrammarsSection`)는 모든 실패를 "이 확장에는 언어 기여가
    없습니다"(`pluginImportVsixNoGrammars`) 한 메시지로 보여준다 — 문자열 매칭으로 사유를 추측하는
    편법 대신 의도적으로 단순화한 것이다. 이 다이얼로그가 열리는 시점엔 이미 `vsix_extract_themes`
    가 아카이브 자체는 파싱 가능함을 증명했으므로, 실제로 가장 흔한 실패는 "그냥 테마 전용 확장이라
    contributes 가 비어 있음"이다.
- 기존 테마 전용 임포트 플로우(`vsix-theme-import.md`)와 **하나의 다이얼로그**로 통합됐다 — Themes
  섹션(기존 후보 선택 로직 그대로)과 Grammars 섹션(위 tri-state)이 같은 다이얼로그 안에 나란히
  있다.

### 6.3 zip 하드닝 — `infra::archive::extract_hardened_zip`

- `plugin_install`(아카이브 경로)·`vsix_import_plugin` 이 공유하는 전용 추출 함수. 엔트리 수 상한
  5000(`ARCHIVE_MAX_ENTRIES`)·누적 압축 해제 바이트 예산 128MB(`ARCHIVE_MAX_TOTAL_BYTES` — zip
  bomb 방어)·`enclosed_name()` 기반 zip-slip 방어·파일/
  디렉토리 모드를 `0o644`/`0o755` 로 고정(아카이브 안의 unix 모드 비트를 신뢰하지 않는다).
- 기존 `infra::lsp_install::extract_zip` 은 **재사용하지 않았다** — 그건 신뢰된 자체 배포 아카이브
  전용(zip-slip 만 방어)이고, 사용자가 임의로 고른 `.vsix`/`.zip` 은 신뢰 전제가 다르다.

### 6.4 동적 언어 등록 (C1 — monaco/shiki 게이트 해소)

> §2 확정 사실 8의 핵심 결함: 플러그인이 새 언어를 기여해도 monaco 가 그 언어 id 를 모르면(고정
> 32종 등록 루프) 에디터가 plaintext 로만 표시했다.

- `shared/lib/monaco/register-plugin-languages.ts` 의 `registerPluginLanguages(plugins)` 가 활성
  플러그인의 `contributes.languages[]` 를 순회해 `monaco.languages.register` 로 등록한다(이미
  등록된 id 는 건너뛰어 중복/재등록 없음).
- 이어서 grammar 를 shiki 하이라이터에 반영하기 위해 **전체 재생성**(`reinitShiki`/`initShiki`,
  증분 아님 — 플러그인 설치는 드문 이벤트라 증분 로드는 이번 범위에서 기각했다, H2 backlog).
  재생성이 싣는 TAIDE grammar 는 "지금까지 요구된 언어" 집합이다(d-51 F7 — `docs/theme-system.md`
  §4.2 grammar 온디맨드 로드). 플러그인 grammar 의 `embeddedLangs` 가 가리키는 TAIDE 언어는 재생성
  전에 그 집합에 합류하므로, 온디맨드 전환이 플러그인 문법의 임베딩을 떨어뜨리지 않는다.
- 이 두 단계는 `plugin_install`·`plugin_uninstall`·`plugin_reload`·`vsix_import_plugin` 4개
  뮤테이션의 성공 콜백과 앱 부팅(`theme-provider.tsx`) 양쪽에서 공유된다(`entities/plugin/
  plugin.query.ts` 의 `applyPluginList`/`refetchAndApplyPluginList`) — 어느 경로로 플러그인 목록이
  바뀌어도 monaco 언어 등록과 shiki 하이라이팅이 항상 최신 목록과 정합한다.

**재생성 시 옛 tokens provider 정리** (d-51 F6 · 감사 §4-B D6)

- `shikiToMonaco` 는 하이라이터가 로드한 언어마다 `monaco.languages.setTokensProvider` 를 부르고
  **그 disposable 을 돌려주지 않는다.** 그래서 문법이 줄어드는 재생성(=그 언어를 기여하던 플러그인
  제거)에서는 사라진 언어의 provider 가 **곧 dispose 될 하이라이터를 붙든 채** 남았다. monaco 는 언어
  등록을 해제할 수 없어 그 파일은 계속 그 언어 id 로 열리고, 매 줄 토큰화가
  `ShikiError: Shiki instance has been disposed` 로 터졌다.
- `shared/lib/shiki/tokens-provider-registry.ts` 의 `swapTokensProviderRegistrations` 가
  `shikiToMonaco` 호출 동안 등록을 가로채 모아 두고, **새 등록이 끝난 뒤** 직전 등록 묶음을 dispose
  한다. monaco 의 `TokenizationRegistry` dispose 는 동일성 검사를 하므로 재등록된 언어는 새 provider
  를 유지하고 사라진 언어만 실제로 해제돼 monaco 기본(plain) 토큰화로 떨어진다. 재생성 자체가
  실패해 하이라이터가 없는 경우에도 같은 정리를 수행한다.

**플러그인 언어의 스니펫 완성** (d-51 F6 · 감사 §4-B D6)

- 스니펫 완성 provider 는 `'*'` 가 아니라 **언어 id 별로** 등록된다(`snippet-completion.ts` — 그
  근거는 `settings-ui`/스니펫 문서 참조). 부팅 시 등록 대상이 `TAIDE_LANGUAGE_IDS` 고정 목록뿐이라
  플러그인 언어에는 provider 자체가 없었고, `<languageId>.json` 스니펫 파일도 전역 `.code-snippets`
  의 `scope` 지정도 그 언어에서는 전혀 뜨지 않았다.
- `registerPluginLanguages` 가 새로 등록한 언어 id 를 `registerSnippetCompletionsForLanguages` 로
  넘겨 같은 provider 를 붙인다(중복 등록 없음, 부트스트랩 순서와 무관).

### 6.5 확장자 → 언어 테이블 통합 (D1)

- `infra::language::language_id_for_path` 하나가 `file::service`·`git::service`·`ide::service`
  3곳이 각자 갖고 있던 확장자→언어id 테이블을 대체한다(플러그인 오버레이 포함). 프론트 사본
  (`shared/lib/language-from-path.ts`)은 별도로 유지된다 — git blob diff 의 언어 판정처럼 Rust IPC
  왕복이 없는 경로가 있어 손으로 포트된 채로 남는다(D2 는 기각 — Rust 단일 소유가 원칙이지만 이
  프론트 사본만은 이미 존재하던 예외).
- 통합 과정에서 `ide::service` 가 예전에만 갖고 있던 `csharp`/`php`/`sql` 매핑을 의도적으로
  드롭했다 — 셋 다 shiki grammar 가 없어(`TAIDE_LANGUAGE_IDS` 밖) 애초에 렌더 불가능한 죽은 언어
  id 였고, `file`/`git` 서비스는 원래도 이 셋을 plaintext 로 취급했다. IDE MCP 클라이언트가 이
  3개 확장자에 옛 문자열을 기대하고 있었다면 그 경로만 회귀로 보일 수 있다(알려진 트레이드오프).

### 6.6 원격 정책

- `plugin_install`·`plugin_uninstall`·`vsix_import_plugin` 은 원격에서 **명시 거부**된다 — 파일
  경로/네이티브 다이얼로그가 필요한 로컬 전용 동작이기 때문이다.
- `vsix_extract_themes` 도 이번에 **허용 → 거부로 전환**했다 — 이전에는 원격 세션도 로컬 임의 파일
  경로를 zip 으로 열어 읽을 수 있는 제한적 임의 파일 읽기 표면이었다(§2 확정 사실 8). 이 전환은
  회귀가 아니라 의도된 보안 정책 변경이다.
- `plugin_list`/`plugin_reload` 는 원격에서도 계속 허용된다(읽기 전용, 로컬 자원 접근 없음).
