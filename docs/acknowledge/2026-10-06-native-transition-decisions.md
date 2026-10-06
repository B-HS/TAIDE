# Rust-native 전환 인수 후 사용자 결정 (2026-10-06)

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

## 미결

전역 ⌘N 제거, 실행 경로가 없는 명령의 키바인딩 편집기 비활성 표시, 팔레트가 열린 동안의 배경 처리, status 라벨 56곳의 처리 방향은 추천안만 제시했고 답을 받지 않았습니다. 리거처·컬러 이모지·bidi는 egui 한계로 기록하고 진행합니다.

HTML·미디어 미리보기의 WebView 의존과 macOS 외 플랫폼 지원 범위, 기본 데이터 경로·keychain 서비스명 승계 시점은 이번에 묻지 않았습니다. 해당 배치(구조 정리·cutover) 착수 전에 다시 확인합니다.
