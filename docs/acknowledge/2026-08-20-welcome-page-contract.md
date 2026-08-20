# Welcome 페이지 확충 계약 (2026-08-20, d-27)

> 사용자 요청(2026-08-20): "Welcome page 에 file 열기, folder 열기, 최근에 열었던 프로젝트들
> (열려있다면 그 프로젝트로 탭변경) 기타 넣을만한것들 해서 꾸며서 넣어놔."
> 착수 전 메인 실사: `features/welcome/welcome-screen.tsx`(54줄) 기존재 — 폴더 열기(⌘O)+최근
> 목록 UI 는 있으나 **app-shell 이 `recentProjects={[]}` 하드코딩**(배선 부재). TabKind 에
> `{kind:"welcome"}` 탭 실재(사용자 스크린샷의 "Welcome ×"). 영속 프로젝트 기록
> (`~/Library/Application Support/dev.taide.app/projects/*/project.json`)은 id·root·name·
> capabilities·rootMissing 만 보유 — **최근 정렬용 타임스탬프 부재**(Rust 스키마 확장 필요).
> `project_list` 는 세션 프로젝트 대상(전 기록 열람 커맨드 부재 정황 — 구현 실사).

## 1. 범위

### 1.1 Rust — 최근 프로젝트 데이터 (표면 추가)

- 프로젝트 영속 기록에 `last_opened_at`(밀리초 epoch — IPC 시간 f64 규칙) 필드 추가(serde
  default 로 기존 파일 무해 호환). `project_open`(기존 재열기 포함)·`project_activate` 시 갱신.
- 신규 커맨드 `project_list_recent`(영속 기록 전수 — 최근순·rootMissing 포함) — **표면 추가
  절차 전수**: 배선 3곳 + 파리티 + T1-K ALLOWED/DENIED 등재(**DENIED 추천** — 로컬 웰컴 UI
  전용·원격 불요) + bindings 재생성(cargo test) + ipc-contract 문서. 기존 `project_list`
  의미 무변경.
- 도메인 경계: project 도메인 내 완결(화이트리스트 무변경 기대).

### 1.2 TS — Welcome 화면 확충 ("꾸며서")

- **폴더 열기**: 기존 유지(⌘O 라벨).
- **파일 열기(신규)**: 추천안 — 활성 프로젝트가 있으면 그 루트 기준 파일 선택 dialog →
  에디터 탭으로 열기(root guard 정합 — 프로젝트 밖 파일은 기존 가드 의미 유지 실사). 열린
  프로젝트가 없으면 비활성+힌트(단일 파일 편집은 프로젝트 모델 밖 — 임의 확장 금지).
- **최근 프로젝트 목록(배선 완성)**: `project_list_recent` 소비 — **이미 열려 있으면
  `project_activate` 로 전환**(사용자 명시 요구), 닫혀 있으면 `project_open(root)`.
  rootMissing 기록은 비활성 표시+사유. 표시는 이름 주 + 루트 경로 부제(d-26 2단 선례).
- **기타("넣을만한것들")** — 추천 구성: 주요 단축키 안내 카드(⌘P·⌘⇧F·⌃` 등 실제 APP_KEYMAP
  에서 유도 — 하드코딩 금지), 버전 표기는 실소비 부재 정황(app_get_info 프론트 소비 0) 실사
  후 필요 시만. 과설계 금지 — 문서 링크·뉴스 피드 류 외부 의존 금지.
- **적용 면 2곳 통일**: 프로젝트 0개 화면(app-shell)과 welcome 탭(kind:"welcome" 렌더러 —
  구현 실사로 위치 특정) 이 같은 컴포넌트를 쓰도록 정리(중복 금지). features 레이어 순수성
  유지 — 데이터·콜백은 상위(widgets/app-shell)에서 주입(FSD).
- i18n 신규 키 4곳 동기·3언어 실번역. 접근성(button·label) 기본 준수.

### 1.3 범위 외

- 원격 표면 확대(신규 커맨드는 DENIED)·프로젝트 삭제/고정 등 관리 기능·뉴스/업데이트 피드.
- 기존 프로젝트 기록 마이그레이션(장식 필드라 default 로 충분).

## 2. 실행·검증

- **선행: d-26b(테마)·d-26 검토 반영 완결 후 착수**(동일 트리 순차). Rust 포함 —
  tauri dev 재빌드로 앱 재시작 고지.
- 구현 Workflow(sonnet+xhigh 단독 — Rust 선행 후 TS 순차) → 메인 2차 → Phase E 4렌즈
  (정확성: 타임스탬프 갱신 지점·activate 전환 / 표면: 신규 커맨드 절차 전수·DENIED·파리티 /
  설계: FSD·컴포넌트 통일·과설계 / 계약: i18n·문서) → 적대적(major 이상) → 수정 → 커밋 →
  병합.
- 실기 확증(사용자): 웰컴에서 최근 프로젝트 클릭 전환·파일/폴더 열기·재시작 후 최근순 유지.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
