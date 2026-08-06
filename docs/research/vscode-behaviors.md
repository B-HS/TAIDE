# VSCode / Cursor / GitLens 참조 동작 정밀 명세 (TAIDE 복제 대상)

조사일: 2026-08-06. 아래 내용은 공식 docs(code.visualstudio.com), GitLens 공식 헬프(help.gitkraken.com) 및 GitLens 저장소 `package.json` 원본, Git Graph 위키에서 확인한 사실만 기재한다. 확인하지 못한 항목은 `미확인`으로 표기했다.

---

## 버전 확정 (2026-08 기준)

| 대상 | 버전 | 확인 근거 / 비고 |
|---|---|---|
| VS Code | 릴리즈 아카이브 상 최신 stable = **1.132**, Insiders = **1.133** | code.visualstudio.com/updates/archive 목록. 단 아카이브에서 **월↔버전 매핑이 명시된 최신 항목은 "February 2026 (version 1.110)"** 이며, 1.111~1.133 은 버전 번호만 노출되어 각 버전의 정확한 릴리즈 월은 **미확인**. "2026-08 시점 stable 이 정확히 몇인지"는 단정하지 않는다. |
| GitLens (gitkraken/vscode-gitlens) | **18.2.0** | 저장소 `main` 브랜치 `package.json` 의 `version` 필드 직접 확인 |
| Git Graph (mhutchie/vscode-git-graph) | 버전 번호 **미확인** (레포는 유지보수 중단 상태로 커뮤니티 이슈 #838 "Git Graph is abandoned" 존재) | 위키 문서는 유효. VS Code 는 자체 **Source Control Graph** 를 내장 기능으로 제공 중 |
| Cursor | **v3.11** (2026-07-10) 이 확인된 최신 릴리즈. 2026-07-29 자 iPad 릴리즈 등 이후 업데이트 존재 | cursor.com/changelog 기반 2차 출처. 3.x 계열에서 "New Cursor Interface"(3.0) 로 UI 개편됨 |

> 주의: VS Code 버전 번호가 2026 년에 월 1회 마이너 증가 규칙을 유지하는지 여부를 공식 문서에서 확정하지 못했다. TAIDE 문서에서 "VS Code 1.1xx 기준" 이라고 쓸 때는 위 불확실성을 함께 표기한다.

---

## 핵심 API·사용법

### 1. SCM(Source Control) 패널 구조

VS Code 의 Source Control 뷰는 저장소 상태를 **세 종류의 리소스 그룹**으로 나눈다 (공식 문서 표현: "unstaged", "staged", "unresolved conflicting merge" changes).

- `Changes` — 스테이지되지 않은 변경
- `Staged Changes` — 스테이지된 변경
- `Merge Changes` — 해결되지 않은 병합 충돌

각 그룹 헤더에는 카운트 배지가 붙고, Activity Bar 의 Source Control 아이콘에는 영향 파일 수 배지가 표시된다.

#### 상태 문자 (status letter)

공식 문서에서 **명시적으로 확인된 것은 `M`(modified), `U`(untracked)** 두 가지다("Modified files display an 'M' icon, while untracked files show a 'U' icon"). `A`/`D`/`R`/`C` 는 git 표준 status code(Added / Deleted / Renamed / Copied)와 동일하게 VS Code 가 렌더링하지만, **문서상 명시 확인 실패 → 미확인**. 색상 토큰은 테마 색상 키로 정의되어 있다(아래는 VS Code 테마 색상 명명 규칙 기준이며, 개별 키의 문서 확인은 부분적):

| 상태 | 문자 | 의미 | TAIDE 권장 색 역할 |
|---|---|---|---|
| Modified | `M` | 추적 중 파일 내용 변경 | 주황/노랑 계열 (`gitDecoration.modifiedResourceForeground`) |
| Added / Untracked | `A` / `U` | 신규 추가 / 미추적 | 초록 계열 (`gitDecoration.addedResourceForeground`, `untrackedResourceForeground`) |
| Deleted | `D` | 삭제 | 빨강 계열 (`gitDecoration.deletedResourceForeground`) |
| Renamed | `R` | 이름 변경 | 초록/청록 계열 |
| Copied | `C` | 복사 | 초록 계열 |
| Conflict | `!` / `C` | 병합 충돌 | 빨강 계열 (`gitDecoration.conflictingResourceForeground`) |

> 정확한 테마 색상 키 목록은 VS Code Theme Color Reference 의 "Git colors" 섹션에서 재확인 필요. 본 조사에서는 키 이름의 문서 원문 확인 **미완료**.

#### 파일 row 액션 아이콘 (hover 시 노출)

- `Changes` 그룹의 파일: `+` (Stage Changes), `↺` (Discard Changes), `Open File`
- `Staged Changes` 그룹의 파일: `−` (Unstage Changes), `Open File`
- 그룹 헤더 hover: `+` (Stage All Changes) / `−` (Unstage All Changes)
- 파일 행 클릭 = **Open Changes**(diff 에디터 열기, preview 탭)

### 2. SCM 파일 context menu

공식 문서에서 확인된 항목:

```
Open File
Open Changes
Stage Changes          (Changes 그룹)
Unstage Changes        (Staged Changes 그룹)
Discard Changes        (휴지통/Recycle Bin 으로 이동)
Stage All Changes
```

diff 에디터 내 선택 영역 우클릭:

```
Stage Selected Ranges
```

그 외(`Add to .gitignore`, `Open Timeline`, `Revert Selected Ranges` 등)는 실제 구현에 존재하나 **공식 문서 원문 확인 실패 → 미확인**으로 둔다.

### 3. 에디터 gutter 변경 표시(dirty diff)

공식 문서 원문:

- "A **red triangle** indicates where lines have been deleted"
- "A **green bar** indicates new added lines"
- "A **blue bar** indicates modified lines"

즉 **추가=초록 바, 수정=파랑 바, 삭제=빨강 삼각형(화살표 형태)** 이다. 동일 정보가 **overview ruler**(스크롤바 영역)에도 표시된다("VS Code will add useful annotations to the gutter and to the overview ruler").

gutter 마커 클릭 시 **inline change peek** 위젯이 열리며, 위젯 내부에서 이전/다음 변경 이동, 되돌리기, 스테이징 등의 액션을 제공한다. (동작 자체는 널리 알려진 사양이나, peek 위젯의 버튼 구성 원문 확인은 **미확인**)

TAIDE 구현 규격 제안:

```ts
type GutterChange =
    | { kind: 'added'; startLine: number; endLine: number }
    | { kind: 'modified'; startLine: number; endLine: number }
    | { kind: 'deleted'; atLine: number }

const GUTTER_STYLE = {
    added: { width: 3, color: 'var(--taide-gutter-added)' },
    modified: { width: 3, color: 'var(--taide-gutter-modified)' },
    deleted: { shape: 'triangle', size: 6, color: 'var(--taide-gutter-deleted)' },
} as const
```

`deleted` 는 라인 사이 경계에 붙는 삼각형이므로 라인 높이를 차지하지 않고 **경계 y 좌표**에 렌더한다.

### 4. Diff 뷰 (side-by-side / inline)

- 기본값: **side-by-side**
- 설정 키: `diffEditor.renderSideBySide` (`false` 로 두면 inline 이 기본)
- 토글 경로: diff 에디터 툴바의 `More Actions (...)` → **Inline View**
- 단축키: macOS `⌥\`, Windows/Linux `Alt+\`
- 툴바에 **Collapse Unchanged Regions** 버튼 존재
- 접근성 diff 뷰어: `F7`

```jsonc
{
    "diffEditor.renderSideBySide": true,
    "diffEditor.ignoreTrimWhitespace": true
}
```

### 5. Commit UI 플로우

1. Source Control 뷰 상단의 **메시지 입력 박스**에 커밋 메시지 입력. AI 생성은 sparkle 아이콘.
2. 커밋 단축키: `⌘Enter` (macOS) / `Ctrl+Enter` (Windows/Linux).
   - 문서 원문: "If there are any staged changes, only those will be committed, otherwise all changes will be committed."
   - 단 최신 문서는 "Only staged changes are included in a commit." 로 기술되어 있어 **버전에 따라 동작 차이 가능 → 두 문장 모두 원문 존재**. TAIDE 는 "스테이지된 것이 있으면 그것만, 없으면 전체" 를 기본으로 하되 설정으로 노출할 것을 권장.
3. **Commit 버튼 드롭다운 / More Actions** 에서 확인된 항목:
   - `Commit`
   - `Commit (Amend)`
   - `Commit All`
   - `Commit Staged (Amend)`
   - `Undo Last Commit`
   - `Commit & Push`, `Commit & Sync`, `Commit Staged (Signed Off)` 는 실제 구현에 존재하지만 이번 조사에서 **문서 원문 확인 실패 → 미확인**
4. **Sync Changes** 버튼: pull + push 를 동시에 수행("pulls latest remote changes and pushes local commits simultaneously"). 개별 실행은 `...` (More Actions) → `Pull` / `Push`.
5. 상태 바에 sync 상태(incoming/outgoing 커밋 수)가 표시된다.

### 6. Git Graph — 레인 배치와 표시 요소

VS Code 는 내장 **Source Control Graph** 를 제공하며("a graphical representation of your commit history and branch relationships"), 확장 Git Graph(mhutchie)의 설정에서 렌더링 규격을 확인할 수 있다.

| 설정 | 기본값 | 의미 |
|---|---|---|
| `git-graph.graph.style` | `rounded` | 브랜치 전이 곡선. `rounded` / `angular` |
| `git-graph.graph.colours` | `["#0085d9","#d9008f","#00d90a","#d98500","#a300d9","#ff0000","#00d9cc","#e138e8","#85d900","#dc5b23","#6f24d6","#ffcc00"]` | 레인 색 팔레트(12색 순환) |
| `git-graph.graph.uncommittedChanges` | Open Circle at the Uncommitted Changes | 미커밋 변경을 그래프 최상단 회색 원으로 표시 |
| `git-graph.repository.commits.order` | `date` | `date` / `author-date` / `topo` |
| `git-graph.referenceLabels.alignment` | `Normal` | 브랜치/태그 라벨 정렬. 브랜치 좌측·태그 우측 등 |
| `git-graph.referenceLabels.combineLocalAndRemoteBranchLabels` | `true` | 로컬/원격 동일 브랜치 라벨 병합 |

레인 배치 알고리즘 자체(어느 커밋이 몇 번 레인을 차지하는지의 정확한 규칙)는 문서화되어 있지 않아 **미확인**. 일반적 구현 방식(TAIDE 구현 지침으로 사용):

1. 커밋을 설정된 order(기본 date desc)로 정렬한다.
2. 위에서 아래로 순회하며 "열려 있는 레인" 집합을 유지한다.
3. 커밋이 어떤 열린 레인의 기대 부모이면 그 레인을 차지하고, 아니면 **가장 왼쪽의 빈 레인**을 새로 할당한다.
4. 머지 커밋은 첫 부모를 자기 레인으로 잇고, 나머지 부모마다 새 레인을 열어 곡선으로 연결한 뒤 병합 지점에서 닫는다.
5. 레인 색 = `colours[laneIndex % colours.length]`.

표시 요소: 커밋 점, 레인 곡선, **브랜치 라벨**(로컬/원격), **태그 라벨**, **HEAD 표시**(Git Graph 는 `Ctrl/Cmd + H` 로 HEAD 커밋으로 스크롤 센터링), 미커밋 변경 행, 커밋 메시지/작성자/날짜/해시 컬럼.

### 7. GitLens 인라인 blame 표기 형식

GitLens 18.2.0 `package.json` 에서 직접 확인한 **기본 포맷 문자열**:

```jsonc
{
    "gitlens.currentLine.format": "${author, }${agoOrDate}${' via  'pullRequest}${ • message|50?}",
    "gitlens.statusBar.format": "${author}, ${agoOrDate}${' via  'pullRequest}",
    "gitlens.blame.format": "${message|50?} ${agoOrDate|14-}",
    "gitlens.currentLine.enabled": true,
    "gitlens.currentLine.scrollable": true,
    "gitlens.hovers.currentLine.over": "annotation"
}
```

해석:

- `currentLine.format` 은 **현재 커서 라인 끝**에 흐린 텍스트로 붙는다. 렌더 결과 예:
  - `You, 3 days ago • fix: 로그인 리다이렉트 수정`
  - PR 연결 시: `You, 3 days ago via  #1234 • fix: ...`
  - 미커밋: `You, seconds ago - Uncommitted changes`
- 토큰 문법
  - `${author, }` — author 뒤에 리터럴 `, ` 를 붙이되 author 가 없으면 통째로 생략
  - `${agoOrDate}` — 상대시간("3 days ago") 또는 절대일자. `gitlens.defaultDateStyle` 로 결정(기본값 문서 확인 **미확인**, 통상 `relative`)
  - `${ • message|50?}` — 메시지를 50자로 자르고, 값이 없으면 접두 리터럴까지 생략(`?` = optional)
  - `|14-` — 14자 폭 좌측 정렬 패딩
- 본인 커밋의 author 는 `You` 로 치환되어 표시된다.
- `blame.format`(파일 전체 gutter blame, `Alt+B` / `gitlens.toggleFileBlame`) 은 **메시지 우선 + 상대시간** 순서로 다르다는 점에 주의.
- 상태 바 blame: "author and date of the last commit affecting the current line", 클릭 시 quick pick 액션 메뉴.
- **hover 상세**: 현재 라인 hover 시 커밋 상세(메시지 전문, author/이메일, 커밋 SHA 클릭 가능, 날짜), 이슈 자동 링크, quick-action 바를 표시한다. `hovers.currentLine.over` 기본값 `annotation` = 주석 텍스트 위에 hover 해야 뜸(라인 전체가 아님).
- Git CodeLens: 파일/블록 상단에 `Recent Change`(최근 커밋 author + date), `Authors`(기여자 수 + 대표 author). 토글 `Shift+Alt+B` / `gitlens.toggleCodeLens`.

### 8. 필수 단축키 (VS Code 기본값)

| 기능 | macOS | Windows/Linux |
|---|---|---|
| Insert Cursor Below | `⌥⌘↓` | `Ctrl+Alt+Down` |
| Insert Cursor Above | `⌥⌘↑` | `Ctrl+Alt+Up` |
| Add Selection to Next Find Match | `⌘D` | `Ctrl+D` |
| Select All Occurrences | `⇧⌘L` | `Ctrl+Shift+L` |
| Move Line Down / Up | `⌥↓` / `⌥↑` | `Alt+Down` / `Alt+Up` |
| Copy Line Down / Up | `⇧⌥↓` / `⇧⌥↑` | `Shift+Alt+Down` / `Shift+Alt+Up` |
| Delete Line | `⇧⌘K` | `Ctrl+Shift+K` |
| Toggle Line Comment | `⌘/` | `Ctrl+/` |
| Toggle Block Comment | `⇧⌥A` | `Shift+Alt+A` |
| Find | `⌘F` | `Ctrl+F` |
| Replace | `⌥⌘F` | `Ctrl+H` |
| Go to Definition | `F12` | `F12` |
| Peek Definition | `⌥F12` | `Alt+F12` |
| Go to References | `⇧F12` | `Shift+F12` |
| Rename Symbol | `F2` | `F2` |
| Format Document | `⇧⌥F` | `Ctrl+Shift+I` |
| Format Selection | `⌘K ⌘F` | `Ctrl+K Ctrl+F` |
| Quick Fix | `⌘.` | `Ctrl+.` |
| Command Palette | `⇧⌘P` | `Ctrl+Shift+P` |
| Go to File | `⌘P` | `Ctrl+P` |
| Go to Symbol | `⇧⌘O` | `Ctrl+Shift+O` |
| Go to Line | `⌃G` | `Ctrl+G` |
| Split Editor | `⌘\` | `Ctrl+\` |
| Close Editor | `⌘W` | `Ctrl+W` |
| Toggle Sidebar | `⌘B` | `Ctrl+B` |
| Show Explorer | `⇧⌘E` | `Ctrl+Shift+E` |
| Show Search | `⇧⌘F` | `Ctrl+Shift+F` |
| Show Source Control | `⌃⇧G` | `Ctrl+Shift+G` |
| Show Run and Debug | `⇧⌘D` | `Ctrl+Shift+D` |
| Show Extensions | `⇧⌘X` | `Ctrl+Shift+X` |
| Commit (SCM 입력창) | `⌘Enter` | `Ctrl+Enter` |
| Diff inline/side-by-side 토글 | `⌥\` | `Alt+\` |
| Accessible Diff Viewer | `F7` | `F7` |

컬럼(박스) 선택은 macOS `⇧⌥드래그`, Windows/Linux `Shift+Alt+드래그` 로 널리 쓰이나 이번 조사에서 문서 원문 확인 **미확인**.

### 9. 사이드바 / Activity Bar 구성

공식 문서 기준 Activity Bar 항목(기본 순서):

1. **Explorer** — 파일/폴더 탐색·관리
2. **Search** — 전역 검색/치환
3. **Source Control** — Git 통합 (기본 SCM 공급자)
4. **Run** (Run and Debug) — 변수, 콜스택, 브레이크포인트
5. **Extensions** — 확장 설치/관리
6. **Chat** — **기본적으로 Secondary Side Bar 에 위치**

- Activity Bar 아이콘은 **드래그로 순서 변경** 가능하고, 우클릭 시 커스터마이즈 context menu 가 열린다.
- **Primary Side Bar**(좌측, 기본 Explorer) / **Secondary Side Bar**(우측, 기본 Chat). 원문: "Drag and drop views from the Primary Side Bar to the Secondary Side Bar to move them".
- Explorer 안에 **Open Editors** 섹션이 있어 에디터 그룹 구조를 볼 수 있다.
- Cursor 는 3.0 "New Cursor Interface" 로 UI 를 개편했고, 3.11(2026-07-10)에서 side chat(`/side`, `/btw`, 채팅 패널 상단 `+`)과 사이드바 채팅 핀 고정을 도입했다. Cursor 사이드바 아이콘의 정확한 기본 구성은 **미확인**.

### 10. 탭 동작

- **Preview 탭**: 원문 "When you single-click or select a file in the Explorer view, it is shown in a preview mode and reuses an existing tab (preview tab)." 프리뷰 탭 제목은 **이탤릭**으로 렌더된다. 더블클릭하거나 편집을 시작하면 고정(비프리뷰) 탭으로 승격되며 다음 파일이 새 탭으로 열린다.
- **Pin**: 탭을 고정하면 탭 바에 항상 남는다(스크롤 아웃·자동 닫힘 방지).
- **Dirty 표시**: 저장되지 않은 파일은 탭에 **점(dot)** 이 표시된다.
- **드래그**: 탭 드래그로 순서 변경, 그리고 에디터 영역 가장자리로 끌면 **에디터 그룹이 여러 방향으로 분할**된다("You can drag tabs to reorder them or split editor groups in different directions"). 분할 시 "a new editor region (edit group) is created which can hold a group of items."

---

## TAIDE 적용 가이드

1. **SCM 상태 모델**: Rust(Tauri) 측에서 `git2` 또는 `git status --porcelain=v2` 결과를 `{ path, indexStatus, worktreeStatus }` 로 정규화해 프론트에 넘기고, 프론트에서 `Merge Changes` / `Staged Changes` / `Changes` 세 그룹으로 분류한다. 충돌(`UU`, `AA`, `DU` 등)은 무조건 Merge Changes 로 보낸다.
2. **상태 문자·색은 테마 토큰으로**: `M`/`A`/`D`/`U`/`R`/`C` 문자와 색을 하드코딩하지 말고 CSS 변수(`--taide-git-modified` 등)로 두어 라이트/다크 대응을 강제한다. 개인 컨벤션상 하드코딩 색은 다크모드 수동 대응이 필요하므로 토큰화가 필수다.
3. **Gutter**: 추가/수정은 3px 세로 바, 삭제는 라인 경계의 삼각형. overview ruler(스크롤바) 미니 표시도 함께 구현해야 VSCode 체감이 난다.
4. **Diff**: 기본 side-by-side, `Alt+\` / `⌥\` 로 inline 토글, 설정 키는 VSCode 와 동일한 `diffEditor.renderSideBySide` 명명을 따라 사용자 이전 학습을 재활용한다.
5. **Commit UI**: 입력창 + `⌘Enter` + 드롭다운(Amend / Commit All / Undo Last Commit / Commit & Push / Commit & Sync). 최소 1차 구현은 `Commit`, `Commit (Amend)`, `Commit All`, `Undo Last Commit` 네 개면 충분하다.
6. **Blame**: GitLens 포맷 문자열을 그대로 채택하되(`${author, }${agoOrDate}${ • message|50?}`), 토큰 파서를 작게 구현한다. `You` 치환, `?`(optional), `|50`(truncate), `|14-`(pad) 세 가지 수식자만 지원해도 기본 UX 는 동일해진다.
7. **그래프**: 위 7 절의 5단계 레인 할당 알고리즘 + 12색 팔레트 순환으로 시작한다. `rounded` 곡선(SVG `path` 의 `Q`/`C`)이 기본.
8. **탭**: preview(이탤릭) → 더블클릭/편집 시 승격, pin, dirty dot, 드래그 분할. preview 는 "탭 1개 재사용" 이 핵심이므로 상태 모델에 `previewTabId: string | null` 을 둔다.
9. **단축키**: 위 8 절 표를 그대로 기본 키맵으로 채택. Tauri 에서는 브라우저 기본 동작(`⌘W` 창 닫기, `⌘P` 인쇄 등)과 충돌하므로 `preventDefault` 및 Tauri 메뉴 accelerator 조정이 필요하다.

---

## 함정·주의

- **`⌘W`, `⌘P`, `⌘F`, `Ctrl+/`** 등은 웹뷰/OS 기본 동작과 충돌한다. Tauri 2 에서는 앱 메뉴 accelerator 로 선점하거나 `keydown` 캡처 단계에서 `preventDefault()` 해야 한다.
- **커밋 대상 규칙이 문서 간 불일치**한다. 구 문서는 "스테이지된 게 있으면 그것만, 없으면 전체", 최신 문서는 "스테이지된 것만". 어느 쪽을 복제할지 제품 결정이 필요하며, 임의 선택 금지 — 설정으로 노출하는 편이 안전하다.
- **`Discard Changes` 는 파일을 휴지통(Recycle Bin/Trash)으로 보낸다**(untracked 의 경우). 즉시 삭제가 아니다. 복제 시 이 안전장치를 빠뜨리면 데이터 손실 사고가 난다.
- **GitLens `blame.format` 과 `currentLine.format` 의 토큰 순서가 다르다.** 파일 blame 은 메시지 우선, 인라인은 author 우선. 한쪽 포맷을 양쪽에 재사용하면 GitLens 사용자에게 어색하다.
- **`gitlens.hovers.currentLine.over` 기본값이 `annotation`** 이라, 라인 어디에나 hover 해도 뜨는 것이 아니라 **주석 텍스트 위**에서만 뜬다. 이 미묘한 차이를 놓치면 "hover 가 너무 자주 뜬다" 는 UX 문제가 생긴다.
- **Git Graph 확장은 유지보수 중단 상태**(이슈 #838). 이 확장을 사양의 유일한 근거로 삼지 말고, VS Code 내장 Source Control Graph 동작도 함께 참조할 것.
- **레인 배치 알고리즘은 공식 명세가 없다.** 본 문서의 5단계는 일반적 구현 지침이지 VS Code/Git Graph 의 실제 알고리즘을 확인한 것이 아니다.
- **VS Code 버전-월 매핑을 단정하지 말 것.** 아카이브에서 월이 병기된 최신 항목은 1.110(2026-02)이다.
- **Cursor UI 는 3.x 에서 크게 바뀌었다.** 구버전 스크린샷/블로그 기반으로 사이드바를 복제하면 현행과 어긋난다.

---

## 참고 링크

- Source Control in VS Code — https://code.visualstudio.com/docs/sourcecontrol/overview
- Intro to Git in VS Code (staging / commit / sync) — https://code.visualstudio.com/docs/sourcecontrol/intro-to-git
- Staging and committing changes — https://code.visualstudio.com/docs/sourcecontrol/staging-commits
- Version Control (gutter 색상 원문: red triangle / green bar / blue bar) — https://vscode-docs.readthedocs.io/en/latest/editor/versioncontrol/
- VS Code User Interface (Activity Bar, side bars, 탭 preview/pin/dirty, 그룹 분할) — https://code.visualstudio.com/docs/getstarted/userinterface
- Default Keybindings — https://code.visualstudio.com/docs/reference/default-keybindings
- VS Code Release Notes Archive — https://code.visualstudio.com/updates/archive
- VS Code Tips and Tricks — https://code.visualstudio.com/docs/editing/tips-and-tricks
- GitLens Core Features — https://help.gitkraken.com/gitlens/gitlens-features/
- GitLens Settings — https://help.gitkraken.com/gitlens/gitlens-settings/
- GitLens `package.json` (포맷 기본값·버전 18.2.0 원본) — https://raw.githubusercontent.com/gitkraken/vscode-gitlens/main/package.json
- GitLens Custom Formatting / Commit Tokens 위키 — https://github.com/eamodio/vscode-gitlens/wiki/Custom-Formatting#commit-tokens
- Git Graph Extension Settings 위키 — https://github.com/mhutchie/vscode-git-graph/wiki/Extension-Settings
- Git Graph 저장소 — https://github.com/mhutchie/vscode-git-graph
- Git Graph 유지보수 중단 이슈 #838 — https://github.com/mhutchie/vscode-git-graph/issues/838
- Cursor Changelog — https://cursor.com/changelog
- Cursor 3.0 New Interface — https://cursor.com/changelog/3-0
