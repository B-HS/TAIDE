# e2e 실행 경로 리서치 (2026-08-14)

> 워크플로 wf_736692ca-b95(opus+medium 3축 병렬). 핵심 주장은 메인이 실물 재검증 완료 —
> 코드 인용 전건(포트 0 바인딩·자동 기동·password_only 로그인 경로·원격 차단 3종·WebglAddon)
> grep 대조, 웹 인용 중 결정적 2건(Tauri 공식 문서 macOS 지원 서술·크레이트 메타)은 직접 fetch 대조.
> 캠페인 계약: `docs/acknowledge/2026-08-14-remaining-features-pro-qa-plan.md` (결정 4 — 리서치만 선행,
> 채택은 전문 QA 설계 시점에 파일럿과 함께 확정).

## 1. 결론 요약

macOS 에서 TAIDE e2e 는 **두 경로가 성립**하고, 상호 보완 관계다.

| 경로 | 방식 | 강점 | 한계 |
|------|------|------|------|
| **A. remote-control 미러 경유** (자체 기능 재사용) | 앱이 띄운 웹 미러(148 커맨드·22 이벤트 공유)에 브라우저 자동화로 접속 | 앱 코드 변경 0, 이미 구현된 자산, 세션 재사용 가능 | qa6 114항목 기준 실행 60~65%·유효 판정 50~60% 추정. WKWebView 고유 표면(IME·OS 단축키·tauri CSP·픽셀)은 증거력 없음 |
| **B. embedded WebDriver** (`tauri-plugin-wdio-webdriver` + `@wdio/tauri-service`) | 앱 안에 WebDriver 서버를 심어 실제 WKWebView 를 조작 | Tauri·WebdriverIO 양쪽 공식 문서의 macOS 지원 경로. 실앱 프로세스 검증 | debug 빌드 전용(cfg(debug_assertions)), Cargo·npm 신규 의존, single-instance 충돌 리스크 |

- 외부 `tauri-driver` 단독은 **macOS 불가**가 여전히 공식 사실 — "Driven directly, only Windows and
  Linux are supported on desktop, as macOS has no WKWebView driver tool available"
  (https://v2.tauri.app/develop/tests/webdriver/ — 메인 fetch 재확인). CrabNebula 포크는 macOS 유료 키 필요라 배제.
- 러너는 **Playwright(라이브러리, webkit 1종만 설치·약 180MB) + node 실행**이 최적합.
  Playwright 의 bun 런타임 지원은 "closed as not planned"(microsoft/playwright#27139·#38095)이고
  bun 에서 launch/connect 행 보고(oven-sh/bun#8222)가 있어 **bun test 통합은 전제하지 않는다**
  (bun 은 패키지 매니저 역할만, 기존 46파일 로직 테스트는 bun test 유지).
- WebKit 엔진 선택 근거 2중: WKWebView 근사 + **xterm WebGL 렌더가 Chromium/Firefox 에서 깨진다는
  보고**(xtermjs#5154 — TAIDE 터미널은 WebglAddon 실사용, terminal-view.tsx:6·141 확인) 회피.

## 2. 경로 A 세부 (코드 재검증 완료)

- 서버는 `127.0.0.1` 포트 0(랜덤) 바인딩, 기동 포트는 `log::info!("원격 접속 서버 기동: port={port}")`
  로 파일 로그에 남음 — 하네스는 `~/Library/Logs/dev.taide.app/TAIDE.log` 에서 포트를 읽는다
  (`remote/commands.rs` bind_and_start).
- `remote_access_enabled=true` 면 부팅 시 자동 기동(`lib.rs` setup). 링크 토큰 발급(`remote_issue_link`)은
  IPC 전용·1회용·슬롯 1개라 자동화에 부적합.
- **자동 인증 경로**: 비밀번호 설정 + `remote_password_only_login=true` 상태에서는 미인증 접속이
  로그인 폼으로 리다이렉트되고, 폼 POST 가 nonce 없이 `issue_session_without_nonce` 로 세션 발급
  (`server.rs` login_post_route — 메인 재확인). 즉 **테스트용 비밀번호 1회 설정만으로 순수 HTTP 로그인
  자동화 성립**. 로그인 실패 잠금(5회 초과·지수 백오프 최대 60초)만 주의.
- 원격 불가 표면: 네이티브 다이얼로그(shim 이 `plugin:dialog` null 반환 — VSIX 임포트·폴더 열기·
  다른 이름으로 저장)·⌘Q/네이티브 메뉴·keyring·창 제어(show/setTheme)·원격 차단 커맨드 3종
  (`remote_set_password`/`remote_clear_password`/`file_flush_complete` — dispatch.rs 명시 거부 재확인)·
  종료 플러시 핸드셰이크(HotExitFlushRequested 팬아웃 제외). 브라우저 CSP 는 `BROWSER_CSP` 로
  데스크톱 tauri.conf CSP 와 다름 — W7 CSP 항목 대체 판정 불가.

## 3. 경로 B 세부 (문서 재검증 완료)

- `tauri-plugin-wdio-webdriver` 1.3.0(2026-08-03 갱신, 누적 42,403 DL, repo webdriverio/desktop-mobile —
  메인이 crates.io API 재확인). 설정: Cargo debug-only 의존 1줄 + `lib.rs` 빌더 체인에
  `#[cfg(debug_assertions)]` plugin 1줄 + capabilities `wdio-webdriver:default` + wdio.conf
  `driverProvider: 'embedded'`(포트 4445).
- 리스크: TAIDE 의 `tauri-plugin-single-instance` 와 스펙별 앱 재기동 충돌 소지, 네이티브 다이얼로그
  조작 가능 여부 문서 미기재(미확인), wdio 러너의 bun 환경 동작 미확인. 네이티브 표면은 접근성 API
  기반 `xa11y`(AXUIElement) 보조축 후보.

## 4. 채택 방침 (제안 — 전문 QA 설계 시 확정)

1. **A 를 1차 축**으로: 앱 무변경·기존 기능 재사용이라 비용 최소. 준비 = 사용자 1회(REMOTE 비밀번호
   설정 + password_only 토글 ON + 활성화 토글 ON) 후 하네스는 로그에서 포트 읽기 → POST 로그인 →
   Playwright storageState 재사용.
2. **B 는 파일럿 1건**으로 검증 후 필요 범위만: WKWebView 실증이 필요한 항목(실키 입력·앱 커맨드)이
   A 로 부족할 때. single-instance 충돌·bun 환경 3항목을 파일럿에서 실측.
3. 신규 의존 승인 요청(그 시점에): `playwright`(라이브러리) + webkit 바이너리. B 채택 시
   `tauri-plugin-wdio-webdriver`(debug-only) + `@wdio/tauri-service` 추가.
4. 검증 기법: 픽셀 비교 최소화 — monaco 는 `getEditors()[0].getValue()`, xterm 은 인스턴스 JSHandle
   buffer 읽기, 구조 회귀는 aria snapshot, WebGL 캔버스만 스크린샷.
5. e2e 로 대체 불가한 실기 잔여(IME·OS 단축키·⌘Q·다이얼로그·keyring·CLI 설치·번들 빌드·tunnel)는
   전문 QA 체크리스트에서 "사람 실기" 열로 분리 유지.

## 5. 미확인 (파일럿에서 실측할 것)

- bun 1.3.14 에서 Playwright webkit launch/connect 실동작 (실패 시 node 실행으로 확정)
- xterm WebGL 렌더 이슈(xtermjs#5154)의 2026 현재 재현 여부
- wdio embedded provider 의 다이얼로그·다중 윈도우 커버리지, single-instance 충돌
- 경로 A 의 로그인 왕복 실동작 (코드 논증만 완료 — 앱 기동 후 curl 1회면 확정)
