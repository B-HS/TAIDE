# M8 실제 IDE 상태 표시

## 대상과 원본

대상은 `native/taide-native-app/src/status-ide.rs`, `application.rs`, `presentation-refresh.rs`, `lib.rs`, `resources/status/{plug-zap,plug,unplug}.svg`입니다. 원본은 `src/features/window/status-bar.tsx`와 locale의 `ide.connected/starting/disconnected/title`입니다.

AppSurfaces가 실제 서버의 `services.ide.status()`를 읽습니다. connected 우선·running 대기·off의 원본 판정 순서를 유지하며, 연결됨만 statusIndicator.success이고 나머지는 appSidebar.iconDefault입니다. 같은 상태를 별도 UI map이나 매 프레임 IPC에 복제하지 않습니다. 실제 서버의 IdeStatusChanged는 기존 PaintSink의 repaint 경로에 연결돼 있습니다. 시작/연결/해제/중지 서버 구현은 변경하지 않았습니다.

기존 locale·11px 글꼴·12px 아이콘·4px gap을 사용하며 실행 중일 때만 포트를 확장한 top-aligned Tooltip을 표시합니다. off에는 tooltip이 없습니다. Label/hover widget을 사용하고 임의 조작 button은 추가하지 않았습니다. 초기 생성·정상 테마 변경·preview 모두 Appearance를 갱신합니다. 토큰·lockfile 내용은 렌더러로 넘기지 않습니다.

공식 Lucide SVG 경로·round cap/join을 기존 resvg0.48.1/usvg와 egui 이미지로 렌더링합니다. 리소스는 include_bytes로 고정하고 currentColor만 white로 바꿔 theme tint를 적용했습니다. SVG 상대/외부 이미지 resolver는 비활성입니다. 배율이 같으면 texture를 재사용하고 변경 시 원자적으로 교체하며 최대 raster side1024를 유지합니다. 기존 LICENSE-LUCIDE ISC/MIT 고지를 유지합니다.

## 최소 검증

명령 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`, 공통 flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 최초 `test --lib status_ide::tests`는 compile11.40초/suite0.02초·FAIL(handle66808)입니다. fixture가 egui Image의 Mesh를 기대했지만 설치된0.36.2는 회전 없는 Image를 texture brush의 RectShape로 출력합니다. 공식 paint_texture_at/Brush 코드를 읽고 검사를 실제 출력 형식에 맞췄습니다. 제품 코드나 검사 기준을 끄지 않았으며 제품 렌더 실패 RED로 분류하지 않습니다.
- [x] 실패 검사만 같은 필터로 다시 실행해 1 PASS·compile4.37초/suite0.03초(handle47089), filtered220입니다. 실제 IdeStore의 default/start/client connect/disconnect/stop·connected 우선·포트 tooltip 문구/off 없음·원본 label/font/color·선택된 SVG texture/12px rectangle/gap4·세 SVG raster의 비어 있지 않은 중성 픽셀·상태 변경 중 upload 없음·2x 배율 교체/free·최종 texture 해제·fixture 소유 task 회수를 확인했습니다. mark_started에 fixture pending task를 사용했으며 실제 socket start/stop 검사로 주장하지 않습니다. 사용자 파일·lockfile을 생성하지 않았습니다.
- [x] 변경된 native `clippy --lib --bin taide-native-app --tests -- -D warnings` exit0·16.54초(handle91547), authored Rust4 exact rustfmt입니다. 기존 Wry dependency17 warnings는 별도입니다. tracked whitespace exit0·신규 Rust4/SVG3 no-index whitespace 출력 없음(exit1은 신규 diff)입니다.
- [x] 기존 실제 IDE WebSocket/인증/MCP/notification/pending/late token/stop·task0의 서로 다른4검사 성공은 재사용했습니다. 직전 LSP 상태와 cursor/font controls 성공도 재실행하지 않았습니다.

Cargo는 직렬/locked/offline/기존 target입니다. 새 의존성·manifest/lock·root/Tauri/MSRV·제품TS·Git 변경은 없으며 사용자 home·clipboard·앱·보호 bundle·OS 설정/IME/VoiceOver·키 파일에 접근하지 않았습니다.

## 확인한 공식 API

설치된 egui0.36.2의 Image/Label/Tooltip/Popup.align·RectAlign.TOP·texture cache/TexturesDelta·RectShape/Brush를 확인했습니다. resvg/usvg0.48.1의 render/Options·image resolver도 읽었습니다. 공식 Lucide [plug-zap](https://github.com/lucide-icons/lucide/blob/main/icons/plug-zap.svg), [plug](https://github.com/lucide-icons/lucide/blob/main/icons/plug.svg), [unplug](https://github.com/lucide-icons/lucide/blob/main/icons/unplug.svg) 공개 SVG를 읽었습니다. pinned 원본 binary의 픽셀 동등성을 검증했다고 주장하지 않습니다.

## 미완료

- [ ] 실제 GUI hover/top tooltip/focus/접근성·DPI/anti-alias·좁은 창/다중 창·원본 전체 status bar 비교는 최종 화면 gate에서 확인합니다. 여기서는 tooltip 생성 여부·문구와 top 배치 코드를 확인했으며 tooltip 실제 픽셀/타이밍은 미측정입니다.
- [ ] 시스템 사용량/detail·Problems/chord·LSP diagnostics/provider/재획득·실제 App production startup/IDE diff/save 화면은 남습니다.
- [ ] actual App assets/effects/ports owner·Settings/AppFile·Rust remote UI·keybinding RED/PTY remount·N1~N8 0/8·최종 TS 제거/배포는 미완료입니다. 전체 완료 전에 commit/push하지 않습니다.
