# OS·알림 정책 runtime 분리

상태: system action 5개·notification action과 이 단위 자동 검증 완료. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 보존한 정책

`crates/taide-runtime/src/system_actions.rs`와 `notification_actions.rs`가 UI 비의존 정책을 소유합니다. `src-tauri/src/domain/system/commands.rs`·`notification/commands.rs`는 기존 async IPC 인수·타입·결과를 유지하며 참조를 전달합니다. 원래 await가 없는 정책을 runtime에서는 동기 함수로 제공합니다. 이미 workspace에 있는 taide-notification local path 소비만 추가했고 Cargo.lock은 runtime 직접 의존 1줄만 바뀌었습니다. 외부 패키지·버전 변경은 없습니다.

system 경로 3개는 현재 프로젝트 snapshot→strict owning-root 정규화→platform open/reveal/file URL 순서입니다. CLI 허용 경로는 예외가 아니며 symlink로 루트 밖을 가리키는 경로도 전달하지 않습니다. 기존 lenient canonicalization이 허용하는 루트 내 미생성 경로를 새 실재 gate로 거부하지 않습니다. 외부 URL은 기존 검증·트림 뒤에만 전달하고 app-data는 허용 enum 4개의 디렉터리 생성 완료 뒤 reveal합니다. guard·blocking scheduling은 추가하지 않습니다. usage store blocking 호출 2개·PID/CPU/실제 프로세스 label 공급은 기존 Tauri adapter에 유지합니다.

알림은 title/body masking→live settings snapshot→read lock 해제→FnOnce focus callback→decide_delivery→Delivered일 때만 전송합니다. master/category 설정이 억제하더라도 기존처럼 focus를 조회합니다. 모든 실제 창의 is_focused 실패를 false로 취급하는 조회는 Tauri callback에 유지합니다. 전송 오류는 그대로 전파하며 Delivered를 실제 OS 표시 성공으로 해석하지 않습니다. OS 설정 열기는 macOS의 단일 고정 URL/cfg와 비macOS unsupported 결과를 유지합니다. system action 5개·root helper·notification action/helper·기존 unit 2개의 참조 치환/포맷 외 본문 동일성과 usage 2개·OS 설정 adapter의 불변을 비교했습니다.

이전 private helper 문서의 보안 근거도 유지합니다. opener 플러그인 권한을 열지 않고 검증된 command를 유일한 OS 경로 통로로 둡니다. 알림 문자열은 agent/terminal 출력에서 만들어지고 OS 알림 이력은 앱이 사후 마스킹할 수 없으므로 OS 포트에 넘기기 전에 두 문자열을 마스킹합니다. 공개 문서의 이동한 private helper 링크는 일반 코드 표기로, 제거된 import에 의존하던 service 링크는 실제 공개 경로로 수정했습니다.

## 검증 근거

- 변경 전 notification 7건·system 13건과 doc 2건·마스킹 helper 2건·URL 9건·redact 20건이 통과했습니다. 불변 서비스/보안 정책 결과를 재사용합니다. 새 경계는 두 runtime module 부재 E0432(exit 101)로 먼저 실패했습니다.
- `cargo test -p taide --test system_notification_actions_runtime --test platform_services_runtime --test taide_notification_extraction --test taide_system_extraction --test domain_boundaries --quiet`: 새 경계 10건·platform 4건·공개 경로 2건·도메인 3건으로 19건 통과, exit 0입니다. UUID synthetic fixture만 사용해 root/CLI/symlink 거절의 무호출, URL gate, 디렉터리 생성 순서/실패, snapshot/focus/전송 순서와 오류 전파를 확인합니다.
- `cargo test -p taide-runtime notification_actions:: --quiet`: 원본에서 옮긴 masking unit 2건 통과, exit 0입니다. 테스트 이름의 OS 대문자 경고는 소문자 식별자로 수정했고 테스트 정책은 불변입니다. runtime/Tauri all-target clippy·strict runtime rustdoc·fmt/diff는 exit 0입니다. normal runtime dependency graph에 Tauri가 없습니다. strict runtime rustdoc 성공을 기존 Tauri 전체 문서 성공으로 확대하지 않습니다.
- 실제 bindings 생성 1건이 통과했고 생성 diff는 공개 문서 링크 2줄뿐입니다. 실제 SHA-256 f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a로 manifest를 동기화한 뒤 `cargo test -p taide --test rust_native_phase0_contract --quiet` 최종 7건이 통과했습니다. 변경 후 서로 다른 검사 총 29건입니다.

실제 OS 열기·알림·사용자 설정/파일·시크릿/키링·프로세스 조사·앱 실행은 하지 않았습니다. 전체 workspace/frontend/GUI와 M6의 잔여 action/lifecycle·M7/M8은 미완료입니다. 일반 push는 기존 승인 범위의 저장소·브랜치에 M6 전체 완료 후 수행합니다.
