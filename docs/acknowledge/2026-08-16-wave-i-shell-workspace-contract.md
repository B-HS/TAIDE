# Wave I 구현 계약 — 셸·워크스페이스 (2026-08-16, 캠페인 최종)

> 정찰 wf_c2974bc4-295(opus+high 4축: 레이아웃·윈도우 현행 / 설정 탭 편집 / 플러그인·VSIX /
> Tauri 2 멀티윈도우). 하중 주장 전건 메인 실물 재검증 완료 — **멀티 윈도우 차단급 결함 4종 확정**
> (부창 닫기=앱 전체 종료 경로·lsp_spawn 재사용이 두 번째 창 Channel 폐기·pty_attach 단일 구독자
> 덮어쓰기·capability 미매칭 창의 이벤트 반생존). 추가 확정: load_layout 은 스키마 버전 불일치 시
> **마이그레이션 없이 default_layout 폴백(기존 탭 전량 소실)**·load_settings 부팅 1회+설정 변경
> 이벤트 부재(sync_download 에 동일 기존 결함)·플러그인 신규 언어 grammar 가 화면 미도달·
> vsix_extract_themes 원격 허용+루트 가드 없음·extract_zip zip bomb 무방비.
> 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md`(완벽 우선·4렌즈·역할 상향).

## 1. 사용자 결정 (2026-08-16)

| # | 결정 | 선택 |
|---|------|------|
| ① Wave H prod | **병합** | fast-forward 완료 |
| ② 멀티 윈도우 | **완전 구현** | 사용자 원문: "MVP 가 아니라 할거면 제대로 완벽하게 잘 구현·검증해" — 창=탭 분리(B안, tabs.md §4.4·§3.1 확정 스펙 정합·Move Tab into New Window)까지 포함. 차단급 결함 4종은 설계 회피(중복 개창 금지)가 아니라 **근본 수정**: LSP 채널 다중화·pty 다중 구독자·창별 close/hot-exit·capability |
| ③ Zen | 추천 패키지 | 표시 상태를 레이아웃(창 단위)에 영속 + **스키마 버전 상승과 마이그레이션 arm 신설**(기존 폴백 결함 상환) + Settings zen 정책 + ⌘K Z chord + ESC 복귀 + 전체화면 Rust 커맨드 + 사이드바 접힘 Rust 승격(폭 debounce 미러) |
| ④ 설정탭·플러그인 | 추천 패키지 | TabKind::AppFile{target 열거}+전용 커맨드(root_guard 무접촉 별도 가드)+settings_reload+SettingsChanged 이벤트(sync 기존 결함 동반 해소)+1차 미러 제외 / 플러그인 로컬 임포트 설치 UI+VSIX grammar→플러그인 생성+monaco 동적 언어 등록+Rust 확장자 오버레이(4벌 중복 해소)+신규 커맨드 원격 거부+vsix_extract_themes 원격 거부 전환+zip 하드닝 해제 함수 |

## 2. 확정 사실 (메인 재검증 완료 — 근거 실물)

1. **부창 닫기 = 앱 전체 종료**: handle_close_requested(lib.rs:313-334)가 창 라벨 무관하게
   prevent_close→hot-exit flush→전역 종료 경로. begin_hot_exit_flush 는 전역 단일 상태.
2. **lsp_spawn 재사용 경로가 두 번째 호출의 on_message Channel 을 저장하지 않고 폐기**
   (lsp/commands.rs:340-366 — should_reuse_session 시 existing_id 반환만). 같은 프로젝트 2창이면
   두 번째 창의 LSP 가 무증상 사망.
3. **pty_attach 는 단일 구독자 덮어쓰기**(`*entry.subscriber.lock() = Some(on_data)` —
   terminal/commands.rs:210대). 두 번째 attach 가 첫 구독자를 탈취(remote↔데스크톱 간에도 성립하는
   기존 결함).
4. **capability**: tauri.conf.json windows=['main'], capabilities/main.json windows:['main'] —
   앱 커스텀 커맨드는 전 창 허용이나 core/plugin 권한(이벤트 listen 포함)은 미매칭 창에서 거부 —
   "조회는 되는데 갱신 이벤트만 안 오는" 반생존. capability windows 는 라벨 글로브 지원(공식 문서 —
   `editor-*` 실동작은 구현 중 검증 게이트).
5. **load_layout 은 version 불일치 시 default_layout() 폴백**(layout/service.rs:663-671 —
   마이그레이션 arm 없음). 스키마 상승이 필요한 이번 웨이브의 필수 전제 결함.
6. **표시 상태 필드 전무** — ProjectLayout·Settings 어디에도 zen/사이드바/패널 필드 없음. 사이드바
   접힘·Problems 패널은 프론트 로컬 상태(비영속). 레이아웃은 프로젝트 단위 키(창 축 없음),
   active_project 는 전역 세션.
7. **load_settings 는 부팅 1회**(lib.rs:400)·설정 변경 이벤트 없음 — sync_download 가 이미 "저장은
   되는데 반영 안 됨" 결함 보유(재적용·무효화 없음).
8. **플러그인**: contributes.languages 의 신규 언어는 monaco 등록 루프(setup.ts 고정 32종)와 shiki
   monacoLanguageIds 게이트에 막혀 **화면에 도달하지 않음**. contributes.lsp/themes 소비자 0.
   확장자→언어 테이블 4벌 중복(Rust 3+TS 1). vsix_extract_themes 원격 허용·루트 가드 없음(제한적
   임의 파일 읽기 표면). infra/lsp_install::extract_zip 은 zip-slip 만 방어(크기·엔트리 상한 없음·
   unix_mode 그대로 — 신뢰 소스 전제라 VSIX 재사용 부적합).
9. window-state 플러그인은 label 키 구조(멀티 창 자연 확장·동적 라벨 누적은 map_label 로 정규화
   필요). 'main' 하드코딩은 Rust 2곳(메뉴 Quit·single-instance). 이벤트는 전역 emit + 프론트 전역
   구독 — 상태 동기화는 멀티 창에 유리. 창 제어 프론트 3곳은 전부 getCurrentWindow() 라 창별 자연
   동작.

## 3. 확정 설계

### 3.1 멀티 윈도우 기반 (Rust — 차단급 결함 근본 수정)

- **창 모델**: main + 보조 에디터 창(라벨 `editor-<n>`, **Rust 발급·소유** — `window_open_auxiliary`
  커맨드가 WebviewWindowBuilder 로 생성(core 창생성 권한 불필요), URL 쿼리로 projectId·windowSlot
  전달. 창 목록·정리는 Rust 소유. capability: main.json windows 를 `['main', 'editor-*']` 글로브로
  확장(**글로브 실동작 검증 게이트** — 실패 시 보조 창 전용 capability 파일로 대체하고 보고).
  window-state 는 map_label 로 보조 창을 단일 키로 정규화(누적 방지)하거나 보조 창 제외 —
  구현 판단·문서화.
- **LSP 채널 다중화(근본 수정)**: 세션의 메시지 출력을 구독자 맵(구독 id→Channel)으로 전환 —
  lsp_spawn 재사용 시 구독 추가, 메시지는 전 구독자 브로드캐스트(응답 id 매칭은 각 창 클라이언트가
  자기 pending 만 소화 — 기존 동작), send 실패 구독자 자동 제거·창 닫힘 시 해제. 기존 단일 창
  동작 무변화 검증.
- **pty 다중 구독자(근본 수정·기존 결함 상환)**: subscriber 를 목록으로 전환 — attach 마다 구독
  추가+ring buffer replay, 출력 브로드캐스트, send 실패 구독자 제거. remote↔데스크톱 동시 시청도
  이걸로 해소. detach/창 닫힘 정리.
- **창별 close/hot-exit**: handle_close_requested 라벨 분기 — 보조 창: 그 창의 탭을 **main 창
  레이아웃 말미로 복귀**(TAIDE 0-손실 철학 — VS Code 는 닫아버리지만 우리는 보존) 후 그 창만 닫음.
  main 창: 현행 전역 hot-exit flush 종료(이때 보조 창 탭 포함 전 창 미러 플러시 — flush 요청
  이벤트를 전 창에 fanout·수신 집계). 메뉴 Quit(Cmd+Q)도 동일 전역 경로. single-instance 는 main
  show+focus 유지.
- **active_project 와 창**: main 창은 현행 전역 활성 프로젝트 유지. 보조 창은 자기
  (projectId, windowSlot) 고정 렌더 — ProjectActivated 전역 이벤트를 무시(자기 뷰 고정). 프론트
  부트스트랩이 URL 쿼리로 분기.

### 3.2 레이아웃 window 축 + Zen (Rust)

- **스키마**: ProjectLayout 에 보조 창 축 추가 — `auxiliary_windows: Vec<AuxWindowLayout{ slot,
  root: PaneNode, focused_pane }>` + 창 단위 표시 상태 `shell_view: ShellViewState{ zen: bool,
  sidebar_collapsed: bool }`(main 창 대상 — 보조 창은 사이드바·상태바 없는 에디터 전용 크롬).
  **LAYOUT_SCHEMA_VERSION 상승 + 마이그레이션 arm 신설**(구버전 JSON → 신 스키마 변환, 기존 탭
  보존 — 현행 default_layout 폴백 결함 상환. 마이그레이션 불가 파손만 폴백). 보조 창 레이아웃
  영속·재시작 시 복원(창 재생성).
- **탭 창간 이동**: `layout_move_tab_to_window(projectId, tabId, target: MainWindow | NewAuxiliary |
  Existing(slot))` — 신규 보조 창 생성 포함, 역방향(main 복귀) 동일 커맨드. 탭 컨텍스트 메뉴
  "Move into New Window"·"Move back to Main Window" + 팔레트. dirty·미러·pinned 상태는 탭에 실려
  그대로 이동(모델은 Rust 소유라 내용 무손실).
- **Zen**: `layout_set_shell_view` 커맨드 — zen 토글 시 사이드바·상태바·탭바 숨김(프론트 렌더),
  ESC 복귀·⌘K Z chord(Wave H 엔진 재사용, KeymapActionId 신설)·팔레트. Settings 정책 2필드:
  `zen_fullscreen`(기본 false — true 면 진입 시 전체화면, Rust 커맨드로 setFullscreen)·
  `zen_hide_status_bar`(기본 true). 사이드바 접힘은 shell_view 로 영속(폭은 프론트 debounce 미러 —
  ADR-0004 예외 문서화).

### 3.3 설정 파일 탭 편집

- **TabKind::AppFile{ target: AppFileTarget }** — 열거형: `Settings` | `Prompt{ id: String }`(3종
  프롬프트 id 화이트리스트). 경로는 Rust 가 AppPaths 로 유도(프론트·레이아웃에 절대 경로 비노출).
- 전용 커맨드 `app_file_read(target)`·`app_file_write(target, content)` — root_guard 무접촉(별도
  화이트리스트 가드), settings 는 write 시 **parse→sanitize→적용→SettingsChanged 이벤트**(신설,
  전 창+원격 fanout — 프론트는 SETTINGS.CURRENT 무효화)를 한 트랜잭션으로. 유효하지 않은 JSON 은
  저장 거부(기존값 보존)+에러 표시. **sync_download 도 동일 재적용 경로를 타도록 정리(기존 결함
  동반 해소)**. prompts 는 순수 쓰기+쿼리 무효화.
- 미러(hot-exit) 1차 제외 — dirty 표시만, 닫기 시 확인 다이얼로그 기존 dirty 흐름 재사용. 동일
  target 탭의 pane 단위 중복 가능성은 기존 파일 탭과 동일 정책(문서화). 원격 dispatch: app_file_*
  는 settings_update 와 동급으로 허용하되 SettingsChanged fanout 정합 확인.
- 진입: 설정 화면 "Open settings.json"·팔레트·프롬프트 편집 진입.

### 3.4 플러그인 설치 UI·VSIX grammar

- **설치**: `plugin_install(sourcePath)` — .vsix(zip) 또는 디렉토리 → 검증 → `plugins/{id}/` 기록 →
  reload. **하드닝 해제 함수 신설**(vsix 도메인 — 누적 크기 예산·엔트리 수 상한·경로 zip-slip·
  unix_mode 마스킹 0o644/0o755 고정 — lsp_install::extract_zip 재사용 금지). `plugin_uninstall(id)`
  (빌트인 보호 없음 — 사용자 디렉토리만). 설정 PLUGINS 섹션: 목록(활성·오류 표면화)+설치(다이얼로그)+
  제거+폴더 열기. 비활성 토글 미도입(E1 — 삭제로 갈음).
- **VSIX grammar 임포트**: vsix 에서 grammars+languages 추출 → `taide-plugin.json` 합성 →
  plugins/{publisher}-{name}/ 에 플러그인으로 착지(B안 — 기존 검증·reload 재사용. vsix 도메인 최초
  쓰기 — 계약으로 명시). 테마 전용이던 기존 vsix 흐름과 UI 통합(임포트 다이얼로그에서 테마/문법 선택).
- **언어 활성화(C1 — 없으면 껍데기)**: monaco 동적 언어 등록(플러그인 languages 를 부트스트랩·reload
  시 monaco.languages.register + shiki 커스텀 grammar 로드·reinitShiki 전체 재생성(H1)) + **Rust
  확장자 오버레이 단일 소유(D1)**: 플러그인 languages 를 읽어 `language_id_for_path` 공용 함수에
  오버레이, file/ide/git 3곳이 공용 호출로 통합(4벌 중복 해소 — TS 사본은 Rust 반환 소비로 전환
  가능한 범위만, 최소 변경).
- **원격 정책(F1)**: plugin_install·plugin_uninstall 은 dispatch **명시 거부**(deny 패턴),
  **vsix_extract_themes 도 원격 거부로 전환**(기존 노출 상환 — 계약 명시 변경). plugin_list/reload
  는 현행 유지.

### 3.5 실행 구조 (완전 구현 — 캠페인 최대 규모)

- **Phase S1(Rust 인프라 단독, sonnet+xhigh)**: §3.1 전체 — LSP 채널 다중화·pty 다중 구독자·창
  생성 커맨드·close 라벨 분기·전 창 flush fanout·capability 글로브(+검증 게이트)·window-state·
  Cmd+Q. 기존 단일 창 회귀 0 을 테스트로 고정. cargo 그린 종료.
- **Phase S2(Rust 도메인 단독, sonnet+xhigh)**: §3.2 스키마+마이그레이션+탭 이동 커맨드+shell_view
  +Zen 커맨드 / §3.3 AppFile·app_file_*·SettingsChanged·sync 재적용 / §3.4 plugin_install·uninstall·
  하드닝 해제·VSIX grammar 착지·확장자 오버레이 / Settings 신필드(zen 2)+locale 전량(4곳)+배선 3곳·
  파리티·bindings.
- **Phase F 프론트 병렬 3(sonnet+xhigh, 파일 소유 분리)**: F1=창 부트스트랩(URL 분기·보조 창
  크롬·Move Tab UI·탭 컨텍스트 메뉴·창 이벤트) / F2=Zen UI(표시 상태 렌더·⌘K Z·ESC·전체화면·
  사이드바 승격 배선) / F3=AppFile 탭 pane·설정 진입점·플러그인 설치 UI(PLUGINS 섹션)·monaco 동적
  언어 등록·SettingsChanged 구독.
- **Phase D 통합(단독)**: 접합부·문서(features/layout-shell·window-chrome stale 정정 포함·
  features/plugins·ipc-contract·data-model·qa6 Wave I 실기(멀티 윈도우 실기 다수)·갭 §3·§7 종결)·
  전체 verify+vite build.
- **Phase E 검토**: 4렌즈(opus+xhigh — 집중: 창 생명주기·채널 다중화 경합·마이그레이션 무손실·
  브로드캐스트 성능·zip 하드닝·원격 정책 전환·AppFile 가드) → 적대적 검증(opus+high) → 수정
  (sonnet+xhigh) → 메인 2차 → 커밋(dev).

## 4. 기각·보류

| 안 | 처리 |
|----|------|
| 멀티 윈도우 MVP(창=프로젝트·중복 개창 금지) | **기각 — 사용자 지시로 완전 구현 격상**("MVP 가 아니라 제대로") |
| 멀티 윈도우 전면 보류(D)·부동 패널 대체(E) | 기각 — 동일 |
| 보조 창 닫기 시 탭 폐기(VS Code 동형) | 기각 — main 복귀(0-손실 철학) |
| 레이아웃 마이그레이션 없이 버전 폴백 유지 | 기각 — 마이그레이션 arm 신설(기존 결함 상환) |
| 설정탭 가상 스킴(taide://)·Settings 탭 임베드(c) | 기각 — AppFile 열거 target 채택 |
| 설정탭 미러(hot-exit) 포함 | 보류 — 1차 dirty 표시만, 후속 재검토 |
| 플러그인 비활성 토글(disabledPlugins) | 기각(1차) — 삭제로 갈음(E1) |
| lsp_install::extract_zip 재사용 | 기각 — 신뢰 전제 상이, 하드닝 전용 함수 신설 |
| 신규 언어 활성화 제외(C2 — grammar 교체만) | 기각 — C1 동반(껍데기 방지) |
| 프론트 확장자 오버레이(D2) | 기각 — Rust 단일 소유(D1) |
| shiki 증분 로드(H2) | 기각(1차) — 전체 재생성(H1, 설치는 드문 이벤트) |
| 마켓플레이스/원격 레지스트리 | 기각 유지 — 로컬 임포트만(기존 방침) |

## 5. 완료 조건

- `bun run verify` 전체 + vite build. locale 4곳·en⊆required·파리티(신규 커맨드 다수 반영).
- 4렌즈+적대적 검증+메인 2차 통과. 초점: **차단급 4종의 근본 수정 실효**(부창 닫기 격리·재사용
  세션 다중 채널 브로드캐스트·pty 다중 구독자·capability 글로브 실동작)·**레이아웃 마이그레이션
  무손실**(구버전 JSON 실데이터 왕복 테스트)·창간 탭 이동의 dirty/미러/pinned 보존·main 종료 시
  전 창 flush 집계·SettingsChanged 재적용(수동 편집·sync 양 경로)·AppFile 화이트리스트 가드 우회
  불가·zip 하드닝(예산·상한·mode)·원격 정책 전환(vsix 거부) 회귀·monaco 동적 언어 등록의 shiki
  재생성 정합·기존 단일 창 사용자 경험 무변화.
- 문서: features/layout-shell(stale 정정 포함)·window-chrome(stale 정정)·plugins·tabs(§4.4 이행)·
  ipc-contract·data-model·qa6 Wave I 실기 항목·갭 분석 §3·§7 종결 표기. 캠페인 A~I 전 웨이브 종결.
