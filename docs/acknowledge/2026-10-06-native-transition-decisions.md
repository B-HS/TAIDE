# Rust-native 전환 인수 후 사용자 결정 (2026-10-06)

> 최종 갱신일: 2026-10-09 · 대응 커밋: `bf5175ca`

## 배경

2026-10-06 전수 감사(`docs/quality-assurance/2026-10-06-native-audit-summary.md`) 뒤 결정 사항을 제시했고, 사용자가 "1. 지금 커밋해놔. 나머지는 추천대로"라고 답했습니다.

## 결정

| 항목 | 결정 | 비고 |
| --- | --- | --- |
| 미커밋 M8 작업 | 지금 체크포인트 커밋 | `2026-09-30-m8-code-first-parity.md`의 "M8 완료 commit·push는 전체 완료 뒤에만"을 이 결정이 대체합니다. 이후 변경은 검증된 논리 단위로 커밋합니다. |
| 구문 강조 엔진 | TextMate 문법 호환 엔진(syntect 계열) | 기존 Shiki와 VSIX 문법이 TextMate 형식이라 재사용할 수 있습니다. 버전·라이선스·MSRV는 도입 시점에 공식 문서로 확인합니다. |
| 브라우저 Wasm 클라이언트(`native/taide-remote-web`) | 데스크톱 native 대응 완료 전까지 동결 | 공용 코드 변경으로 컴파일이 깨지지 않게만 유지하고 신규 기능 작업은 하지 않습니다. |
| 보조 창 | eframe 다중 viewport로 구현 | 질문 당시 추천안을 명시하지 않았습니다. TS 보조 창 동작을 그대로 재현한다는 원칙에 따라 이 값을 추천안으로 적용하며, 사용자가 정정하면 따릅니다. |
| native 메뉴바 의존성 | 추가(muda 계열) | 위와 같이 추천안을 명시하지 않았던 항목입니다. TS 앱의 메뉴(File > Open Recent 등)를 재현하려면 필요하므로 추가를 추천안으로 적용하며, 도입 시점에 최신 버전·호환을 확인합니다. |

## 구문 강조 엔진 결정 (2026-10-06 저녁)

편집기 설계 문서(`docs/research/2026-10-06-native-editor-display-layer-design.md` 8절)의 D1~D3에 대해 사용자가 "1. 추천되는 게 있으면 추천대로 2. 상관없어 3. 4종까지 싹 다 포함"이라고 답했습니다. 위 표의 "구문 강조 엔진: syntect 계열" 결정은 이 결정으로 대체합니다.

| 항목 | 결정 | 비고 |
| --- | --- | --- |
| D1 엔진 | `ferriki-textmate =0.12.0` + `ferroni =1.8.1`을 적합성 게이트 통과 조건으로 채택 | syntect는 `.sublime-syntax`만 읽어 VS Code tmLanguage·플러그인·VSIX 문법을 그대로 쓸 수 없습니다. 게이트(오프라인 빌드, 31개 언어 표본의 TS 출력 일치, 성능, 줄 한도)에 실패하면 차선(`syntaxmate 0.2.1`, 그다음 직접 이식 + onig)으로 넘어가며 그때 다시 보고합니다. |
| D2 새 의존성 반입 | 승인 | 버전 고정. lockfile 변경은 `native/taide-native-app`과 신규 `native/taide-native-syntax`에 한정합니다. |
| D3 문법 자산 | `@shikijs/langs 4.4.3`의 30종과 임베드 모듈을 전부 포함, 회색 지대 4종(elixir·toml·yaml·erb)도 포함 | `THIRD_PARTY_LICENSES.md`를 갱신합니다. 임베드 모듈 중 기존 고지 표에 없는 것의 라이선스는 추출 단계에서 확인하고 문제가 있으면 다시 보고합니다. |

메인이 반입 전에 확인한 사실(2026-10-06): 두 크레이트를 스크래치 프로젝트에서 `cargo fetch`로 내려받아(빌드·실행 없음) 버전과 존재를 확인했습니다. 추가되는 전이 의존성은 `aho-corasick`, `bitflags`, `memchr`, `smallvec`, `serde`, `serde_json`과 그 하위입니다. `ferriki-textmate`는 빌드 스크립트가 없고(약 1.1만 줄), `ferroni`의 `build.rs`는 기본으로 꺼져 있는 `ffi` feature에서만 C 소스를 컴파일합니다(약 9.9만 줄, `unsafe` 82곳). 두 크레이트의 `src`에서 프로세스 실행·네트워크·파일 쓰기·환경변수 접근 코드는 검색되지 않았습니다. 라이선스는 각각 MIT OR Apache-2.0, BSD-2-Clause이고 선언 MSRV는 1.94입니다.

## 기각된 대안과 이유

| 대안 | 기각 이유 |
| --- | --- |
| syntect로 구문 강조 | `.sublime-syntax`만 파싱해 VS Code tmLanguage JSON·플러그인·VSIX 문법을 그대로 읽지 못하고, Sublime 패키지는 scope 체계가 달라 테마 색이 TS와 달라집니다. 2026-10-06 오전의 "syntect 계열" 결정은 조사 뒤 대체됐습니다 |
| syntect + syntect-tmlanguage 변환 | begin/while 규칙, `\G` 앵커, 선택자 기반 injection을 지원하지 않아 markdown·yaml 문법을 재현할 수 없습니다 |
| tree-sitter | 플러그인·VSIX의 TextMate 문법과 테마 tokenColors를 재사용할 수 없고 언어별 C 파서를 새로 골라야 합니다 |
| giallo·zalo | EUPL-1.2(copyleft)이고 확인된 API가 문서 단위라 줄 단위 증분 토큰화에 맞지 않습니다 |
| `taide-native-editor`에 정규식 의존성 추가 | 동결된 `taide-remote-web`의 lockfile과 의존 그래프가 바뀝니다. 정규식 평가는 `taide-native-syntax`(앱 전용)에서 하고 코어는 인터페이스만 봅니다 |
| `EditorAppearance`·`EditorPresentation`에 필드 추가 | 구조체 literal 소비처가 13곳(동결 크레이트 테스트 포함)이고 기존 UI 테스트가 literal을 고정합니다. 새 진입 함수(`show_presented`, `show_tokenized`, `show_request`)를 추가했습니다 |
| 구현 단계의 병렬 실행·git worktree 격리 | native 크레이트가 서로를 path로 참조해 한쪽의 편집 중 상태가 다른 쪽 빌드를 깨고, 여러 단계가 `application.rs`·`editor_surface.rs`를 함께 고칩니다. 단계는 직렬로, 리뷰와 조사만 병렬로 돌립니다 |
| 합성 키·마우스 입력으로 화면 조작 | 사용자가 쓰는 다른 창으로 입력이 갈 수 있습니다. 세션 데이터를 미리 만들어 실행하고 창만 캡처합니다 |
| TS의 테마 전환 색 섞임 재현 | TS는 테마 이름이 항상 `taide`라 두 번째 테마부터 vscode-textmate 레지스트리가 갱신되지 않습니다. 결함으로 판단해 native는 새 테마로 다시 토큰화합니다(사용자에게 보고, 반대 지시 없음) |
| 빌드 캐시 삭제에 `rm -rf` | 자동 모드가 거부합니다. `cargo clean --profile dev`는 `debug`만 지워 실기용 번들을 보존하고, 사용자가 허용 규칙을 추가했습니다 |

## 진행 방식에 관한 사용자 지시 (2026-10-06~07)

| 시각 | 지시 | 적용 |
| --- | --- | --- |
| 10-06 오전 | "서브에이전트는 사용하지 않고 workflow만", "workflow에서는 fable을 사용하지 않고 opus와 sonnet을 판단해 분배" | 감사·구현·설계 opus, 리뷰 sonnet |
| 10-06 오전 | "batch1 끝나면 일시정지 2시부터 다시 시작" → "limit이 리셋되는 12:30분부터 시작" | 14:00 예약 취소, 12:31 재개 |
| 10-06 저녁 | "일단 배치4까지만 하고 멈춰놔" | 배치 4 커밋 뒤 정지 |
| 10-07 새벽 | "재개하기 전에 빌드 캐시 이런 거 싹 다 제거하고 시작해" | `cargo clean` 뒤 배치 5·6 진행, 배치 7 시작 |
| 10-07 아침 | "배치 얼마 남았어" | 정보 요청으로 처리(중단하지 않음). 남은 배치 약 17개, 배치당 약 3시간으로 답함 |
| 10-07 아침 | "일단 이번 배치까지만 하고 멈춰" | 배치 7 뒤 정지 예정. **현재도 유효** |
| 10-07 아침 | "잠시 싹 다 일시정지 나중에 다시 할 거야" | 배치 7 workflow 중단 |
| 10-07 아침 | "일단 여기까지 commit push 해놔" | 진행분을 체크포인트로 커밋(`fbc28b84`) |

## 미결

전역 ⌘N 제거, 실행 경로가 없는 명령의 키바인딩 편집기 비활성 표시, 팔레트가 열린 동안의 배경 처리, status 라벨 56곳의 처리 방향은 추천안만 제시했고 답을 받지 않았습니다. 리거처·컬러 이모지·bidi는 egui 한계로 기록하고 진행합니다.

HTML·미디어 미리보기의 WebView 의존과 macOS 외 플랫폼 지원 범위, 기본 데이터 경로·keychain 서비스명 승계 시점은 이번에 묻지 않았습니다. 해당 배치(구조 정리·cutover) 착수 전에 다시 확인합니다.
