# 문서별 LSP 자동완성 옵션 공급

현재 상태: SDK의 정적/동적 completion 옵션 공급과 검증을 완료했습니다. 자동완성 화면·삽입의 전체 완료는 배치 22 QA에서 별도로 판정합니다.

대상은 crates/taide-lsp/src/native.rs, native/registration.rs, native/session.rs입니다. 설치된 공식 lsp-types 0.97.0의 CompletionOptions와 기존 문서별 signature 옵션 공급을 기준으로 구현했습니다. signature/completion의 동일 문서·selector 경계를 공유하며 새 의존성은 추가하지 않았습니다.

SessionHandle은 실행 상태의 문서 mirror에 해당하는 정적 completionProvider와 동적 textDocument/completion 등록 옵션을 공급합니다. 등록 revision·문서 selector·재등록·해제·닫힘·채널 종료를 캐시 수명에 반영합니다. 잘못된 triggerCharacters, allCommitCharacters, resolveProvider와 completionItem 옵션은 등록을 부분 적용하지 않고 거절합니다.

cargo test --manifest-path crates/taide-lsp/Cargo.toml --package taide-lsp --locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast -- --test-threads=4를 직접 실행했습니다. /private/tmp/taide-batch22-completion-product-sdk-all.log는 exit 0, 전체 2대상·86통과·0실패·0ignored입니다. 실제 앱 child/본문/peek 소비 근거는 2026-10-10-native-batch22-completion-snippets.md에 저장합니다.

실제 앱 데이터·클립보드·OS 입력·보호 앱 번들을 사용하지 않았습니다. SDK 공급 완료를 전체 전환율로 환산하지 않습니다.
