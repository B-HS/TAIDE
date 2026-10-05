# M8 Settings의 앱 파일 버튼·실제 문서 소비

## 대상·계약

`native/taide-remote-web/src/app-file-opens.rs`가 원본 `layout_open_tab`/AppFile(Settings)/settings.json/preview=false를 현재 Settings owner의 project·pane에 보냅니다. 실제 BrowserEditor가 Output.open_settings_file을 소비하고 Workbench가 matched seq 응답을 소유합니다. ShellState는 실제 ProjectLayout ack를 적용하되 제거된 프로젝트와 과거 revision은 무시합니다. 기존 서버 allowlist/정책/인가·OMLX 설정 보호·OS 권한은 변경하지 않았습니다.

`app-files.rs`는 기존 FileViews와 같은 EditorStore를 사용합니다. AppFileTarget은 path 없는 원본 typed 대상이며 읽기/쓰기에는 app_file_read/app_file_write만 사용합니다. 여러 owner는 같은 DocumentKey 문서를 공유합니다. mount/read/write ticket을 비교해 늦은 응답·중복·다른 remount를 차단합니다. 저장은 단위 ack 뒤 canonical 재읽기까지 pending이며 원본 mark_app_file_saved가 저장 중 추가 편집을 보존합니다. Settings 변경은 깨끗한 문서만 refresh하고 dirty 문서는 덮어쓰지 않습니다. 명시 retry만 수행하며 읽기 실패의 frame별 재시도는 없습니다.

BrowserEditor의 실제 show_app_file은 원본 NativeEditor/동일 표시 테마·Settings indent를 사용합니다. 실제 Command-S를 app file 저장에 연결하고 공유 문서의 탭 dirty를 기존 DirtyState로 전송합니다. native request_tab_save와 TS AppFilePane 모두 일반 파일 cleanup 앞에서 앱 파일을 별도 처리하므로 새 소비도 앱 파일에 일반 file-save cleanup을 추가하지 않습니다. AppFile은 원본 계약대로 mirror/LSP/외부 파일 watcher 경로를 사용하지 않습니다. 전체 AI/editor 기능의 완료는 주장하지 않습니다.

쓰기 실패는 기존 app_file_failed Toast(설정 JSON 전용 제목)를 사용합니다. 피드백과 일회성 종료 실패 채널을 분리해 과거 오류가 재시도 성공 후 종료를 다시 실패시키지 않습니다. 쓰기/canonical 및 탭 열기 ack까지 종료를 대기하고 실패 시 연결/초안을 유지합니다. owner 제거 후 이미 전송한 저장도 ack까지 pending으로 남되 사라진 owner의 문서에 결과를 적용하지 않습니다.

## 검증

- [x] 새 portable open1 첫 PASS(2.53초/.00초): typed args/current auxiliary pane/seq/거절/중복/Closed/malformed.
- [x] 새 ShellState ack1 첫 PASS(.36초/.00초): 과거 revision·모르는/제거된 project 적용0.
- [x] 새 공유 AppFiles1 첫 PASS(1.43초/.00초): 읽기/공유 view·저장 ack→canonical·추가 편집 보존·실패 보존·settings invalidation·다른 mount late reply 폐기·disconnect 단일 오류. pending owner 제거 뒤에도 전송 저장 ack를 대기하도록 변경한 영향 입력을 추가한 최종1 PASS(1.74초/.00초). 같은 성공 입력 반복이 아닙니다.
- [x] 새 실제 Chrome/Wasm 연속1 PASS: 원본 Settings 버튼→실제 앱 파일 surface→Meta-A/합성 JSON 입력→Command-S held 저장→Close Pending→의도 거절/Failed/원본 Toast/dirty 초안/socket1→Cancel→명시 retry→canonical 읽기/dirty false→Ready/socket0·quiet1.1초 frame/pump/request 불변. seq16/open1/read2/write2/dirty2·실패 frame24/pump29→최종35/43·page error/panic/unclaimed0입니다. 첫 실행의 기존 Catalog 의도 locale 거절이 새 mode에 적용된 fixture 조건만 수정했습니다. 성공 뒤 반복하지 않았습니다.
- [x] actual probe build4.71초·normal production Wasm/canvas strict .39초·TS strict/Prettier·Rust13 exactfmt exit0. 초기 Rust E0502는 owned Operation match로, AppError equality와 DocumentId Ord 오용은 기존 Failure 직렬화와 HashSet으로 수정했으며 검사 억제/의존성 추가는 없습니다.

실제 성공 자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-app-file-result.json`입니다. 그 실패 JSON/PNG는 초기 Catalog fixture 실패의 역사 자료로 최신 성공 화면으로 제시하지 않습니다. 이번 성공은 실제 WebRunner/원본 surface frame·core 문서·RPC·종료 관찰이며 새 픽셀 screenshot/전체 시각 parity를 주장하지 않습니다. 합성 backend에서 원본 wire 계약을 재현했으며 실제 사용자 설정이나 OS 파일을 읽고 쓴 검사가 아닙니다.

## 남은 범위

이 Settings/AppFile 하위 소비 경계만 완료입니다. 모든 Prompt/탭/창 caller·전체 Native App/원격 App·LSP/AI·Settings 나머지 섹션·assets/bundle·unsaved close dialog·최종 security/perf/beta/TS 제거/Rust99 gate는 남습니다. provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류·M8 미완료·goal active·main 직접·전체완료 전 Git 없음입니다. 최신 probe bindings는 이 AppFile 소스이며 이전 성공은 당시 소스 근거를 재사용합니다.
