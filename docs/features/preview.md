# 기능 — 파일 미리보기 (preview)

> Phase 7.5 신규. 텍스트가 아닌 파일을 탭에서 바로 본다.
> 범위 결정 근거: `docs/acknowledge/2026-08-06-phase75-decisions.md` §1.2.
> 대형 파일 정책은 `editor.md` §3 과 동일 임계값을 공유한다.

## 1. 대상과 렌더 방식

별도 `TabKind::Preview` 는 신설하지 않았다 — **기존 `TabKind::File` 탭을 그대로 쓰고, 프론트가
확장자로 렌더러를 고른다**: `pane-node-view.tsx` 가 `resolvePreviewKind(fileName)`
(`shared/lib/preview-kind.ts`)이 non-null 이면 Monaco 대신 `PreviewPane` 을 렌더하고, 사용자는
`openWithOverride`(`entities/editor/open-with-registry`, 탭 메뉴 "Reopen Editor With…")로
editor/preview 를 수동 전환한다. 판정은 `file_open` 의 크기 티어와 무관하게 확장자 기반이다.

| 종류 | 확장자 | 렌더 |
|------|--------|------|
| Image | png jpg jpeg gif webp bmp svg avif | `<img>` (svg 는 sanitize 후) |
| Video | mp4 webm mov m4v | `<video controls>` |
| Audio | mp3 wav flac m4a ogg | `<audio controls>` |
| Pdf | pdf | `pdfjs-dist` |
| Html | html htm | **격리된 iframe**(§4 보안) |
| Spreadsheet | xlsx xls csv | `xlsx`(SheetJS) → 표 컴포넌트 |
| Presentation | pptx | §3 — **가장 취약한 항목** |
| Hwp | hwp hwpx | `@rhwp/core` → SVG |

- **PDF·스프레드시트·프레젠테이션·HWP 뷰어는 `React.lazy` 청크**다(d-51 F7 · 감사 §1-1).
  `pdfjs-dist`·`xlsx`·`@rhwp/core` 는 앱에서 가장 무거운 잎 의존성이라 부팅 번들에서 분리했고,
  해당 확장자의 파일을 실제로 열 때 받아온다(Suspense fallback 은 바이트 대기 화면과 동일한 빈 면).
  이미지·비디오·오디오·HTML 은 네이티브 요소 래퍼라 그대로 eager 다.
- **pdf.js worker 는 첫 `PdfPreview` 렌더에서 스폰**한다(d-51 F7 · 감사 §1-2). 예전에는
  `shared/lib/pdf/setup.ts` 모듈 평가 시점에 `new PdfWorker()` 가 돌아 PDF 를 한 번도 열지 않는
  세션도 워커 스레드를 띄웠다. 지금은 `getPdfjsWithWorker()` 가 최초 호출에서 포트를 만들어
  `GlobalWorkerOptions.workerPort` 에 꽂고, 이후 호출은 같은 포트를 재사용한다.
- 미지원 확장자는 **"미리보기를 지원하지 않습니다 + 외부 앱에서 열기"** 화면
  (`opener` 플러그인의 `openPath`). 억지로 텍스트로 열지 않는다.
- preview 탭도 `tabs.md` §3 의 preview/pin 규칙을 그대로 따른다(단일 클릭 = preview 탭).

## 2. 데이터 경로

- 파일 바이트는 **`file_read_raw(path)` → `ipc::Response`(ArrayBuffer)** 로 받는다
  (`ipc-contract.md` §1 "대용량 단발 = Response"). 이벤트/JSON 경유 금지.
- 프론트는 `Blob` → `URL.createObjectURL` 로 소비하고, **unmount 시 반드시 `revokeObjectURL`**
  (누수 방지 — `architecture.md` §6).
- 크기 상한: `constants.rs` 의 `READ_ONLY_FILE_BYTES`(20MB) 초과는 미리보기 대신 안내 + 외부 열기.
  비디오·오디오는 예외적으로 상한을 두지 않고 **커스텀 프로토콜(`asset:`) 스트리밍**을 쓴다
  (전체를 메모리에 올리면 안 된다).

## 3. 라이브러리 선정과 한계 (실사 확인 — 2026-08-06)

| 용도 | 채택 | 확인된 버전 | 비고 |
|------|------|------------|------|
| PDF | `pdfjs-dist` | 6.2.108 | worker 배선 필요(Monaco 와 동일 패턴) |
| xlsx | `xlsx` (SheetJS) | 0.18.5 | 시트 탭 + 표 렌더. 수식은 계산값만 표시 |
| HWP/HWPX | `@rhwp/core` | 0.8.2 | Rust+WASM, MIT. SVG 렌더 API |
| pptx | **미정 — §3.1** | — | |

### 3.1 pptx 는 신뢰할 만한 렌더러가 없다 (정직한 한계)

조사 결과 **npm 의 `pptxgenjs`(4.0.1)는 생성(generator) 라이브러리이지 렌더러가 아니다.**
pptx 를 브라우저에서 원본 충실도로 그리는 성숙한 오픈소스는 확인하지 못했다(**미확인**).

따라서 pptx 는 다음 순서로 **점진 적용**한다.

1. **1차**: 슬라이드 텍스트·이미지 추출 수준의 **개요 미리보기**(OOXML 을 직접 파싱).
   원본 레이아웃 재현은 하지 않는다고 UI 에 명시한다.
2. **2차**: LibreOffice(`soffice --headless --convert-to pdf`)가 **설치돼 있으면** PDF 로 변환해
   PDF 렌더러로 표시. 없으면 1차로 폴백 + 설치 안내.
   → 외부 바이너리 의존이므로 **번들하지 않고 감지만** 한다(ADR-0010 의 "바이너리 동봉 금지"와 같은 취지).

**이 한계를 UI 에서 숨기지 않는다.** "레이아웃이 원본과 다를 수 있습니다" 안내를 띄운다.

## 4. 보안 (`ipc-contract.md` §4 연장)

- **HTML 미리보기는 신뢰할 수 없는 입력이다.** `sandbox` 속성을 채운 iframe 에서만 렌더하고
  (`allow-scripts` 미부여), 부모 문서와 격리한다. CSP 의 `frame-src` 를 그에 맞게 좁힌다.
- **SVG 도 스크립트를 담을 수 있다** — `<img>` 로 렌더하거나 sanitize 후 인라인한다.
  `dangerouslySetInnerHTML` 로 원본 SVG 를 그대로 넣지 않는다.
- xlsx/hwp 파서는 신뢰할 수 없는 바이너리를 다룬다. **파싱 실패가 앱을 죽이지 않도록**
  에러 경계로 감싸고 실패 시 "열 수 없음 + 외부 앱에서 열기"로 떨어뜨린다.
- 경로는 기존 `ensure_within_root` 검증을 그대로 통과시킨다.

## 5. 수명주기

- preview 위젯 unmount 시: objectURL revoke, pdfjs `document.destroy()`,
  rhwp WASM 인스턴스 해제, 비디오 `src` 해제.
- 탭 전환으로 숨겨질 때 비디오·오디오는 **일시정지**한다(백그라운드 재생 방지).
- 대형 파일 파싱은 **web worker** 로 밀어 UI 를 막지 않는다(xlsx·hwp).

## 6. 범위

| 1차 | 2차 |
|-----|-----|
| Image·Video·Audio·PDF·HTML·xlsx·HWP 미리보기, 미지원 시 외부 열기, 크기 상한 | pptx 개요→LibreOffice 변환, xlsx 수식/서식 재현, HWP 편집, 이미지 확대/비교 |
