# Native 전환 감사: 미리보기 영역 (이미지·SVG·미디어·PDF·HTML·스프레드시트·PPTX·HWP)

작성일: 2026-10-06. 읽기 전용 감사이며 빌드·테스트·실행은 하지 않았습니다. 이전 에이전트의 HANDOFF.md/PROCESS.md 수치는 사용하지 않고 실제 소스만으로 판정했습니다.

## 1. 범위

TS 기준 (전부 읽음)
- `/Users/hyunseokbyun/development/TAIDE/src/widgets/preview-pane/preview-pane.tsx`
- `/Users/hyunseokbyun/development/TAIDE/src/features/preview/{image,video,audio,html,pdf,spreadsheet,presentation,hwp,unsupported,preview-status}-preview.tsx`
- 지원 로직: `src/shared/lib/{preview-kind,html-preview-document,spreadsheet,pptx-outline}.ts`
- 연관 진입점: `src/widgets/editor-area/pane-node-view.tsx`(형식 분기), `pane-tab-bar.tsx`·`src/features/tab/tab-context-menu.tsx`(다시 열기), `src/features/explorer/file-tree-context-menu.tsx`(연결 프로그램·브라우저에서 열기), `src/entities/layout/tab-path-change.ts`(캐시 무효화)
- inventory: `2026-09-28-ts-view-inventory.md` 46행(preview 한 줄, 형식별 직접 검사 미완료로 기재)

native 대조 (전부 읽거나 호출 체인 확인)
- `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/preview.rs` 및 `preview_*.rs` 전체 목록(37개 파일, 약 1.16만 줄). 핵심 파일은 직접 읽음: preview.rs, preview_pdf.rs, preview_pdf_macos.rs, preview_pdf_surface.rs, preview_hwp.rs, preview_hwp_surface.rs, preview_status.rs, preview_svg.rs, preview_spreadsheet.rs, preview_spreadsheet_surface.rs, preview_presentation.rs(부분), preview_presentation_surface.rs, preview_web.rs, preview_web_view.rs, preview_web_document.rs, preview_web_media.rs, preview_web_helper.rs, preview_web_http.rs(핵심부)
- 연결 확인: `src/main.rs`, `src/application.rs`(필드 112-168, 로드 디스패치 3981-4034, 렌더 4924-5091, 정리 3170-3237, web view sync 4335-4362, connect_web 5402), `src/host.rs`(명령 처리), `src/open_with.rs`, `src/explorer.rs`
- vendor 디렉터리 5개의 UPSTREAM.md/Cargo 수준 확인
- `tests/` 아래 preview-*.rs 통합 테스트 목록 확인 (내용은 읽지 않음)

## 2. 구조 요약 (native)

- 진입점: `main.rs` -> `NativeApplication`(eframe/egui). 파일 탭 렌더 중 `open_with.surface(path)`가 `Surface::Preview(kind)`이면 application.rs 4924행 이후에서 형식별 surface를 호출합니다. 형식 목록과 확장자 매핑(open_with.rs:32-50)은 TS `preview-kind.ts`와 정확히 같습니다.
- 로드는 `load_*_previews` 큐 -> `HostCommand::Read*Preview` -> `taide_file::service::read_raw`(20 MiB 상한 `READ_ONLY_FILE_BYTES`, TS 백엔드의 `file_read_raw`와 같은 공유 crate 함수이므로 동등) -> 결과가 application.rs 700-815행에서 각 Cache.accept 로 들어옵니다.
- 렌더 방식은 세 갈래입니다.
  1. egui 직접 렌더: 이미지(래스터·SVG), PDF(텍스처), HWP(텍스처), 스프레드시트, PPTX, 상태 화면.
  2. OS 프레임워크: PDF는 macOS CoreGraphics(CGPDF), AVIF·ICC 이미지는 macOS ImageIO.
  3. WebView: HTML·오디오·비디오는 vendored wry(WKWebView)를 egui 창의 자식 NSView로 얹어 표시합니다.
- vendor 디렉터리가 vendoring된 이유 (각 UPSTREAM.md 근거)
  - `wry-preview`: wry 0.55.1 사본. 기본 비활성 feature `native-preview-deny-permissions`로 macOS WKUIDelegate의 파일 업로드 패널을 거절(nil completion)하고 카메라/마이크 권한을 Deny로 고정합니다. HTML/미디어 미리보기 WebView 전용.
  - `rhwp`: rhwp v0.8.2 사본(TS가 쓰는 WASM 엔진과 같은 버전). 국소 패치(overflow·slice panic 수정, HWP3 limit, `native-retained` 메모리 계측 derive)를 적용해 native 라이브러리로 직접 링크합니다. TS의 `@rhwp/core` WASM 대응물입니다.
  - `cfb-retained`, `zip-retained`: cfb 0.14.0, zip 8.6.0 사본. rhwp의 HWP5(CFB)/HWPX(ZIP) lazy resolver 메모리를 정확히 계측하기 위한 조건부 derive만 추가했습니다. 파서 동작은 변경하지 않았습니다.
  - `egui-input`(참고): egui 입력 패치. 미리보기와 무관.

## 3. 기능 대응표

상태 표기: done / partial / unwired / missing / n/a. effort는 native에 남은 작업량 추정입니다.

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|------|---------|-------------|-------------|---------|--------|
| 1 | 확장자 -> 미리보기 형식 결정(이미지·비디오·오디오·pdf·html·sheet·pptx·hwp) | preview-kind.ts:5-32 | done | open_with.rs:32-50, 호출 application.rs:4924 | 없음 (확장자 목록 동일) | - |
| 2 | 탐색기 "연결 프로그램: 에디터/미리보기" | file-tree-context-menu.tsx:91-122 | done | explorer.rs:986-1017, application.rs:4704-4711, open_with.rs Registry | 없음 | - |
| 3 | 탭 우클릭 "다른 편집기로 다시 열기: 에디터/미리보기" | tab-context-menu.tsx:59-60,153-157, pane-tab-bar.tsx:184,228 | missing | 검색어 `reopenWith`, `reopenEditorWith`, `ReopenWith`, `tab.reopenEditorWith`, `explorer.openWithEditor`를 native 전체(app/ui/model)에서 검색: explorer.rs 외 없음. 로케일 키 `tab.reopenEditorWith`(ko.json:1006)는 있으나 호출부 없음. native-ui shell.rs에는 탭 컨텍스트 메뉴 자체가 없음(shell.rs:823,949는 pinned 표시) | 탭 컨텍스트 메뉴 전체(다시 열기 서브메뉴 포함). 이 항목은 탭 영역 감사와 겹침 | M |
| 4 | 탐색기 "브라우저에서 열기"(html) | file-tree-context-menu.tsx:93 | done | explorer.rs:1032-1041, host.rs:1076-1090 | 없음 | - |
| 5 | 외부 저장·이동·삭제 시 미리보기 갱신/무효화 | tab-path-change.ts(FILE.RAW 캐시 무효화) | done | application.rs:3170-3197 invalidate/invalidate_all/invalidate_root, 3219-3236 reconcile, 3492-3496 reset_pending | 없음(탭 이름변경 시 open-with 승계는 확인 못함, 불확실성 참조) | - |
| 6 | 외부 앱으로 열기 버튼 + UnsupportedPreview(읽기 실패) | unsupported-preview.tsx, preview-pane.tsx:59,64,85 | done | preview_status.rs:168-263(show), 연결 application.rs:4944,4964,4984,5000,5044 -> host.rs:1028 OpenPath -> system_open_path | 이미지 읽기·디코드 실패는 이 화면을 쓰지 않음(행 13) | - |
| 7 | 정적 이미지 png/jpg/jpeg/bmp/webp | image-preview.tsx | done | preview.rs:93-135 decode, 3981 load_previews, 5075-5082 show_image | 없음 | - |
| 8 | 애니메이션 gif/apng/webp | `<img>` 브라우저 재생 | done | preview_animation.rs(루프 횟수·최소 지연 보정·4096 프레임 상한), preview.rs:301-319,366-393 advance, request_repaint_after | 프레임 4096개·64MiB 초과 애니메이션은 오류(브라우저는 상한 없음) | S |
| 9 | AVIF | resolvePreviewKind avif | partial | preview.rs:100-103 macOS에서만 ImageIO(preview_macos.rs). 비macOS는 `native raster decoding does not support this format yet`(preview.rs:80) | Windows/Linux AVIF 디코더(예: 순수 Rust 디코더 도입) | M |
| 10 | EXIF 회전 / ICC 프로파일 색 | 브라우저가 자동 처리 | partial | EXIF: preview.rs:117-122 apply_orientation 완료. ICC: preview.rs:108-116 macOS ImageIO로만 처리 | 비macOS ICC 색변환 없음(sRGB 가정으로 색이 달라질 수 있음) | M |
| 11 | SVG(래스터 이미지와 동일 화면) | image-preview.tsx(img svg+xml) | partial | preview_svg.rs(resvg+system fonts, 중첩 이미지·깊이·픽셀 예산), preview.rs:97-99 비래스터 포맷 fallback | 벡터를 고유 크기 1x 텍스처로 래스터화해 표시하므로 HiDPI(레티나)에서 흐림, 확대 불가. viewBox만 있는 SVG 크기 해석이 브라우저(300x150 기본)와 같은지 미확인. 외부 href는 의도적으로 해석 안 함(TS도 img라서 동일) | M |
| 12 | 이미지 배치(contain, 중앙, 스크롤, 패딩 16) | image-preview.tsx:9-12 | done | preview.rs:403-425 show_image(scale min 1.0, ScrollArea::both, centered) | 없음 | - |
| 13 | 이미지 읽기/디코드 오류 화면 | preview-pane.tsx:64 UnsupportedPreview | partial | application.rs:5055-5074는 기본 egui label 3개(안내문, 제목, 원시 오류 문자열)와 기본 버튼. preview_status::show(아이콘·중앙 정렬·테마 색)를 쓰지 않음 | 아이콘/레이아웃/테마 색, 원시 영어 오류 문자열 노출 제거. 다른 형식은 preview_status 사용 중이라 불일치 | S |
| 14 | 비디오 mp4/webm/mov/m4v 재생(컨트롤·seek) | video-preview.tsx(`<video controls>`) | partial | WKWebView: preview_web_media.rs:20 템플릿 -> preview_web_http.rs(Range 지원, preview_web_range.rs) -> preview_web_view.rs:127-152. application.rs:5013-5027 placement | 재생 UI가 egui가 아니라 WebKit 컨트롤. macOS 전용(비macOS는 `isolated preview renderer is not implemented on this platform`, preview_web_view.rs:219-234). 코덱은 OS WebKit 의존 | XL (진짜 egui 컨트롤+자체 디코더 시) |
| 15 | 오디오 mp3/wav/flac/m4a/ogg(아이콘·파일명·컨트롤) | audio-preview.tsx | partial | preview_web_media.rs:21 AUDIO_TEMPLATE(아이콘·파일명·audio controls)를 같은 WebView로 표시 | 14와 동일. 오디오 디코더·출력 crate가 native Cargo.toml에 없음(검색어 symphonia, rodio, cpal, ffmpeg, AVFoundation 모두 0건) | XL |
| 16 | 미디어 로컬 파일 스트리밍·Range 요청·소유권 해제 | convertFileSrc(asset protocol) | done | preview_web_http.rs:378-516(Host 검증, GET/HEAD만, Range, 청크 스트림), preview_web_resource.rs:249-271 MIME, preview_web_cache.rs invalidate_media, application.rs:3636 | 없음(재생은 14, 15) | - |
| 17 | PDF 페이지 렌더 | pdf-preview.tsx(pdfjs canvas) | partial | preview_pdf.rs:75-91 decode, preview_pdf_macos.rs(CGPDFDocument 렌더, UserUnit·회전·CropBox 처리), preview_pdf_surface.rs:255-326, 로드 application.rs:3989 | macOS 전용: 비macOS는 `native PDF decoding is not connected on this platform`(preview_pdf.rs:87)으로 항상 실패 화면. 텍스트 선택·링크는 TS에도 없음 | L (비macOS 렌더러 도입 시) |
| 18 | PDF 이전/다음 페이지, N/M 표시, 비활성 경계 | pdf-preview.tsx:37-38,141-170 | done | preview_pdf_surface.rs:109-221, preview_pdf.rs:260-275 change | 없음 | - |
| 19 | PDF 확대/축소 50%~300%, 25% 단위, 퍼센트 표시 | pdf-preview.tsx:19-22,39-40 | done | preview_pdf.rs:17-20(MIN_ZOOM 2, MAX 12, DIVISOR 4, 초기 4 = 0.5~3.0, 100%) | 큰 페이지의 300%는 `max_side`(텍스처 한도) 초과 시 오류(preview_pdf_macos.rs:94-102). TS canvas는 이런 오류 없음 | S |
| 20 | PDF 로딩 스피너·오류(`preview.pdf.loadFailed`)·외부 열기·데이터 교체 시 재로딩 | pdf-preview.tsx:55-58,124-137 | done | preview_pdf_surface.rs:293-315, preview_status.rs, preview_pdf.rs:210-230 invalidate | 소스 확보 전(`has_source` false) 빈 화면(TS는 즉시 스피너). 사소 | S |
| 21 | HTML 렌더(스크립트 차단 sandbox) | html-preview.tsx(iframe sandbox allow-same-origin + CSP script-src none) | partial | preview_web_view.rs:127-152(JS 비활성, incognito, devtools off, clipboard off, 새 창 거부, 다운로드 거부), preview_web_document.rs:16 CSP, 자식 프로세스 sanitize preview_web_client.rs:65 + main.rs:11-24 | WKWebView(WebKit 엔진) 의존, macOS 전용. egui 위에 겹친 자식 NSView라 egui 오버레이와 z-order 충돌(7절). 비macOS 미구현 | XL (엔진 대체 시) / M (플랫폼 확장 시) |
| 22 | HTML 상대 경로 자산(`<base>` 재작성, 같은 폴더 CSS/이미지) | html-preview-document.ts:1-25 | done | preview_web_document.rs:73-110 prepare_html(기존 base 승계, 단일 base, 선두 CSP meta), preview_web_http.rs 로컬 루프백 서버 + preview_web_resource.rs 프로젝트 루트 소유권 검사 | 없음 | - |
| 23 | HTML의 외부(https) 이미지·스타일 로딩 | CSP가 script/object/frame/form만 차단, 외부 자원은 허용 | partial | preview_web_document.rs:18 RESOURCE_POLICY(`default-src 'none'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; ...`)가 preview_web_http.rs:361 모든 응답 헤더에 적용 | TS보다 엄격: 외부 http(s) 이미지·CSS·폰트가 native에서는 차단됨. 의도적 보안 강화로 보이나 TS와 동작이 다름 | 결정 필요(S) |
| 24 | HTML 내 링크 클릭 | iframe 안에서 이동(자원 URL로 이동, 스크립트는 차단) | partial | preview_web_view.rs:28-38,144-146 같은 문서 fragment 외 내비게이션 전부 거부 | 문서 간 상대 링크 이동 불가(클릭해도 무반응) | S |
| 25 | XLSX 파싱·표시 | spreadsheet.ts(xlsx 라이브러리), spreadsheet-preview.tsx | done | preview_spreadsheet.rs:220-319 decode_xlsx(calamine), preflight(ZIP 항목·DTD·shared string 예산), surface 125-373 | 없음 | - |
| 26 | XLS(BIFF)·XLML·HTML-as-xls 파싱 | xlsx 라이브러리가 내용 판별 | done | preview_spreadsheet.rs:49-67 시그니처 분기 -> preview_spreadsheet_xls.rs, _biff.rs, _xlml.rs:468-471 -> _html.rs | 없음 | - |
| 27 | CSV 파싱(SheetJS 동등 유형 추론: 숫자·날짜·퍼센트·구분자 추정) | spreadsheet.ts raw:true | done | preview_spreadsheet_csv.rs(numeric/fuzzy_number/fuzzy_date, 구분자 후보 `| ; \t ,`, UTF-16 처리) | SheetJS와의 완전 동등은 테스트(tests/preview-spreadsheet-csv.rs)에 의존, 미실행 | - |
| 28 | 시트 탭 전환(tablist, 가로 스크롤) | spreadsheet-preview.tsx:51-72 | done | preview_spreadsheet_surface.rs:162-244 | 없음 | - |
| 29 | 500행 제한 및 truncatedNotice | spreadsheet.ts:15, preview.tsx:74-78 | done | preview_spreadsheet.rs:11 MAX_PREVIEW_ROWS 500, surface 251-279 | 없음 | - |
| 30 | 빈 시트·시트 없음·로드 실패 + 외부 열기 | spreadsheet-preview.tsx:33-46,81-83 | done | preview_spreadsheet_surface.rs:143-160,280-283 | 없음 | - |
| 31 | XLSX의 ISO 날짜/기간 셀 | xlsx 라이브러리는 숫자로 환원 | partial | preview_spreadsheet.rs:196-207 `DataRef::DateTimeIso`/`DurationIso`/비유한 Float는 `native spreadsheet cell type is not connected yet` 오류로 워크북 전체 실패 | 해당 셀 타입이 하나라도 있으면 파일 전체 미리보기 실패. 문자열 또는 Null 로 degrade 필요 | S |
| 32 | PPTX 슬라이드 텍스트 추출(ZIP·XML 엔티티) | pptx-outline.ts | done | preview_presentation.rs:77-130 entities, 파싱·예산(64MiB, 4096 슬라이드), read 25-45 | 없음 | - |
| 33 | PPTX 슬라이드 목록·선택·본문 문단 | presentation-preview.tsx:66-90 | done | preview_presentation_surface.rs:96-287 | 없음 | - |
| 34 | PPTX 레이아웃 경고 배너·텍스트 없음·로딩·오류 | presentation-preview.tsx:42-64,89 | done | preview_presentation_surface.rs:40-94,114-136,247-264 | 소스 확보 전 빈 화면(사소) | - |
| 35 | HWP/HWPX 페이지 SVG 렌더 | hwp-preview.tsx:60-107(@rhwp/core WASM) | partial | preview_hwp.rs:65-76 `render_page_svg_legacy_native` -> preview_svg::decode 래스터화, preview_hwp_preflight.rs(HWP5/HWPX/HML/HWP3 예산 사전 검증), vendor/rhwp 직접 링크, 로드 application.rs:3997 | TS는 브라우저 canvas `measureTextWidth`로 글자폭 측정(hwp-preview.tsx:23-34). native는 rhwp native 측정 경로이며 vendor/rhwp/UPSTREAM.md가 "native font 측정·큰 문서 호환·원본 전체 렌더 동등성은 M8 gate에 남음"이라고 명시. 실제 폰트 폭 동등성 미검증. 래스터 실패 시 빈 페이지(TS와 동일 의미) | M (검증 포함) |
| 36 | HWP 페이지 이전/다음, N/M 표시 | hwp-preview.tsx:57-58,130-160 | done | preview_hwp_surface.rs:90-196 | 없음 | - |
| 37 | HWP 로딩·오류·페이지 없음 | hwp-preview.tsx:109-126 | done | preview_hwp_surface.rs:116-142 (loadFailed, noPages) | 없음 | - |
| 38 | 비macOS(Windows/Linux) 지원 | Tauri는 3 OS 웹뷰 | missing | PDF(preview_pdf.rs:84-89), HTML/오디오/비디오(preview_web_view.rs:219-234), AVIF·ICC 모두 macOS cfg. 검색어 `target_os = "windows"`, `target_os = "linux"`, `pdfium`, `mupdf`, `hayro`, `lopdf` 에서 preview 관련 구현 0건 | 비macOS에서는 PDF·HTML·미디어·AVIF가 항상 실패 화면 | XL |
| 39 | 원격 브라우저(Wasm) 클라이언트의 파일 미리보기 | PreviewPane은 로컬 앱 전용 | missing | `/native/taide-remote-web/src`에서 검색어 preview, pdf, png, image/, spreadsheet, hwp, PreviewKind, open_with 중 파일 미리보기 구현 없음(preview 문자열은 탭 preview 플래그와 폰트 미리보기뿐: shell.rs:53, app-file-opens.rs:37) | 원격 접속 시 이미지/PDF 등 탭이 미리보기 없이 열림. 원격 영역 감사와 겹치며 TS에 대응 기능이 있었는지 불명확 | 판단 필요 |
| 40 | 형식별 lazy chunk 분리(Suspense) | preview-pane.tsx:30-42 | n/a | Vite 번들 분할 전용. native는 단일 바이너리이고 로드 지연은 비동기 큐(load_*_previews)가 담당 | - | - |

집계: 총 40행. done 24, partial 12, unwired 0, missing 3, n/a 1.

## 4. 잘못 구현됐거나 보강이 필요한 코드

1. 이미지 오류 화면이 형식 간 불일치 (application.rs:5055-5074): 이미지만 `ui.label` 3개와 원시 오류 문자열(영어 내부 메시지)을 그대로 표시합니다. PDF/HWP/시트/PPTX/web은 `preview_status::show`를 쓰므로 TS `UnsupportedPreview`(아이콘·파일명·외부 열기 버튼)에 맞춰 같은 함수로 통일해야 합니다.
2. XLSX 셀 타입 하드 실패 (preview_spreadsheet.rs:205): `DateTimeIso`, `DurationIso`, 비유한 Float 하나 때문에 워크북 전체가 `Failure::Decode`로 끝납니다. 셀 단위 degrade가 맞습니다.
3. 네이티브 WebView 오버레이의 z-order 취약성 (preview_web_view.rs, application.rs:4345-4350): WKWebView는 egui 위에 얹힌 별도 NSView라 egui가 그리는 모든 것 위에 올라옵니다. `web_enabled`는 모달·`Popup::is_any_open`·드래그만 감지합니다. 토스트(toasts.show는 4365행, web_enabled 조건에 없음)나 툴팁이 HTML/미디어 영역 위에 뜰 때 가려질 수 있습니다. 실행 검증은 하지 못했습니다(구조적 추정, 불확실성 참조).
4. HTML 미리보기가 TS보다 엄격한 CSP를 적용 (preview_web_document.rs:18, preview_web_http.rs:361): 외부 http(s) 이미지·CSS·폰트가 차단되어 TS에서 정상 렌더되던 문서가 다르게 보일 수 있습니다. 의도적 보안 결정이면 docs/acknowledge에 기록이 필요합니다.
5. HTML 링크 이동 전면 거부 (preview_web_view.rs:28-38): 같은 파일의 fragment 외 이동이 전부 막혀 상대 링크가 무반응입니다. TS iframe은 이동했습니다(스크립트 없이).
6. SVG를 1x 래스터로 표시 (preview_svg.rs:92-99, preview.rs:403-425): 레티나에서 작은 SVG가 흐리고 확대가 안 됩니다. 표시 크기(ui 크기 x pixels_per_point)로 재래스터화하는 경로가 없습니다.
7. 도달 불가한 방어 분기 (application.rs:5050-5053): 모든 PreviewKind가 앞에서 처리되므로 `native preview provider is not connected` 분기는 죽은 코드이며 영어 하드코딩 문자열입니다. 요청 범위 밖이면 보고만 합니다.
8. PDF 큰 페이지 고배율 실패 (preview_pdf_macos.rs:94-102): 페이지 x 줌이 텍스처 한도를 넘으면 loadFailed 화면이 됩니다. 한도 내로 줌을 자동 clamp 하거나 별도 메시지가 필요합니다.
9. 외부 크레이트 경로가 experiments 디렉터리를 가리킴 (taide-native-app/Cargo.toml:54,108): `eframe`, `vte` 패치가 `experiments/native-shell-spike/vendor/eframe`, `experiments/terminal-core-spike/vendor/vte`를 path 로 참조합니다. 제품 코드가 실험(spike) 디렉터리에 빌드 의존합니다. 미리보기 전용 결함은 아니나 전환 완료 시 정리 대상입니다.

## 5. 실제 앱 연결이 끊긴 지점

- 미리보기 형식 자체에서는 unwired(테스트에서만 쓰이는 심볼)를 찾지 못했습니다. 이미지·PDF·HWP·PPTX·시트·web 6개 Cache가 모두 application.rs 필드(112-168), 로드(3981-4034), 결과 수락(465-483, 700-815), 렌더(4924-5084), 정리(3170-3237)까지 호출 체인으로 연결됩니다. `preview-tooltip-tests.rs`와 `tests/preview-*.rs`는 연결된 코드의 테스트입니다.
- 끊긴 곳은 기능 단위입니다.
  1. 탭 컨텍스트 메뉴의 "다시 열기" 진입점이 없음(행 3).
  2. 비macOS 경로 전부 실패 화면(행 38).
  3. 브라우저(Wasm) 원격 클라이언트에는 미리보기 없음(행 39).

## 6. "UI까지 Rust-native" 목표와 충돌하는 지점

| 영역 | 현재 의존 | 격리 방식 | 충돌 내용 |
|------|-----------|-----------|-----------|
| HTML | WKWebView(WebKit) | JS 비활성, incognito, 단일 문서 내비게이션만 허용, 파일 업로드·카메라·마이크 거부(vendor 패치), 로컬 루프백 HTTP 서버(capability 토큰 경로, Host·메서드 검증, CSP 헤더), HTML 파싱·정제는 별도 자식 프로세스(`--html-preview-helper`, 타임아웃·kill) | HTML/CSS 렌더링 엔진이 WebKit. 순수 Rust 대체는 현실적 후보가 제한적(XL). TS(JS) 의존은 제거됐으나 웹 엔진 의존은 남음 |
| 오디오·비디오 | 같은 WKWebView의 HTML 컨트롤 | 위와 동일 + Range 스트리밍 | 재생 UI와 디코더가 WebKit. egui 컨트롤+자체 디코더로 바꾸려면 오디오 출력·디코드 crate 도입 필요(현재 0) |
| PDF | macOS CoreGraphics(CGPDF) | 래스터를 egui 텍스처로 변환(WebView 아님) | WebView는 아니나 OS 프레임워크 종속. 비macOS 불가 |
| AVIF·ICC | macOS ImageIO | 래스터 변환 | 동일 |
| 공통 UI 위젯(툴바·시트 탭·슬라이드 목록·상태 화면) | egui | - | 충돌 없음 |

결론: 미리보기 UI의 크롬(툴바, 페이지·줌, 시트 탭, 슬라이드 목록, 오류 화면)은 egui로 이미 native입니다. 콘텐츠 렌더링 중 HTML·오디오·비디오만 WebKit 의존이며, 이는 TS 시절(Tauri WKWebView)과 동일한 엔진이지만 "UI까지 Rust-native" 기준으로는 예외 지점입니다.

## 7. 권장 구현 순서 (의존 관계 포함)

1. 이미지 오류 화면을 preview_status로 통일 (행 13). 독립, S.
2. XLSX 셀 타입 degrade (행 31). 독립, S.
3. HTML 정책 결정(외부 자원 허용 여부, 링크 이동)을 사용자와 확정한 뒤 반영 (행 23, 24). 결정이 선행.
4. 탭 컨텍스트 메뉴 "다시 열기" 구현 (행 3). 탭 영역 감사의 탭 메뉴 작업과 같이 진행해야 중복이 없음. open_with.rs Registry는 준비돼 있음(set/surface).
5. SVG를 표시 크기 기준으로 재래스터화 (행 11). preview.rs `Cache.advance`/`show_image`와 preview_svg.rs 경계 변경 필요, M.
6. WebView 오버레이 가림 문제를 실제 실행으로 검증 후 필요 시 토스트·툴팁 시 web_enabled 반영 (7절 3번). 실행 검증이 선행.
7. HWP 글자폭 측정 동등성 검증(행 35): TS 대비 샘플 문서 렌더 비교. 필요 시 폰트 측정 보강.
8. 플랫폼 확장(행 9, 10, 17, 21, 14, 15, 38): macOS 외 대응 여부를 먼저 결정(결정 필요). 결정이 "macOS 전용"이면 docs/acknowledge에 기록하고 종료, 아니면 PDF 렌더러 -> AVIF/ICC -> HTML/미디어 엔진 순으로 XL 작업.
9. 원격 브라우저 미리보기(행 39): 원격 영역 감사 결과와 맞물려 필요 여부 결정.

## 8. 확인하지 못한 것 (불확실성)

- 빌드·테스트·앱 실행을 하지 않았습니다. 모든 판정은 소스 읽기와 grep 호출 체인만 근거입니다. `tests/preview-*.rs` 55개 중 파일 목록만 확인했고 통과 여부는 모릅니다.
- HWP 렌더 폰트 폭·페이지 나눔의 TS(WASM) 대비 동등성은 확인하지 못했습니다(행 35).
- WKWebView 자식 뷰와 egui 토스트·툴팁·일반 컨텍스트 메뉴의 실제 가림 여부는 실행 없이 확인 불가입니다. `egui::Popup::is_any_open`이 툴팁까지 포함하는지는 egui 버전 소스를 확인하지 않았습니다.
- 탭 이름변경·이동 시 open-with 설정 승계(TS tab-path-change.ts:257-263 대응)는 `open_with.rs`의 `observed` 구조가 담당하는 것으로 보이나 끝까지 읽지 않았습니다.
- missing 판정(행 3, 38, 39)에 사용한 검색어를 위 표에 적었습니다. 행 3: reopenWith, reopenEditorWith, ReopenWith, tab.reopenEditorWith, explorer.openWithEditor. 행 38: target_os windows/linux, pdfium, mupdf, hayro, lopdf. 행 39: preview, pdf, png, image/, spreadsheet, hwp, PreviewKind, open_with(remote-web 범위). 단, 탭 컨텍스트 메뉴가 `src` 외 경로(예: 다른 crate의 UI 정의)에 있을 가능성은 `native-app/src`, `native-ui/src`, `taide-model/src` 검색 범위 밖이면 놓쳤을 수 있습니다.
- 스프레드시트 CSV 추론의 SheetJS 완전 동등성, 비정상 파일의 예산 오류 경로는 테스트 부재 여부를 확인하지 않았습니다.
- 이미지 형식 목록에는 TS에 없는 ico·tiff 등이 native에서 어떻게 보이는지(`.ico`는 확장자 매핑에 없으므로 에디터로 열림)는 형식 매핑이 동일하므로 영향 없다고 판단했으나 실제 열기 경로는 보지 않았습니다.
