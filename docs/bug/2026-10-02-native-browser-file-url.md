# Native 내부 파일 URL의 거절·구분자 해석

## 증상·대상

대상은 `native/taide-native-app/src/bootstrap.rs`의 NativePlatform.open_url과 `platform_url`, 기존 `crates/taide-runtime/src/system_actions.rs`, `crates/taide-system/src/service.rs`입니다. 탐색기의 HTML Open in Browser를 연결하면서 native 포트가 root guard를 통과한 file://까지 외부 URL로 거절하는 코드를 확인했습니다.

## 원인과 수정

- NativePlatform의 open_url은 외부 링크용 HTTP(S) whitelist를 내부 파일 URL에도 적용했습니다. 외부 system_open_external_url은 별도 guard를 통과해야 하고 내부 system_open_in_browser는 열린 프로젝트의 canonical root guard 뒤에 file_url을 전달하므로 두 경계를 구분해야 합니다.
- 기존 shared file_url은 raw 경로를 문자열로 붙입니다. 파일명 안의 #/?/%·공백·Unicode는 URL 구분자/escaping과 구분되지 않습니다. shared/Tauri API·기존 저장/검증 계약은 이번 native 변경에서 수정하지 않았습니다.
- native 포트는 trusted runtime에서 온 raw file:// suffix를 absolute Path로 변환하고 설치된 공식 url 2.5.8의 Url::from_file_path로 인코딩합니다. 다른 URL은 기존 validate_external_url을 유지합니다. OS open은 고정 executable·별도 argv·--를 유지하며 shell interpolation은 없습니다. helper는 root authorization의 대체물이 아닙니다.

## 검증·제한

- `native/taide-native-app/tests/explorer-system.rs`의 URL 경로 왕복·scheme 경계와 actual host/mock platform/root 밖/symlink 거절 신규 2건이 0.01초에 통과했습니다. initial code source로 원인을 확인했고 수정 전 실제 OS open/실패 실행을 했다고 주장하지 않습니다.
- 실제 browser 실행·다른 OS/invalid non-UTF8 경로·shared/Tauri URL 수정·전체 OS authorization과 M8 완료는 미완료입니다. 상세 실행/경계는 explorer-system-menu QA에 있습니다.
