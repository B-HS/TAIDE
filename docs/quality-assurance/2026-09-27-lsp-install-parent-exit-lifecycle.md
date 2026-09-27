# LSP 설치 부모 선종료 QA

## 대상 파일과 리포트

대상은 Unix infra의 회수 없는 child 관찰/그룹 종료와 runtime 설치 child의 회수 플래그입니다. 직접 만든 sh/sleep·PGID anchor·UUID marker만 사용합니다. 실제 설치기·앱·사용자 파일/프로세스·시크릿은 사용하지 않습니다.

## 상세 검증

- [x] `cargo test -p taide-runtime --lib lsp_install_toolchain::tests::부모_선종료 --quiet`: 선행 재현은 슬롯 반환 뒤 자손 생존으로 exit 101이었습니다. fixture의 자기 anchor가 실패 cleanup에서도 그룹을 소유합니다.
- [x] 첫 구현의 toolchain 14건 중 정상/실패 exit code 검사 2건이 EPERM으로 실패했습니다. Apple 원천과 대조해 좀비 부모만 남은 자기 그룹을 판별하도록 보완한 뒤 같은 toolchain 14건이 통과(exit 0)했습니다. 다른 권한 오류를 포괄적으로 무시하지 않습니다.
- [x] `cargo test --offline -p taide-infra --lib owned_child::tests --quiet`: 2건 통과(exit 0), 살아 있는 자기 child 관찰/KILL과 종료 관찰 반복/빈 그룹 정리 뒤 exit code 7 보존을 확인했습니다.
- [x] `cargo test --offline -p taide-runtime --lib lsp_install_ --quiet`: 늦은 취소 검사 추가 뒤 최종 29건 통과(exit 0)했습니다. 샌드박스 실행은 24건 통과/localhost bind 제한 5건 실패(exit 101)였으며 동일한 자기 fixture만 사용하는 승인된 실행 환경에서 모두 통과했습니다. 앞의 toolchain 14건은 29건에 포함되며 이번 서로 다른 검사는 runtime 29·infra 2로 31건입니다.

## 정적 검사와 재사용

- [x] `cargo clippy --offline -p taide-infra -p taide-runtime -p taide --all-targets -- -D warnings`: exit 0입니다. 코드/검사 입력이 같은 선행 성공은 재사용하며 전체 crate 회귀를 통과했다고 주장하지 않습니다.
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --offline -p taide-infra -p taide-runtime --no-deps --quiet`: exit 0입니다. 공개 API 계약과 libc ABI 사용을 확인했으며 외부 앱 실행은 없습니다.
- [x] `cargo fmt --all -- --check`, `git diff --check`, 대상 history/QA 4개 Markdown의 `bunx --no-install prettier --write`: 모두 exit 0입니다. 변경 없는 전체 PROCESS/architecture 포맷을 재작성하지 않습니다.
- [x] bindings/IPC manifest와 Tauri adapter·DTO·이벤트 payload는 불변입니다. `src/shared/api/bindings.ts`의 SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`입니다. 같은 입력의 bindings 생성·IPC baseline·adapter 성공은 재사용합니다. Cargo.lock의 libc 버전/체크섬은 동일하며 infra 직접 의존 한 줄만 추가했습니다.

## 남은 gate와 테스트 부채

- [ ] Linux/다른 Unix·Windows process tree는 현재 macOS 검사로 통과 처리하지 않습니다. 다른 OS 지원 gate에서 같은 fixture를 실행합니다.
- [ ] 그룹을 벗어난 자손·권한 변경·여러 좀비만 남은 그룹은 이번 단일-parent 판별로 완료하지 않습니다. 그룹 종료 실패를 보고하며 무관한 PID에 대한 추가 시그널이나 권한 우회는 하지 않습니다.
- [ ] 시그널 전달은 모든 자손의 wait/join이 아닙니다. 이 fixture는 자기 자손 PID 제거를 관찰하지만 모든 자손의 bounded 종료를 보장하지 않습니다.
- [ ] native 종료 실기·비설치/nested worker·LSP wait/PTY·전체 M6/M7/M8·Phase 0 gate는 그대로 미완료입니다.
