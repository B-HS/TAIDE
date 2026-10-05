# M8 공용 Toast provider와 실제 browser 소비

## 대상·보존

`native/taide-native-ui/src/{toast,toast-motion,toast-swipe}.rs`에 원본 Toast와 기존 검사를 동일 이동했습니다. native App은 같은 Toasts를 re-export하고 옛 helper 사본은 삭제했습니다. 코드와 SVG는 공유 위치에 보존되며 Git에서 이전 경로도 복구할 수 있습니다. public 가시성·import·SVG 상대 경로·읽기 전용 inspection·포맷 차이를 제외한 원본 본문과 motion/swipe의 대조는 동일합니다. 본문 대조에서 발견된 차이는 함수 인자의 포맷용 trailing comma뿐이었습니다.

Wasm의 실제 시계는 기존 lock에 있던 `web-time=1.1.0`을 직접 사용합니다. native에서는 std::time::Instant의 re-export이며 Wasm에서는 performance.now를 사용합니다. 원본 수명·일시정지·위치·닫기·swipe·focus·AX·SVG·오류 번역 계약은 변경하지 않았습니다. std Instant로 Wasm을 우회하지 않습니다.

BrowserEditor는 동일 Toasts와 Settings/Theme 오류 큐를 소유합니다. Canvas는 정상 화면과 close 실패 화면 모두에서 Toast를 표시하고 Closing/Failed에서는 원본 disabled 정책을 유지합니다. Workbench가 실제 prefers-reduced-motion MediaQueryList listener를 소유하고 Drop으로 해제합니다. 변경은 기존 단일 pump를 깨우며 interval·새 idle loop를 추가하지 않습니다. inspection은 dev/probe feature에서만 켜집니다. 전체 전역 preview와 다른 surface caller까지 완료한 것은 아닙니다.

## 검증

- [x] 공유 portable 원본 오류 제목 검사1 PASS(build1.33초/.00초): `toast::tests::native_toast는_theme_오류를_settings_제목없이_원본으로_표시한다`.
- [x] 같은 binary의 다른 원본 카드/아이콘/닫기/종료 중 내용 검사1 PASS(.01초): `toast::tests::native_toast는_실제_카드_내용_아이콘_닫기와_종료중_내용을_그린다`.
- [x] 새 실제 Chrome/Wasm `settings-toast` 연속1 첫 PASS: 실제 Create/save 보류→Close Pending→의도 save 거절→Failed/연결 유지/원본 오류 Toast→Chrome reduced-motion 변경→Failed 닫기 비활성→cancel Open→원본 close→제거→unmount→media 복원→Ready/socket0→quiet1.1초 불변.
- [x] 최신 probe build4.24초·공식 wasm-bindgen 생성·production Wasm canvas strict1.05초·TS strict/Prettier·Rust format exit0. native lib/bins/tests 최종 strict5.37초 exit0이며 기존 Wry17 외 새 경고는 없습니다. 테스트 전용 shim/root module cfg와 optional nondefault inspection forwarding으로 실제 미사용 경고 원인을 정리했습니다.

실측은 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-toast-result.json`과 `settings-toast.png`입니다. seq12/save1(의도 거절)/delete0·Failed frame31/pump30→최종frame62/pump55·upgraded1/active0·추가 요청/상태/연결0·page/panic/consumer 누출0입니다. 의도 Theme 오류1은 숨기지 않고 상태와 실제 붉은 Toast의 `Synthetic theme save refusal` 제목·오류 SVG·닫기로 확인했습니다. close 실패 화면 자체는 probe 표시이며 전체 제품 close overlay 완성을 주장하지 않습니다.

원본 Tooltip/theme 성공 경계는 재실행하지 않았습니다. OS 설정·사용자 데이터·보호 앱·제품TS·기존 의존성 버전/MSRV·Git은 유지합니다. Chrome media emulation만 변경 후 복원했습니다.

## 남은 범위

Settings UI의 preference/폴더/settings.json 명령·live preview 전체 application 적용·다른 surface/tab caller·원본 layout 문제·제품 bundle·최종 gate는 미완료입니다. provider2/4(50%)·전체363/433(83.83%)·최종0/8·전체 ETA 산정 보류·goal active·main 직접입니다.
