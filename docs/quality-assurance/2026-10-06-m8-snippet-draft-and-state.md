# M8 Snippets 공용 초안·retained 상태

후속 UI/생산자와 actual native/browser 소비자는 `2026-10-06-m8-snippet-editor-and-producer.md`, `2026-10-06-m8-snippet-consumers.md`에 기록했습니다. 아래의 화면/소비자 미구현 문장은 이 core 경계 당시 이력이며 현재 상태는 후속 문서를 따릅니다.

## 대상 파일

- `native/taide-native-ui/src/snippet-draft.rs`, `snippet-editor-state.rs`, `lib.rs`, 기존 `keybinding-search.rs`입니다.
- `native/taide-native-ui/tests/snippet-draft.rs`, `tests/fixtures/snippet-draft-parity.json`입니다.
- 공용 UI manifest와 native UI/App/remote web/browser probe lock의 기존 serde 직접 의존성 간선입니다.

## 리포트

원본 `src/shared/lib/snippet-draft.ts`, `snippet-file.ts`, `widgets/snippet-editor/snippet-editor.tsx` 및 `new-snippet-file-dialog.tsx`의 초안 계약을 공용 Rust로 구현했습니다. 화면이나 native/browser 소비자의 완료가 아닙니다. 제품 TypeScript·기존 backend service를 수정하지 않았습니다.

## 상세

1. 필수 이름/prefix/body, 완전히 빈 행 제외, description/scope만 채운 행의 미완성 판정, 미완성 우선·trim 후 case-sensitive 중복 검사, ID 제외/원본 필드·순서 그대로 dirty 비교입니다. prefix만 쉼표 분리, body는 끝 빈 줄을 포함한 줄 배열, description은 쉼표를 분리하지 않는 문자열, 빈 description/scope 생략입니다.
2. 원본 ECMAScript trim을 보존합니다. 기존 검색 코어의 같은 공백 판정을 crate 내부로 공유하며 FEFF 제거/NEL 보존을 원본 기대값과 비교했습니다. 파일명은 전역 suffix의 대소문자 구분·기존 slash/backslash/colon/두 점 거절·원본31 language 순서입니다. 실제 파일 접근은 기존 서버의 추가 검증을 그대로 사용할 예정입니다.
3. 저장 JSON은 기존 모델과 serde SerializeMap/PrettyFormatter로 4칸 들여쓰기·끝 newline 없음·필드 순서를 보존합니다. JS Object.fromEntries의 일반 키 첫 삽입 순서/중복 마지막 값과 0~4294967294 canonical 숫자 키의 선행 숫자 정렬도 보존합니다. serde_json 기본 BTreeMap으로 초안을 다시 정렬하지 않습니다. 로딩은 서버 BTreeMap의 wire 순서에 JS 숫자 키 규칙만 적용합니다.
4. retained 상태는 같은 파일 선택 무동작·dirty 전환/나가기 보류·Cancel·확정 일회성 소비·목록 재조회 시 현재 초안/ID 보존·늦은 첫 목록 로딩·조건부 전역 scope·행 삭제·파일 삭제 이후 선택 초기화입니다. ID는 takeover 상태 안에서 증가하는 opaque 행 식별자이며 UUID 바이트 호환을 주장하지 않습니다. 새 파일 Dialog의 언어/전역 이름은 닫았다 열어도 유지하고 기존 파일과 정확한 이름 중복을 막습니다. 저장 validation과 content 생성은 원본처럼 별도이며 실제 handler 연결 때 validation을 먼저 소비해야 합니다.
5. 기존 serde 1의 직접 의존성만 추가했습니다. 임의 새 라이브러리/버전/MSRV 변경은 없고 네 lock의 taide-native-ui 목록에 serde 한 간선씩 추가했습니다. 설치된 serde SerializeMap·serde_json Serializer/PrettyFormatter API를 확인했습니다. 현재 실제 owner/remount/비동기 요청 token·캐시 무효화·실패/종료 drain은 연결하지 않았습니다.

## 검증

- [x] 원본 Bun 함수에서 합성 입력으로 직렬화/검증10건·파일명9건·dirty5건·map 로딩의 기대값을 생성했습니다. fixture는 이 실제 원본 실행 결과이며 Rust 구현의 자체 기대값이 아닙니다. 숫자 경계/음의 0/leading zero/중복 덮어쓰기·FEFF/NEL·개행/escaping/빈 행·미완성·case-sensitive 이름을 포함합니다.
- [x] 신규 portable core/상태 검사2건 첫 실행 PASS입니다. build2.33초·suite0.00초·failed0/ignored0/filtered0입니다. 상태 검사는 하나의 retained 인스턴스에서 선택 전 목록 없음→늦은 목록→편집→Cancel→재조회→나가기 확정 일회성→전역 선택→빈 행/미완성/행 삭제→새 파일 값 보존/unsafe/중복→파일 삭제 초기화를 연속 검사합니다.
- [x] native App lib/bins/tests check11.15초 exit0입니다. 기존 Wry17 경고 외 새 경고는 없습니다.
- [x] 실제 normal remote Canvas Wasm check2.48초 exit0입니다. 브라우저 실행/새 bindings 생성 증거가 아니며 기존 bindings는 직전 키바인딩 inspection 소스입니다.
- [x] Rust 신규/모듈4 exactfmt와 git diff --check exit0입니다. 동일 성공 검사는 반복하지 않았으며 실제 Snippets backend service 검사는 재실행하지 않았습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --test snippet-draft --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib --bins --tests --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 남은 범위·다음 구현

- [ ] Settings의 Snippets 목차/제목/Manage·Folder 버튼과 전체 takeover pane입니다. 기본 Section은 아직7개이며 Snippets를 추가하면 원본 Editor 다음/Terminal 이전입니다.
- [ ] 원본 name/prefix/body/description/global scope 구조화 입력과 sidebar256·body textarea4행·Trash2/Plus/FolderOpen 아이콘·새 파일/삭제 파일/삭제 행/dirty 폐기 Dialog입니다. Monaco/JSON textarea/임시 화면으로 대체하지 않습니다.
- [ ] native/browser typed snippet_list/save/delete와 목록 invalidation·Toast·owner/remount·stale/late 응답·쓰기 실패/close drain 및 실제 연속 UI 검증입니다.
- [ ] 전체 시각/AX/theme/DPI/modifier/Presence/IME·전체 App/assets/나머지 Settings·최종 N1~N8/Rust99%입니다. 실제 CJK/VoiceOver는 합의대로 사용자 실기·최후 순위입니다.

Snippets 세부1/4(25%)·provider2/4(50%)·전체363/433(83.83%, 부모 집계 중복 증가 없음)·최종0/8·ETA 산정 보류입니다. goal active·main 직접·workflow/서브에이전트 없음·전체완료 전 commit/push 없음·live handle 없음입니다. 보호 앱·OS 설정·사용자 프로젝트·Keychain·원격 저장소는 건드리지 않았습니다.
