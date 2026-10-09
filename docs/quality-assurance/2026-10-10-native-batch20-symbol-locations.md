# 배치 20 — 정의·선언·타입/구현 이동과 참조·peek

현재 상태: batch19 구현 `8832ac61`과 완료 근거 `bb4eb96e`를 일반 푸시해 로컬/원격 차이 0/0을 확인했습니다. 기능 대응표는 완료 282/588(48.0%), 부분 92·미연결 113·미구현 101입니다. batch20 체크리스트 0/7이며 전체 출시 전환율/잔여 시간은 미산정입니다. 서브에이전트·workflow 없이 main이 직접 수행하고 Cargo/fmt는 실제 앞 process 종료를 확인한 뒤 하나씩 실행합니다.

## 범위와 확인한 기준

TS `src/shared/lib/lsp/adapters/definition.ts`의 공용 location adapter와 declaration/type-definition/implementation, references adapter는 위치 요청·취소·Location/LocationLink 변환과 peek target preload를 연결합니다. 참조 요청은 includeDeclaration을 보존합니다. `src/shared/lib/lsp/peek-model-preload.ts`는 기존 모델을 우선하고 새 normal 모델의 파일 수/TTL을 제한하며 실제 편집 탭이 인수한 모델을 자동 폐기하지 않습니다. 실패하거나 대형/읽기 전용/refused인 파일은 새 peek 모델로 미리 불러오지 않습니다.

현재 native의 typed SDK·LSP 문서 mirror/owner/세대/취소, 문제 이동·workspace 심볼의 현재 창 preview/reveal와 기존 EditorStore/표시 view-zone·원본 명령 registry/keymap을 재사용할 수 있습니다. 정의/선언/타입/구현·참조의 실제 앱 요청 및 본문/peek 소비자는 아직 없으므로 기존 서비스/프로토콜만으로 완료 처리하지 않습니다.

원본 Monaco의 goToCommands/goToSymbol·link gesture·referencesModel·peek widget/controller/tree/CSS와 실제 TS의 openCodeEditor 경계를 더 읽고 단일/복수/없음·이동/옆 그룹/peek/순환·Ctrl/Meta+클릭의 동작을 확정합니다. 새 기능/디자인·원본 버그/내부 수치 강제 재현·engine/의존성·동결 browser 변경·OS 합성 입력은 추가하지 않습니다. 실제 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle을 유지합니다.

## 체크리스트

- [ ] a. 실제 TS/Monaco·공식 typed API·현재 native 공급/선택/열기/원본 화면 경계 대조
- [ ] b. Location/LocationLink·UTF-16·정렬/중복/그룹·현재 요청/선택·peek 모델 수명 검증
- [ ] c. 실제 typed LSP 5종·준비/미지원/빈/오류·취소/편집/닫힘/프로젝트/재시작 공급
- [ ] d. native 명령/기본 키·Ctrl/Meta link gesture·단일/복수 이동·원본 peek 표시/입력 소비
- [ ] e. 실제 앱의 현재 pane/기존/새 탭·옆 그룹·peek target loading/인수·dirty/readonly/reveal 통합
- [ ] f. 변경 크레이트 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 현재 검증과 잔여 게이트

batch20의 구현/검사를 통과했다고 판정하지 않습니다. 변경 없는 경계만 batch19의 editor 206·UI 362·app 663·LSP SDK 81 및 개별 이전 syntax/egui SDK 근거를 재사용합니다. 변경한 크레이트는 f에서 전체 대상을 직접 1회 실행하고 실패 영향만 재검사합니다.

보호 Trash 3·기존 ignored 5·실제 OS/IME/접근성/pixel·대형 성능/soak/패키징/출시·테마의 7자리 HEX 결정은 미완료입니다. experiments/lsp-coordinator-spike standalone lock 불일치와 edition 2021/2024 포맷 차이는 batch19 QA에 기록한 전체 workspace/CI 부채이며 frozen browser 또는 lockfile을 우회하여 해결하지 않습니다.
