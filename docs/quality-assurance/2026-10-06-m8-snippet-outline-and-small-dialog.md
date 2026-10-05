# M8 Snippets outline 그림자·Close focus·작은 폐기 Dialog

## 대상·원본

대상은 공용 `snippet-editor.rs`와 실제 `tests/snippet-editor.rs`입니다. [반응형 QA](2026-10-06-m8-snippet-responsive-layout.md)의 후속이며 기존 Snippets UI 범위입니다. `button.tsx`, `dialog.tsx`, `alert-dialog.tsx`, 설치된 Tailwind theme.css·egui0.36.2의 Shadow/Ui/Button/Atom 원천을 대조했습니다.

Close 원본은 `focus:ring-2 focus:ring-ring focus:ring-offset-2`이고 일반 Button은 `focus-visible`입니다. [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:focus-visible)·[WICG 원천](https://github.com/WICG/focus-visible/blob/main/src/focus-visible.js)을 확인했지만 일반 Button의 modality 판단을 이번에 구현·검증한 것으로 표기하지 않습니다.

## 구현

1. 원본 outline shadow-xs인 offset0/1px·blur2px·black5%(8bit alpha13)를 버튼 배경보다 앞선 paint-list 슬롯에 그립니다. 실제 UI ID/입력 노드를 추가하지 않으며 Cancel/Delete/New/Add에 같은 경계를 사용합니다. disabled 쓰기 중 shadow도50%로 처리합니다.
2. Close가 실제 focus를 가지면 원본 background offset2px과 focus border 색상의 바깥2px 링을 그립니다. 일반 Button에 임의의 항상-focus 링을 적용하지 않았습니다.
3. size=sm Alert의320px cap은 viewport 너비로 제한하되 기본 Dialog의32px margin 제한을 섞지 않습니다. 실제300px 화면에서 폐기 Dialog도300px입니다. CSS grid 셀을 넘치는 자연 글자 폭이 content 폭을 늘리지 않도록 버튼의 text atom에 명시된 셀 폭을 사용합니다. 글자를 잘라 임의의 대체 문구로 만들지 않았습니다. 원본 whitespace-nowrap를 Extend 모드로 연결했습니다.

## 검증·실패

- [x] 신규 실제 Settings 연속1 RED→GREEN입니다. 최초 outline shadow가 없어 실패(build.60초/suite.11초)했습니다. 그림자/Close focus/작은 viewport cap 수정 뒤 두 도형 assertion은 통과했지만300px 폐기 Dialog가309.5px로 넘쳐 실패했습니다(build1.20초/suite.14초). 이는 grid 셀보다 버튼의 자연 글자 폭이 컸기 때문입니다. fixed text atom으로 수정한 뒤1PASS/filtered7(build1.28초/suite.14초)입니다.
- [x] 최종 성공은 outline shadow 도형의 blur/색/offset, 합성 RawInput Shift-Tab으로 Close focus를 얻은 링의 stroke/color/offset·Outside, dirty 초안의 실제 Back 버튼으로 열린300px 폐기 Dialog 너비입니다. 생산 focus 강제 설정/클릭 우회가 없으며 fixture의 단조 frame time을 사용합니다. OS 물리 입력 검사가 아니고 성공 뒤 같은 검사를 반복하지 않았습니다.
- [x] 후속 명시 nowrap 소스의 Canvas Wasm check.56초 exit0·경고0입니다. 성공한 geometry 결과를 재사용하며 nowrap의 모든 locale glyph/픽셀을 별도 통과로 주장하지 않습니다. Rust2fmt/문서 포맷·diff는 exit0입니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_close의_focus링 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 남은 범위

일반 Button focus-visible·정확한 primary/ghost padding·색상 전환, global input line-height/설명 AX 연결·글꼴 semibold/전체 AX bounds/theme/DPI/픽셀과 포커스 그래프는 남습니다. Chrome Snippets 전체 입력·쓰기/Toast/close drain, native 전체 shutdown·자동완성 UI/삽입, 나머지 Settings/App/assets/최종 gate·사용자 담당 마지막 CJK/VoiceOver 검증도 남습니다. 최신 browser bindings는 선행 키바인딩 소스이며 새 probe에는 전역 snippet_list fixture가 필요합니다.

이번 좁은 구현/직접 검사/기록3/3(100%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·workflow/서브에이전트 없음·전체 M8 완료 전 Git 없음·live 검사 없음입니다. 제품 TS/OS/Keychain/보호 앱/사용자 데이터·의존성/lock/MSRV는 변경하지 않았습니다.
