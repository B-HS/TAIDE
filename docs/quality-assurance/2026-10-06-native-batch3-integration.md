# 전환 배치 3 통합 검증 (2026-10-06)

## 범위

배치 3은 toast 일반 API와 status 문자열 이전, 공용 modal·아이콘 레지스트리 구현, 편집기 표시 계층·구문 강조 설계입니다. 단계별 상세는 `2026-10-06-native-batch3-{toast-feedback,modal-icons}.md`와 `docs/research/2026-10-06-native-editor-display-layer-design.md`입니다. 두 구현 단계 모두 리뷰에서 차단 항목이 2건씩 나와 수정 단계를 거쳤습니다.

## 리뷰 차단 항목과 수정

| 단계 | 차단 항목 | 수정 |
| --- | --- | --- |
| toast | 터미널 복사 실패가 탐색기와 같은 경로로 오류 toast를 띄움(TS는 무시) | 터미널 복사를 별도 host 명령으로 분리해 실패는 로그만 |
| toast | 읽기 전용 문서 저장이 내부 debug 문구(`native editor: ReadOnly`)로 표시 | `editor.readOnlySaveBlocked` 로컬라이즈 문구로 통일 |
| modal | 팔레트 닫힘 전환 200ms 동안 modal 층이 유지돼 뒤 화면이 요청한 포커스가 해제됨 | 닫힘 전환 중에는 modal 층으로 등록하지 않음(Radix의 `trapFocus: open`과 동일) |
| modal | 탭 아이콘 미구현 | 탭 종류별 아이콘 추가(lucide 4종 추가) |

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (최종 상태) | 347 통과, 0 실패 |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test workbench` | lib 116 통과, workbench 12 통과 |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test snippet-editor --features inspection` | 18 통과 |
| `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| `cargo fmt -- --check` (app, ui) | exit 0 |
| `snippet_button_hover는_…` 테스트를 배치 3 변경을 치운 커밋 기준(`09ced3a`)에서 실행 | 실패. 구현자가 고친 스니펫 버튼 불투명도 모션 결함이 기존 결함임을 확인 |

## 화면 확인

화면 잠금이 풀린 상태에서 격리 데이터 디렉터리로 앱을 실행해 창만 캡처했습니다: `assets/2026-10-06-native-welcome-after-batch3.png`.

- [x] 창 제목 `TAIDE`, 1400x900, traffic light가 자체 타이틀바 위에 겹쳐 표시되고 OS 타이틀바와 중복되지 않음
- [x] 테마(다크) 배경과 버튼 외형이 적용됨
- [x] 상태바 항목(문제 수, IDE 상태, CPU·RAM, 글꼴 크기 조절) 표시
- [ ] welcome 화면이 창 상단에 붙어 있고 최근 프로젝트·단축키 카드가 없습니다(감사에서 partial로 판정한 항목 그대로). 프로젝트가 없을 때 왼쪽 rail도 보이지 않습니다.
- [ ] 팔레트, toast, 탐색기 아이콘, 탭 아이콘, 편집기는 프로젝트를 열어야 볼 수 있어 캡처하지 못했습니다. 합성 입력은 사용자의 다른 창으로 갈 수 있어 쓰지 않았습니다.

## 사용자 결정이 필요한 사항

편집기 설계 문서 8절의 결정 6건입니다. D1~D3은 구문 강조 구현(설계 단계 3)의 선행 조건이고, 표시 계층 골격·글꼴·word wrap(단계 1·2·4)은 결정과 무관하게 진행합니다.

1. **D1 구문 강조 엔진**: syntect는 `.sublime-syntax`만 읽어 VS Code tmLanguage 문법과 플러그인·VSIX 문법을 그대로 쓸 수 없습니다. 추천은 vscode-textmate 이식 엔진 `ferriki-textmate 0.12.0` + `ferroni 1.8.1`을 적합성 게이트(오프라인 빌드, 31개 언어 표본의 TS 출력 일치, 성능, 줄 한도) 통과 조건으로 채택하는 것입니다. 이 크레이트는 최초 공개가 2026-09-28인 beta라 성숙도 위험이 큽니다. 대안은 `syntaxmate 0.2.1`, vscode-textmate 직접 이식 + onig(C 의존), syntect 유지(색과 지원 언어가 TS와 달라지고 플러그인 문법 기능을 잃음)입니다.
2. **D2 새 의존성 반입**: D1의 두 크레이트를 버전 고정으로 내려받고 native 앱 lockfile을 갱신하는 것에 대한 승인.
3. **D3 문법 자산**: `@shikijs/langs 4.4.3`의 30종과 임베드 모듈 JSON을 native 바이너리에 포함하고 라이선스 고지를 갱신. 2026-08-12에 승인된 회색 지대 4종(elixir·toml·yaml·erb)을 native에도 적용할지 여부.
4. **D4 리거처**: epaint 0.36.2는 리거처를 끌 수 없습니다. 한계로 기록하고 진행하는 것을 추천합니다.
5. **D5 컬러 이모지·bidi**: egui 한계로 기록하고 진행하는 것을 추천합니다.
6. **D6 찾기 위젯의 정규식 방언**: Monaco는 JS 정규식입니다. 찾기 위젯 착수 시 후보를 비교해 다시 보고합니다.

이전 배치에서 넘어온 결정 3건(전역 ⌘N, 실행 경로 없는 명령의 노출, 팔레트 배경)과 이번 배치의 status 라벨 56곳 처리 방향도 남아 있습니다. status 56곳은 TS에 대응 표시가 없는 native 내부 오류로, 테마·로케일 실패는 TS의 `StatusErrorBanner`를, 트리 실패는 TS의 인라인 `rootUnavailable` 상태를 구현하면 각각 옮길 수 있습니다.

## 설계 문서의 추가 발견

- `install_syntax`의 비테스트 호출부가 없어 plaintext 외 파일에는 "저장 시 후행 공백 제거"가 적용되지 않고 있습니다. 구문 강조가 토큰 종류를 공급하는 시점부터 실제로 동작합니다.
- 감사 문서의 "리거처 경로 없음"은 사실과 반대입니다. epaint가 기본 feature로 셰이핑해 리거처는 항상 켜져 있습니다.
- egui 0.36.2 텍스트 계층의 한계: 탭이 탭 정지가 아닌 고정 폭, 굵기 플래그 없음, 물결 밑줄 없음, 컬러 이모지 경로 없음, bidi 미지원, 기본 monospace 체인에 CJK 글꼴 없음.
