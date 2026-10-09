# 배치 19 — 구문 접기 공급·수동/Import 접기 명령

현재 상태: batch18 구현 `2870345d`와 근거 `a99fb507`의 일반 푸시·0/0을 확인했습니다. 현재 기능표는 완료 281/588(47.8%), 부분 93·미연결 113·미구현 101입니다. batch19 체크리스트 0/7이며 전체 출시 전환율/잔여 시간은 미산정입니다. 서브에이전트·workflow 없이 main이 직접 구현하고 앞 Cargo/fmt process 종료를 확인한 뒤 다음 명령을 실행합니다.

## 범위와 실제 기준

TS `src/shared/lib/lsp/adapters/folding-range.ts`는 준비된 foldingRangeProvider의 textDocument/foldingRange를 요청해 완전한 줄 범위와 comment/imports/region 종류를 공급합니다. `initialize-params.ts`는 lineFoldingOnly와 rangeLimit를 광고합니다. 설치된 Monaco의 syntaxRangeProvider는 여러 provider 결과를 시작 줄/우선순위로 정렬하고 중첩·교차·같은 시작을 정리하며 foldingModel/foldingRanges는 사용자 범위·현재 커서·편집 후 상태를 병합합니다. 원본 수동 범위 생성/제거와 toggleImportFold는 현재 native registry에서 세 명령 모두 미지원으로 분류됩니다.

현재 native는 들여쓰기·언어 marker의 FoldRegion/FoldingModel·tracked fold/선택·16종 명령과 gutter/접힌 본문·wrap/스크롤/고정 줄 폴백을 공급합니다. typed SDK에는 FoldingRangeRequest가 이미 등록됐으나 앱 요청/표면의 소비는 남아 있습니다. 이 경계를 실제 코드로 더 확인하고 구문 범위·종류·수동 범위를 native에 연결합니다. 원본에 없는 기능/디자인·원본 버그/내부 수치 강제 재현·engine/의존성·OS 합성 입력을 추가하지 않습니다. frozen remote-web source/manifest/lock·실제 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle은 유지합니다.

## 체크리스트

- [ ] a. 실제 TS/Monaco·공식 typed API·현재 native 접기/표시/명령 경계 대조
- [ ] b. 구문/종류·중첩/순서/한도·수동 범위·편집/다중 뷰 모델 검증
- [ ] c. 실제 typed LSP 준비/미지원/빈/오류·취소/버전/프로젝트/재시작 공급
- [ ] d. native-host gutter/본문/키/마우스·수동/Import 명령·표시/스크롤 소비
- [ ] e. 실제 앱·고정 줄 provider 우선/폴백·reveal/readonly/대형 통합
- [ ] f. 변경 크레이트 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 현재 검증 상태

설치된 공식 lsp-types 0.97.0의 FoldingRangeParams/Range/Kind/ClientCapabilities와 기존 SDK typed 요청 등록, TS adapter/initialize 및 Monaco의 syntax provider·수동 생성/제거·Import toggle 코드를 읽었습니다. 현재 native의 FoldRegion·들여쓰기/marker·surface cache와 세 명령의 미지원 분류도 확인했습니다. a의 전체 공급/명령/view state·원본 provider 순서와 수명 대조는 아직 진행 중이며 새 구현/검사를 통과했다고 판정하지 않습니다.

batch18 app 전체 67대상·659/UI inspection 19대상·359와 변경 없는 batch16 editor 201, 이전 syntax/SDK는 해당 경계가 바뀌지 않을 때만 근거를 재사용합니다. 이번 변경 크레이트의 전체 대상은 f에서 별도로 실행합니다. 보호 Trash 3·기존 ignored 5·실제 OS/IME/접근성/pixel/대형/soak/출시 부채와 잘못된 7자리 theme HEX 결정은 미완료로 유지합니다.
