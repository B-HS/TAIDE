# M8 native Open With와 PNG preview 선행 연결

## 기준과 구현 범위

원본 `src/shared/lib/{file-extension,preview-kind}.ts`, `src/entities/editor/open-with-registry.ts`, `src/entities/layout/tab-path-change.ts`, `src/widgets/{explorer/explorer-container,editor-area/pane-node-view,preview-pane/preview-pane}.tsx`, `src/features/{explorer/file-tree-context-menu,preview/image-preview}.tsx`, `src/app/providers/ipc-sync-provider.tsx`를 대조했습니다. 기존 원본은 8종 provider·26개 확장자를 분류하며 image는 중앙 정렬·양방향 스크롤·16px padding·자연 크기를 초과하지 않는 contain 표시입니다. image zoom 버튼은 현재 원본에 없습니다.

대상은 native app의 `src/{open_with,preview,events,explorer,host,application}.rs`, lib module/Cargo와 `tests/{open-with,preview,preview-events,explorer}.rs`입니다.

1. 파일 context의 Open With 하위 메뉴에 기존 3개 locale key를 사용합니다. preview 대상 파일에만 editor/preview 항목을 표시하고 directory·dotfile `.png`·비대상 확장자에는 숨깁니다. 선택은 window-local raw-path registry를 먼저 바꾼 뒤 기존 파일 열기 요청을 preview=false로 보냅니다.
2. editor override는 최대 200개·쓰기 순서 eviction이며 조회는 순서를 바꾸지 않습니다. Preview 선택은 override를 지웁니다. main/aux File 탭의 실제 path 이동과 revision을 따라 이전 선택을 옮기고, 해당 project의 마지막 동일 raw-path File 탭 close/삭제·project close를 구분해 정리합니다. 파일 close는 stale controller cache가 아닌 실제 state layout을 확인합니다. 다른 native window의 별도 registry에는 영향을 주지 않습니다.
3. File 탭을 preview와 editor surface로 분기합니다. 기본 PNG preview에서는 EditorStore 문서를 열지 않습니다. editor 선택 시 기존 문서 admission/editor 경로를 사용하며 preview에서 돌아오는 focus 추적을 초기화합니다. PNG의 전체 실제 OS 창 입력/픽셀 parity를 검사한 것은 아닙니다.
4. 기존 lock에 이미 있는 image 0.25.10의 PNG feature를 직접 재사용합니다. 새 package/version·root Cargo/MSRV·원본 앱 변경은 없습니다. 고정 image의 ImageReader/Limits/ImageDecoder/DynamicImage, egui/epaint 0.36.2의 texture/image/menu API를 로컬 공식 source로 확인했습니다. native manifest/lock의 직접 dependency edge만 추가했습니다.
5. PNG는 typed host request→root/CLI 승인·owned mutation/operation→TaskSupervisor blocking read/decode→token 검증→UI texture 업로드→중앙 contain 이미지로 이어집니다. 실제 file-read-raw 20MiB 정책을 재사용하며 malformed·권한 밖/relative·닫힌 project/shutdown·변경된 canonical 승인을 거절합니다. image 실패에는 원본 locale의 외부 열기 버튼을 기존 root 검증 system_open_path로 연결했습니다. 나머지 7종 provider는 미연결 표시를 유지하고 완료로 세지 않습니다.

## 캐시와 변경 이벤트

한 native window cache에서 동시에 admission 중인 이미지 요청은 하나입니다. editor 전환·탭/project close·rename·파일 변경 시 loading entry/texture를 정리하되 기존 active request의 token은 회신까지 유지합니다. 정리된 entry의 늦은 회신은 새 캐시에 들어가지 않습니다. host submission 실패·종료 취소·재연결은 pending state를 별도로 해제합니다. root rescan은 Path component 경계로 제한합니다.

PaintSink의 실제 FsChanged/FsRescanRequired를 기존 TreeChanges에 연결했습니다. preview invalidation은 tree refresh와 분리하여 from_app modified도 처리합니다. 정확한 변경 path는 dedupe하며 512개 초과 시 bounded 전체 invalidation으로 전환합니다. rescan project는 기존 64개 project 경계를 사용합니다. cache invalidation을 queued reply 처리 전에 소비하므로 앞선 generation의 디코딩 결과를 표시하지 않습니다.

decoded RGBA는 64MiB·texture cache는 128MiB로 제한하고 GPU가 보고한 max_texture_side를 worker와 upload 양쪽에서 검사합니다. 이 값은 격리 native 선행 보호이며 원본의 추가 사용자 정책으로 확정한 것이 아닙니다. 기존 브라우저에서 표시 가능한 대형 이미지의 native tiling/다운샘플·메모리·품질 parity는 남습니다. image Limits.max_alloc은 모든 decoder 임시 할당에 대한 strict process-memory 증명이 아니며 encoded/RGBA/ColorImage/GPU 복제와 backend texture 해제 시점도 전체 memory gate에서 확인해야 합니다. M7 remote 전송 정책을 이 cache 정책으로 변경한 것은 아닙니다.

## 실제 검증과 실패 정정

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test open-with --test explorer open_with -- --nocapture`: 메뉴 1건 PASS(0.14초), 최초 registry 2건 PASS(0.00초), compile 1.99초입니다. 26개 확장자/case·dotfile·비대상, 200개 쓰기순 eviction/조회 no-touch/raw-path/window 분리·rename·project/aux close와 실제 submenu 두 선택을 확인했습니다.
- [x] layout 삭제의 override 정리를 추가한 영향 검사 `--test open-with --test preview`: 최종 registry 2건 PASS(0.00초), 실제 PNG host 1건 PASS(0.02초), compile 1.90초입니다. 합성 PNG의 실제 디스크 읽기·RGBA/alpha·egui upload delta·2×1 중앙 natural-size 페인트, 닫힌 loading/stale token 거절·invalid pixels·pending 재시도·outside/relative/malformed/renderer size/닫힌 project/shutdown 거절·disconnect/shutdown/tracked_count 0을 확인했습니다. PNG를 실제 macOS GPU 창에서 본 검사는 아닙니다.
- [x] `--test preview-events`: 1건 PASS(0.00초·compile 3.15초)입니다. self-write modified는 tree invalidation 없이 raw preview를 갱신하고 정확한 path/한 번 drain·rescan·512개 포화→전체 invalidation, `/root`와 `/rootish`의 구분·inflight 늦은 upload 거절·다음 admission과 전체 정리를 확인했습니다. 이전 PNG/메뉴/Copy/Cut/Trash/PTY 성공을 반복하지 않았습니다.
- 최초 lib compile은 잘못된 ShellIntent 직접 variant와 layout revision 타입으로 실패했고 실제 ShellMutation::FocusPane/u32로 정정했습니다. 첫 fixture compile은 TaskSupervisor handle, Explorer rows map, texture delta map/smallvec 형태를 잘못 사용해 실패했고 실제 API로 수정했습니다. PNG 초기 페인트 기대는 Mesh였지만 egui Image는 textured Rect를 사용합니다. pinned paint_texture_at/Shape::texture_id로 원인을 확인해 실제 texture와 rect/위치/픽셀을 검증했습니다. assertion 실패 중 delta Drop의 secondary abort는 fixture에서 delta를 먼저 명시적으로 clear하도록 수정했습니다. 제품 assertion을 제거하거나 실패 검사를 통과로 세지 않았습니다.
- [x] app lib/bin 및 explorer/open-with/preview/preview-events/host strict clippy는 마지막 exit 0(1.37초)입니다. 최초 filter_map_bool_then 경고는 동일 filter→map으로 수정했습니다. 동작이 같은 registry 성공은 재사용했으며 검사기 allow/우회는 없습니다. 변경 Rust 파일 rustfmt와 git diff --check는 exit 0입니다.

## 계속 구현할 경계

- [ ] JPEG/GIF/WebP/BMP/AVIF·SVG sanitize/raster와 animation/lifecycle·대형 이미지·decode crash/TOCTOU/file growth·aggregate process/GPU memory를 구현·검증합니다. PNG 첫 프레임 통과를 전체 image provider 완료로 세지 않습니다.
- [ ] video/audio·PDF·sandbox HTML·spreadsheet·PPTX·HWP/HWPX의 실제 provider·오류/취소/파일 변경·locale/theme·resource 소유를 연결합니다. 미연결 안내·텍스트 에디터·외부 열기만으로 in-app preview 동등성을 대체하지 않습니다.
- [ ] 실제 AppSurfaces의 editor↔preview 전환/dirty 상태·project/aux close·root outside CLI·focus/scroll/AX·메뉴 픽셀·3개 locale와 외부 열기 OS 경계를 전체 host/window gate에서 검사합니다. 기존 사용자 실기 app/bundle·clipboard·입력기·VoiceOver·사용자 파일은 조작하지 않았습니다.

N5-P1·N5 및 M8 상위 N1부터 N8은 미완료입니다. TS 제거·최종 제품 의존성 채택·배포·M8 완료·commit/push를 수행하지 않았습니다. 성공 증거를 재사용하고 다음은 남은 raster decoder/형식과 provider 경계를 이어갑니다.
