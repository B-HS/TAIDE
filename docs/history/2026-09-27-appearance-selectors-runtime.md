# 현재 테마·언어 selector runtime 분리

상태: selector 이전과 이 단위 자동 검증 완료. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 보존한 정책

`crates/taide-runtime/src/theme_actions.rs`와 `locale_actions.rs`는 현재 테마·언어 선택 정책을 소유합니다. `src-tauri/src/domain/theme/commands.rs`·`locale/commands.rs`의 current command만 runtime으로 위임하며 기존 async IPC 시그니처·타입·공개 문서는 불변입니다. 원래 body에 await가 없으므로 native 소비자는 동기 함수와 빌린 system 문자열을 사용합니다. 이미 workspace에 있는 taide-theme·taide-locale local path 소비만 추가했고 Cargo.lock은 runtime 직접 의존 2줄만 바뀌었습니다. 외부 패키지나 버전 변경은 없습니다.

테마는 동일 settings read scope에서 follow_system_theme와 theme_id를 선택한 뒤 lock을 놓고 load_theme를 호출합니다. system theme는 기존 builtin_id_for_system의 대소문자 처리·dark fallback을 유지하고 직접 지정한 사용자 테마의 load 오류는 그대로 전파합니다. 언어는 현재 language를 clone한 뒤 resolve_language→load_locale 순서입니다. system 언어 prefix와 명시 언어/없는 언어의 fallback, 실제 있지만 파손된 사용자 pack의 오류를 보존합니다. 두 selector 모두 설정 쓰기·이벤트·mutation guard를 추가하지 않습니다. 원본 command body와 참조 치환/포맷 외의 정책 동일성을 비교했습니다.

나머지 theme 4개·locale 2개·snippet 단순 위임과 OS system 값 공급은 기존 adapter/소비자 경계입니다. 기존 service를 중복하는 새 action은 만들지 않으며 task blocking·font OS cache·전체 command body 판정과 프로젝트/LSP/PTY 잔여 경계는 이번 완료 범위가 아닙니다.

## 검증 근거

- 변경 전 `cargo test -p taide-theme -p taide-locale --quiet`: theme 49건·locale 18건 통과, exit 0입니다. service는 변경하지 않았으므로 67건의 근거를 재사용합니다.
- 새 appearance_actions_runtime은 module 부재 E0432(exit 101)로 먼저 실패했습니다. 새 5건은 live 설정/사용자 테마·pack, system fallback, load 오류, mutation guard를 이미 보유한 상태의 읽기, 설정 불변과 adapter 위임을 직접 확인합니다.

- `cargo test -p taide --test appearance_actions_runtime --test taide_theme_extraction --test taide_locale_extraction --test rust_native_phase0_contract --test domain_boundaries --quiet`: 새 selector 5건·기존 공개 경로 2건·IPC 7건·도메인 3건으로 변경 후 17건 통과, exit 0입니다. 기존 core/layout unit body는 바뀌지 않아 이번 selector 단위에서 반복 실행하지 않았습니다.
- `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`와 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`: exit 0입니다. fmt·diff·변경 문서 Prettier 검사도 exit 0입니다. strict runtime rustdoc 성공을 Tauri 전체 성공으로 확대하지 않습니다.
- bindings SHA-256은 e69d19c6da72dd6695fcb29dc54a4a75c123f8ef015c539b75404dee1194f1d6으로 불변이며 signature/문서도 불변이므로 생성 검사는 반복하지 않습니다. normal runtime dependency graph에 Tauri는 없습니다.

UUID fixture의 합성 설정·테마·언어 파일만 사용하며 실제 사용자 설정·시크릿·키링·앱 실행/OS 폰트 스캔에는 접근하지 않습니다. 전체 workspace·frontend·GUI와 M6 전체는 미완료이고 일반 push는 승인된 저장소·브랜치에 M6 전체 완료 후 수행합니다.
