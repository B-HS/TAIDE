# M8 Audio·Video approved host·cache·WebView 코드 연결

## 구현 상태

`preview_web_media.rs`, `preview_web_{host,cache,http,resource}.rs`, `application.rs`, `lib.rs`, `tests/preview-web-media.rs`, `LICENSE-LUCIDE`입니다. 기존 AudioPreview/VideoPreview의 controls·파일 이름·Music·여백·크기를 같은 승인된 HTTP host와 macOS child WebView에 연결했습니다. 실제 codec·OS/GUI/pixel·접근성·전체 provider/M8 완료 증거는 아닙니다. 사용자 실기 bundle·앱·데이터·시스템 설정은 조작하지 않았습니다. workflow·서브에이전트 없이 메인이 직접 수행했습니다.

## 원본과 연결 경계

1. 원본은 `src/features/preview/{audio,video}-preview.tsx`와 `src/shared/styles/global.css`입니다. audio는 중앙 column·16px 간격/여백·40px Music·14px/20px 파일 이름과 448px 최대 폭/말줄임·width 100% controls입니다. video는 중앙·16px 여백·최대 너비/높이 100% controls입니다. autoplay/preload를 새로 지정하지 않습니다. 원본 body의 13px 시스템 font family·antialias·user-select와 overflow hidden을 유지합니다. 실제 WebKit의 UA controls·Tailwind preflight·DPI/fonts 픽셀 정합성은 별도 실기입니다.
2. 파일 이름은 DOM text, src는 검증된 URL attribute, 색상은 typed RGBA의 hex8로 주입합니다. 작은 신뢰 template만 worker에서 파싱하고 기존 prepared HTML/CSP를 사용합니다. wrapper output 16KiB·이름/source 각 4096B 상한이며 untrusted media 파일 본문을 DOM으로 읽지 않습니다. 큰 파일은 기존 열린 regular descriptor/root anchor·stamp·64KiB Range body를 그대로 사용합니다.
3. 실제 Bridge는 Audio/Video 요청을 `media_owner`와 tracked document worker로 전달합니다. HTML만 기존 별도 helper를 사용합니다. media wrapper는 capability 아래 `/__taide_media_document`, 파일은 별도 실제 encoded 절대 경로입니다. 둘이 같은 URL이면 HTML을 media 본문으로 다시 읽게 되므로 namespace를 분리했습니다. 원본 HTML 게시 경로는 유지했습니다. register/게시/각 HTTP read에서 승인·stamp·owner 검사를 재사용합니다.
4. NativeApplication의 reconcile/live/error/surface 경로가 Html/Audio/Video를 모두 포함합니다. macOS renderer의 비활성/닫힘/팝업·focus/bounds/deny/close 경계와 기존 bidirectional cache budget을 재사용합니다. foreground/background/muted는 원본 theme token에서 전달하고 Color32의 premultiplied 값을 unmultiplied로 변환합니다. appearance watch는 다음 worker 요청에 snapshot을 전달하며 theme 변경 시 media만 invalidate하고 active media token을 취소합니다. active HTML은 유지합니다.
5. 현재 theme 변경은 media 문서/child 재생성을 유발합니다. 원본 CSS 갱신처럼 playback position·pause·volume을 유지하는 동등성은 아직 구현/검증되지 않았습니다. JS/parent IPC를 열어 우회하지 않습니다. 실제 앱의 live theme 갱신 자체도 전체 theme 구현 게이트입니다. 기본 연결을 전체 상태 동등성으로 계산하지 않습니다.

Music geometry는 기존 설치 `lucide-react` 1.28.0의 `dist/esm/icons/music.mjs`입니다. 이미 보유한 `LICENSE-LUCIDE`의 ISC에 Music이 Feather-derived임을 명시하고 upstream Cole Bemis MIT 전문을 추가했습니다. 새 dependency·root manifest/lock·제품 TS·MSRV 변경은 없습니다. 최종 배포 notice inclusion은 N7 게이트입니다.

## 실행 증거

Cargo 직렬, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 신규 `cargo test … --lib web_media_document -- --nocapture`: 1 PASS, compile 2.95초/suite 0.00초입니다. filename script literal escaping·controls/autoplay/preload·audio/video structure/style·외부 URL/형식/이름 상한 거절을 검사했습니다. 최초 compile의 E0277은 test의 StrTendril와 &str 비교였고 `text().as_ref()`로 정정했습니다. 실패한 compile은 test body를 실행하지 않았습니다.
- [x] 신규 `cargo test … --test preview-web-media -- --nocapture`: 1 PASS, compile 6.91초/suite 0.03초입니다. 존재하지 않는 absolute helper를 주입한 실제 lazy Bridge에서 SourceReady→Prepared 순서·audio document/encoded filename·HTTP policy/Range·60MiB sparse video HEAD/suffix Range·다른 열린 root 거절·media-only theme invalidation/active HTML 유지·owner/ticket 폐기 HTTP 404·변경 appearance 다음 요청·닫힌 root HTTP 403·CLI exact file/형제 거절·source 변경의 문서/asset 거절·worker/root shutdown/작업 0/port 회수를 연속 확인했습니다. synthetic localhost만 승인 실행했으며 실제 GUI/decoder 재생 검사가 아닙니다.
- [x] `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings`: exit 0(6.30초)입니다. app 선택 target 기준이며 vendor Wry의 기존 deprecated/unused-unsafe 경고 17개는 남습니다. 검사기 suppression은 추가하지 않았습니다.
- [x] 이번 변경 Rust 8개 파일의 exact `rustfmt --edition 2024 --config skip_children=true --check`: exit 0입니다. 동일 상태의 기존 helper/client/resource/HTTP/HTML/cache/HWP 성공은 반복하지 않습니다.

## 잔여 필수 게이트

- [ ] Audio·Video 원본 상태 동등성: theme 변경의 playback 유지·WebKit UA controls·codec/seek/volume/fullscreen·키보드/AX·정확한 geometry/pixel입니다. 위 sparse/synthetic bytes는 실제 media codec 증거가 아닙니다.
- [ ] HTML/media 실제 보안·renderer 실기: 보호 bundle과 다른 합성 앱 한 세션에서 bounds/focus/overlay·navigation/파일 선택/capture 거절·source 변화·종료/crash를 연속 확인합니다. 코드 구현 우선·사용자 담당 IME/VoiceOver 마지막 순서를 유지합니다.
- [ ] Windows/Linux renderer·Windows anchored resource·전체 CPU/RSS/GPU/cache/source graph·장기 session·최종 package/notice·N1~N8입니다. logical document/cache budget은 전체 프로세스 메모리 상한이 아닙니다.
