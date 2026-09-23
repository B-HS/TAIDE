# Swift 단독 전환 가능성 평가

> 확인일: 2026-09-23
> 대상: TAIDE 0.2.6의 TypeScript·React 렌더러와 Rust·Tauri 코어
> 범위: 코드 변경 없이 아키텍처, 기능 동등성, Swift 생태계, 전환 전략을 평가합니다.

## 1. 결론

TAIDE를 macOS 전용 Swift 앱으로 다시 만드는 것은 기술적으로 가능합니다. 다만 현재 코드를 Swift로 자동 변환하거나 점진적으로 문법만 옮기는 방식은 불가능합니다. 현행 구조는 React 표시층과 Rust 시스템 코어가 약 16.5만 줄에 걸쳐 Monaco, xterm, Git, PTY, LSP, 원격 서버, 미리보기, 상태 복원을 결합하므로 **전면 재작성**으로 봐야 합니다.

권장안은 현행 TS·Rust를 유지하는 것입니다. Swift 전환의 목적이 macOS 네이티브 창·메뉴·접근성이라면 Swift 호스트와 기존 WebView를 결합하는 프로토타입부터 검증할 수 있지만, 이 경우 JavaScript가 남아 Swift 단독은 아닙니다. Rust까지 제거하는 전환은 편집기·터미널·Git·다언어 LSP의 동등성 검증이 선행된 경우에만 합리적입니다.

## 2. 현재 규모와 책임 경계

| 구분 | 관찰값 | 핵심 책임 |
| --- | ---: | --- |
| TypeScript·TSX·CSS | 874파일, 93,645줄 | React UI, Monaco 모델과 편집 UX, xterm 렌더링, LSP 어댑터, 분할·DnD, 미리보기, Query 캐시·IPC 이벤트 동기화 |
| Rust | 163파일, 71,565줄 | 영속 상태, 파일·트리·레이아웃, Git, PTY, LSP 프로세스, 파일 감시, AI·동기화, 원격 HTTP·WebSocket 서버, 보안 경계 |
| IPC 계약 | `src/shared/api/bindings.ts` 3,358줄 | Rust 타입에서 생성된 command·event·raw channel 계약 |
| 테스트 파일 | TS·Rust 합계 294개 | 모델·캐시·레이아웃·Git·PTY·LSP·복원·보안 회귀 |

정본 구조는 `docs/architecture.md`의 `React view → typed IPC → Rust core`입니다. Rust는 Tauri 호출을 얇게 중계하는 계층이 아니라 25개 도메인의 상태와 시스템 자원을 소유합니다. React도 단순 화면 템플릿이 아니라 Monaco·xterm의 수명주기와 LSP 변환, 캐시 무효화, 다중 창 동기화를 담당합니다.

## 3. 기능별 Swift 대체 판정

| 영역 | Swift 후보 | 난이도 | 판정 |
| --- | --- | --- | --- |
| 앱 셸·설정·사이드바 | SwiftUI + AppKit | 중간 | 네이티브 재작성 가능. 로케일, 키맵, 접근성, 다중 창 상태를 다시 구현해야 합니다. |
| 코드 편집기 | `NSTextView` + TextKit 2 | 매우 높음 | Monaco의 다중 커서, undo/model 공유, diff, semantic decoration, 키맵, 완성 UI, 대형 파일 모드를 제공하지 않으므로 자체 편집기 제품 개발에 가깝습니다. |
| 터미널 렌더러 | SwiftTerm AppKit | 매우 높음 | 기본 ANSI 터미널 기반은 있으나 xterm의 add-on, OSC 133, IME, flow control, 링크, scrollback replay와 TAIDE 세션 계약을 새로 연결해야 합니다. |
| PTY | Darwin `forkpty`/`openpty` 또는 패키지 | 매우 높음 | Foundation `Process`는 PTY가 아닙니다. resize, process group, signal, login shell, batching, attach replay를 직접 구현해야 합니다. |
| Git | Git CLI 또는 libgit2 Swift wrapper | 매우 높음 | partial hunk·line stage, conflict, stash, graph, blame, SSH·HTTPS 인증까지 동등하게 유지해야 합니다. SwiftGit2는 libgit2 C binding이므로 엄격한 순수 Swift가 아닙니다. |
| 다언어 LSP | `Process` + Pipe + JSON-RPC client | 높음 | 프로세스 수명주기, Content-Length framing, restart·document replay, 20종 Monaco provider 변환을 다시 구현해야 합니다. SourceKit-LSP만으로 18개 언어를 대체할 수 없습니다. |
| 파일 감시·검색 | FSEvents + 자체 debounce/cache | 높음 | rename grouping, ignore, self-write, overflow full rescan, gitignore walker 동등성이 필요합니다. |
| 원격 제어 | Network.framework 또는 SwiftNIO | 매우 높음 | HTTP·WebSocket, range serving, Host·Origin 검증, 인증·세션 회수, backpressure, 기본 거부 dispatch를 다시 구현해야 합니다. |
| 미리보기 | PDFKit·Quick Look·WKWebView | 혼합 | 이미지·PDF·미디어는 유리하지만 XLSX grid, HWP/HWPX, PPTX outline은 기존 JS/WASM 파서 대체가 필요합니다. Monaco/xterm을 WKWebView에 두면 Swift 단독이 아닙니다. |
| 상태·보안 | Observation·actors, Keychain, CryptoKit | 중간~높음 | API 대체는 가능하지만 현재 mutation ordering, hot-exit handshake, 다중 창 event/cache 수렴, fail-closed 오류 의미를 보존해야 합니다. |

## 4. Swift 단독의 의미

가능한 목표는 두 가지로 나뉩니다.

1. **제품 소스에서 TS와 Rust 제거**: SwiftUI/AppKit 코드 중심으로 재작성할 수 있습니다. 다만 Apple 프레임워크, Darwin C API, 외부 LSP·Git 프로세스, 필요 시 C 라이브러리에 의존합니다.
2. **Swift 언어와 표준 라이브러리만 사용**: macOS GUI, PTY, 파일 감시, Git 요구와 양립하지 않습니다. AppKit·FSEvents·POSIX PTY 자체가 Objective-C/C 경계입니다.

Swift 호스트에 Monaco와 xterm을 WKWebView로 유지하는 안은 가장 낮은 위험으로 UI 셸을 바꿀 수 있지만 JavaScript 런타임을 유지하므로 첫 번째 목표에도 해당하지 않습니다.

## 5. 선택지 비교

| 선택지 | 장점 | 비용·위험 | 판정 |
| --- | --- | --- | --- |
| 현행 TS + Rust 유지 | 이미 구현·검증된 전체 기능, 성숙한 Monaco·xterm·libgit2 유지 | Swift 네이티브 UI를 얻지 못함 | 현재 권장 |
| Swift 셸 + 기존 WebView + Rust 코어 | macOS 창·메뉴 통합을 실험하면서 편집기·터미널·코어 보존 | TS·Rust 모두 남고 Tauri IPC를 새 transport로 연결해야 함 | 목적이 명확할 때만 프로토타입 |
| SwiftUI/AppKit UI + Rust 코어 | 웹 UI 제거, 고위험 시스템 코어와 테스트 유지 | Monaco·xterm·LSP UI를 전면 재작성하고 Swift↔Rust transport가 필요 | 장기 단계 전환 후보 |
| Swift 전면 재작성 | 단일 주 언어, macOS 최적화 가능 | 약 16.5만 줄 기능과 294개 테스트 계약 재구축, Git·PTY·편집기·원격 보안 퇴행 위험 | 비권장 |

## 6. 전환을 검토할 최소 실험

전면 전환 결정을 내리기 전에 다음 네 가지가 현행 기준을 통과해야 합니다.

1. `NSTextView`/TextKit 2로 대형 파일, incremental decoration, completion, split view state, undo/model 공유를 구현하고 Monaco와 UX·메모리를 비교합니다.
2. SwiftTerm 기반 번들 앱에서 login shell, resize, IME, hyperlink, long scrollback, selection, OSC 133, hardened runtime 동작을 확인합니다.
3. TypeScript, Rust, Swift LSP 서버를 연결해 initialize, didOpen/change, completion, diagnostics, crash restart와 document replay를 검증합니다.
4. Git CLI와 libgit2 wrapper로 partial stage·unstage, conflict, worktree, stash, SSH credential을 비교하고 기존 fixture를 통과시킵니다.

이 네 실험 중 편집기나 터미널이 기준에 미달하면 Swift 전면 전환보다 현행 구조 유지가 합리적입니다.

## 7. 공식·일차 자료

- Apple: [SwiftUI TextEditor](https://developer.apple.com/documentation/swiftui/texteditor), [TextKit](https://developer.apple.com/documentation/appkit/textkit), [Foundation Process](https://developer.apple.com/documentation/foundation/process)
- Apple: [FSEvents](https://developer.apple.com/documentation/coreservices/file_system_events), [NWListener](https://developer.apple.com/documentation/network/nwlistener), [Keychain Services](https://developer.apple.com/documentation/security/keychain-services)
- Apple: [PDFView](https://developer.apple.com/documentation/pdfkit/pdfview), [QLPreviewView](https://developer.apple.com/documentation/quicklookui/qlpreviewview), [WKWebView](https://developer.apple.com/documentation/webkit/wkwebview)
- 프로젝트: [SwiftTerm](https://github.com/migueldeicaza/SwiftTerm), [SourceKit-LSP](https://github.com/swiftlang/sourcekit-lsp), [SwiftNIO](https://github.com/apple/swift-nio)
- 프로젝트: [SwiftGit2](https://github.com/SwiftGit2/SwiftGit2), [libgit2](https://github.com/libgit2/libgit2), [Swift 6 동시성 전환 가이드](https://www.swift.org/migration/documentation/swift-6-concurrency-migration-guide/dataracesafety/)

## 8. 검증 범위와 한계

- 저장소는 `rg`, `find`, `wc`, `sed`로 읽기 전용 조사했습니다.
- 세 하위 조사가 프론트엔드, Rust 코어, Swift 생태계를 독립적으로 확인했고 결론이 일치했습니다.
- 제품 코드를 변경하지 않아 빌드·테스트는 실행하지 않았습니다. 문서 내부 경로·링크와 diff만 검증합니다.
- 실제 개발 기간은 팀 규모, 동등성 기준, 외부 패키지 허용 범위가 정해지지 않아 산정하지 않았습니다.
