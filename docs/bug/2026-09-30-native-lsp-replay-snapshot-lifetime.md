# Native LSP replay 사본의 완료 후 보유

## 대상

`crates/taide-lsp/src/native.rs::LspCoordinator::finish_replay`, 새 replay 소유 수명 unit, 기존 coordinator/native-session fixture입니다.

## 관찰과 원인

initialize reply에서 문서 사본을 만들고 write가 끝난 뒤 실제 문서와 비교해 replay 중 편집·닫기/재열기의 delta를 보냅니다. 기존 finish_replay는 delta가 없어 Running으로 진입하는 경우에도 documents를 다시 clone하고 사본을 유지했습니다. Running 이후 close가 원본만 지우므로 replay 사본의 마지막 텍스트·URI·language는 restart/disconnect/finish_stop 전까지 보유됩니다.

신규 unit은 delta write 중 사본 유지·revision 갱신·stale generation 거절을 먼저 확인한 뒤 Running의 replay 사본 비움을 기대했습니다. 최초 관찰은 phase Running인데 replay map이 비어 있지 않은 assertion 실패입니다(exit 101, 0.00초). 실제 allocator RSS 측정이나 OS 메모리 누수 주장과 구분합니다.

## 수정

outgoing delta가 비었을 때만 replay map을 clear하고 Running으로 전환해 바로 반환합니다. delta가 있으면 기존처럼 최신 mirror 사본을 보유합니다. Running 진입 전에 canonical documents를 지우지 않으며 다음 restart는 실제 최신 documents에서 새 replay를 만듭니다. [BTreeMap::clear 공식 계약](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html#method.clear)을 확인했습니다. 새 상한·메모리 정책·dependency·GUI 선택은 없습니다.

## 검증

```sh
cargo test -p taide-lsp --lib replay_snapshot --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test coordinator replay_write_중 --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo test --manifest-path experiments/lsp-coordinator-spike/Cargo.toml --test native-session native_owner는_crash --target-dir experiments/terminal-core-spike/target --locked --offline -- --nocapture
cargo clippy -p taide-lsp --lib --tests --target-dir experiments/terminal-core-spike/target --locked --offline -- -D warnings
```

수정 뒤 소유 수명 unit 1건·기존 replay write 중 편집/close-open 1건은 각각 0.00초 통과했습니다. 새 unit은 다음 generation의 최신 문서 didOpen·재복구 뒤 사본 해제·close 뒤 양 map 비움도 확인합니다. 실제 crash→pending 실패→child 회수→새 generation 최신 mirror→hover→stop/join 1건은 진단 보강 뒤 0.14초 통과했습니다. strict LSP clippy exit 0(0.92초), 진단이 변경된 native-session test target clippy exit 0(0.88초)이며 대상 Rust format·문서 Prettier·diff 검사 exit 0입니다. 기존 성공 검사는 반복하지 않았습니다.

실제 native-session 첫 실행은 initial Running 대기에서 3.00초 Elapsed로 실패했습니다. 당시 snapshot을 보존하지 않아 원인은 미확정입니다. 해당 initial assertion에 result·SessionSnapshot 진단을 추가한 다음 실행은 성공했으며 timeout·제품 동작은 바꾸지 않았습니다. sandbox가 ps를 거절해 테스트 executable 경로만 대상으로 한 read-only 확대 조회를 수행했고 남은 합성 child/test 프로세스는 없었습니다. 사용자 앱·설정은 변경하지 않았습니다. 두 test별 filter를 한 Cargo 명령에 넣은 최초 시도는 CLI 인자 오류로 검사를 실행하지 못했고, 올바른 별도 명령으로 위 결과를 얻었습니다.

## 남은 위험

replay가 실제 진행 중일 때 필요한 문서 사본과 wire JSON은 여전히 존재합니다. 문서/URI/language·initialize/capabilities의 전체 retained allocation quota와 process peak RSS·latency는 미완료입니다. Running 이후 사본 해제를 native LSP 전체 memory gate나 M8 완료로 계산하지 않습니다.

initial Running 대기 실패가 관련 source 변경 또는 CI에서 재발하면 보강한 snapshot으로 phase/failure/pending/pid를 먼저 판정합니다. 현재 원인을 추정해 timeout을 늘리거나 성공 검사를 반복하지 않습니다. 제품 MSRV 1.89·native GUI·실제 언어 서버·장기 부하 검증은 별도입니다.
