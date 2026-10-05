# M8 native 탐색기의 경로 메뉴·시스템 연결

## 원본과 구현 범위

- 원본은 `src/features/explorer/{file-tree-context-menu.tsx,explorer-shortcuts.ts}`, `src/widgets/explorer/explorer-container.tsx`, `src/shared/lib/{relative-path,preview-kind,file-extension,copy-text-to-clipboard}.ts`와 설치된 Radix context-menu Trigger source입니다.
- 대상은 `native/taide-native-app/src/{explorer,host,application,bootstrap}.rs`, `tests/{explorer,explorer-system}.rs`입니다. 기존 runtime `system_actions::{system_reveal_path,system_open_in_browser}`와 root guard/PlatformServices를 직접 재사용했습니다. 원본 Trigger에는 Shift+F10 핸들러가 없으므로 새 바인딩을 추측해 추가하지 않았습니다. OS/browser의 contextmenu 이벤트·접근성 동등성이 통과한 것은 아닙니다.
- 행의 Copy Path/Copy Relative Path/Reveal in Finder를 원본 shortcut label·exact modifier와 연결했습니다. 상대 경로는 root 끝 slash 처리·case-sensitive prefix·root 밖은 원래 absolute path 유지 규칙을 그대로 사용합니다. 문자열 복사에는 filesystem canonicalization을 적용하지 않습니다. root가 없으면 relative copy는 작동하지 않으며 row 없는 blank 메뉴에는 이 항목들을 표시하지 않습니다.
- 파일 이름의 마지막 dot이 첫 글자보다 뒤이고 확장자가 case-insensitive html/htm일 때만 Open in Browser를 표시합니다. `.html` dotfile과 directory는 대상이 아닙니다. 메뉴는 원본 locale key를 사용하며 나머지 file-specific/Open With/compare/history/terminal/find/clipboard 메뉴는 별도 미완료입니다.
- UI Action→typed HostCommand→기존 bounded 비차단 host/TaskSupervisor blocking 작업→typed reply로 연결했습니다. Finder/browser는 runtime의 열린 프로젝트 root/symlink 경계를 통과한 뒤 PlatformServices를 호출합니다. OS command는 기존 NativePlatform의 고정 open executable/argv를 사용합니다. clipboard는 GUI-origin CopyText만 받고 읽기/다른 IPC 노출을 추가하지 않습니다.
- egui-winit의 clipboard.set_text는 오류를 로그에만 남기며 반환하지 않습니다. 원본 copyTextToClipboard의 실패 표시를 보존하려고 이미 설치된 arboard 3.6.1을 direct edge로 재사용했습니다. lazily initialized Clipboard를 host callback의 Mutex에 유지해 request마다 drop하지 않고 Linux clipboard 소유 수명을 보존합니다. 한 host 안의 쓰기는 직렬입니다. 반환 실패는 `common.copyFailed` locale의 native status에 연결하고 성공 메시지는 만들지 않습니다. 원본 toast surface의 전체 재현·실제 clipboard/다른 OS 동시 접근은 미완료입니다.
- 이미 설치된 url 2.5.8도 isolated app direct edge로 재사용했습니다. 새 package/root manifest/root dependency·root MSRV·제품 앱·실기 bundle을 변경하지 않았습니다. native Cargo.lock의 해당 app dependency record만 갱신하고 locked/offline 검사가 성공했습니다.

## 내부 파일 URL 오류

기존 NativePlatform.open_url은 validate_external_url로 모든 URL을 다시 검사했습니다. runtime system_open_in_browser는 root guard 뒤에 file_url을 전달하므로 정상 내부 file://도 거부됐습니다. 또한 기존 shared file_url은 raw Path display를 file:// 뒤에 붙여 파일 이름의 #/?/%를 URL 구분자와 구분하지 않습니다.

외부 URL의 HTTP(S) whitelist는 유지했습니다. native 플랫폼 포트는 기존 trusted runtime의 raw file URL만 absolute filesystem path로 변환한 뒤 공식 Url::from_file_path로 인코딩합니다. UI/외부 요청의 root 검사를 이 helper가 대체하지 않습니다. root/Tauri shared file_url 자체나 그 기존 테스트는 변경하지 않았으며 전체 제품의 URL 처리 완료를 주장하지 않습니다. 상세는 `docs/bug/2026-10-02-native-browser-file-url.md`입니다.

## 실제 검사·실패

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] clipboard/typed host 연결의 초기 lib/bin check exit 0, 1.00초입니다. 후속 URL 변환·메뉴 코드는 아래 최종 strict 검사에 포함됐습니다.
- [x] `cargo test ... --test explorer-system`: 신규 2건 PASS, 0.01초입니다. 첫 함수명에 non-snake-case warning이 있어 한국어/snake-case 이름으로 정정했으며 검사기를 끄지 않았습니다. 이름만 변경한 같은 기능 성공은 재실행하지 않았고 최종 strict로 확인했습니다.
- 첫 검사는 실제 native port가 사용하는 platform_url의 Unicode·공백·#/%/? 인코딩/파일 경로 왕복, absolute 경계·상대 경로 거절, HTTP(S) 유지·javascript/userinfo 거절과 외부 guard의 file:// 거절을 확인합니다. 실제 open process는 호출하지 않았습니다.
- 두 번째 검사는 실제 HostBridge/TaskSupervisor와 mock platform/clipboard를 조립했습니다. 복사 성공→실패→다음 성공의 reply/순서, canonical root 안의 Reveal/browser와 실제 port URL 변환, root 밖/relative/symlink 탈출의 platform 호출 전 거절, platform 반환 실패, host disconnect 뒤 callback owner 회수와 합성 disk 내용 보존을 확인했습니다. mock 성공을 OS clipboard/Finder/browser 실기 성공으로 세지 않습니다.
- `cargo test ... --test explorer 경로메뉴는_복사` 최초 1건 FAIL, 0.04초입니다. 메뉴 클릭·닫힘 직후에 다시 누르는 fixture가 egui previous-frame popup hit 영역을 아직 가지고 있었습니다. 고정 popup/memory의 visible_areas_last_frame/end_pass를 확인하고 다음 빈 페인트 frame을 fixture에 반영했습니다. 제품 클릭 규칙·메뉴 조건이나 assertion을 약화하지 않았습니다.
- [x] 위 관련 메뉴 1건만 PASS, 0.07초입니다. 네 실제 메뉴 클릭 Action·세 exact shortcut/추가 modifier 거절, root trailing slash·outside prefix fallback·primary 없음, uppercase HTML/htm/.html/txt/directory·blank menu를 확인했습니다. 화면 높이는 메뉴 동작 확인용 480px이며 작은 창의 scroll/clip·픽셀/AX 동등성이 통과한 것은 아닙니다.
- [x] 제품 lib/bin/explorer/explorer-system strict clippy exit 0, 1.16초입니다. 후속 fixture의 빈 페인트 변경은 final test target만 strict 검사하고 제품 lib/bin 결과를 재사용합니다.
- [x] 최종 explorer fixture-only strict clippy exit 0, 0.38초입니다. 같은 제품 lib/bin·explorer-system 성공을 재사용했습니다.
- [x] 초기 app fmt --check·해당 QA/bug Prettier·git diff --check exit 0입니다. 아래 Copy adapter 변경의 최종 포맷 검사는 별도로 기록합니다.

## 실제 Copy 이벤트 연결

고정 egui-winit 0.36.2의 `is_copy_command/on_keyboard_input`은 Command+C에 Alt/Shift가 함께 있어도 `Key::C` 대신 `Event::Copy`를 전달합니다. 트리의 Key 처리만으로 경로 복사를 검증하던 검사는 이 실제 adapter 경계를 덮지 못했습니다. native keyboard 순회에 Copy를 연결하고 같은 frame의 ModifiersChanged 순서를 유지했습니다. egui `matches_exact`는 공통 Command 패턴에서 추가 macOS Ctrl을 무시하므로 원본 exact shortcut에 맞춰 Ctrl 일치도 확인합니다.

- 신규 `cargo test ... --test explorer 실제adapter_event`는 첫 실행에서 경로 복사 Action 0건으로 FAIL(0.02초), Copy 연결 뒤 추가 Ctrl 허용으로 FAIL(0.02초)했습니다. 해당 제품 원인을 각각 수정했습니다.
- 실제 macOS modifier fixture를 바꾸는 과정에서 MAC_CMD만 사용해 공통 command=false가 되어 첫 Action 0건으로 FAIL(0.02초)했습니다. 공식 pinned winit의 ModifiersChanged 처리는 mac_cmd와 command를 함께 true로 설정합니다. fixture를 `MAC_CMD | COMMAND`로 교정했으며 제품 조건이나 assertion을 약화하지 않았습니다.
- [x] 수정한 관련 adapter 검사 1건 PASS, 0.02초입니다. 실제 형태의 Copy 두 건/Alt·Shift 순서, 일반 Copy의 경로 복사 방지, 추가 Ctrl 거절, Cmd+Ctrl+Down의 일반 이동과 focus 없는 Copy 비소비를 확인했습니다. OS에서 직접 키를 누른 실기 성공으로 세지 않습니다.
- [x] Copy 처리·exact Ctrl 제품 변경 뒤 lib/bin/explorer strict clippy exit 0, 0.65초입니다. 이후 fixture 플래그만 수정했습니다.
- [x] 최종 fixture-only strict clippy exit 0, 0.40초입니다. 이후 내부 clipboard branch/메뉴 삽입의 관련 2건 PASS(0.08초)와 strict 결과는 explorer-clipboard QA에서 추적합니다.

## 남은 전체 경계

- [ ] 실제 OS clipboard/Finder/browser·Windows/Linux opener, concurrent clipboard 접근/manager·종료 중 실제 OS 호출, locale toast surface·작은 창의 max-height scroll/메뉴 keyboard focus/restore·keyboard context OS 이벤트/VoiceOver·픽셀·icon·모든 메뉴 동등성은 미완료입니다. 현재 macOS 후보 포트의 구현을 다른 OS 지원 완료로 세지 않습니다.
- [ ] 내부 Cut/Copy/Paste·Open to the Side/Open With/compare/history/terminal/find·모든 shortcut/Git/auto reveal·Explorer 전체와 M8 N1~N8·213 view/editor/LSP/terminal·성능/보안/beta/rollback/배포·TS 제거/Rust99%는 미완료입니다.

실제 사용자 파일·clipboard/Trash·OS 입력기/VoiceOver·실기 앱/bundle을 조작하지 않았으며 새로운 package 설치나 commit/push는 하지 않았습니다. 기존 성공/Trash·GUI startup 검사는 반복하지 않았습니다.
