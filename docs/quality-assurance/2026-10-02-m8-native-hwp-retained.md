# M8 HWP typed retained payload

## 대상과 현재 상태

`native/taide-native-retained/{src/lib.rs,derive/src/lib.rs,tests/retained.rs,README.md}`, rhwp vendor의 선택적 `native-retained` feature와 실제 model/Core/render graph, app `tests/preview-hwp3-boundary.rs`가 대상입니다. N5-P1g의 Core 소유 수명·retained admission 안에서 계산 기반을 구현했습니다. 전체 M8/N5/HWP provider 및 persistent Core는 아직 미완료입니다.

## 구현

1. derive는 struct/tuple/enum의 모든 field·모든 variant에 trait 계약을 요구합니다. unsupported field/union은 compile-time 오류이며 skip 옵션이 없습니다. scalar-only type은 element 방문을 생략하고 Vec/VecDeque/String capacity·Box·Arc layout·중첩 payload를 계산합니다. 실제 방문은 bounded work queue이고 Arc allocation은 identity별 한 번만 계산합니다.
2. RefCell/Mutex/RwLock subtree는 guard 수명 안에서만 읽습니다. mutable borrow·busy/poison lock·byte/visit budget·allocation 실패를 반환하며 unsafe Sync·강제 unlock을 사용하지 않습니다. 같은 owner의 cycle은 Arc 방문으로 끊습니다.
3. rhwp model의 원시 record·문자 위치·fonts/resources·표/도형/그림·중첩 paragraphs·loaded BinData와 실제 Core의 style/composed/pagination/measured/render/layout/normalization·HML metadata·validation·snapshot/clipboard/pending pagination field를 연결합니다. `serde(skip)`와 private cache도 비용에서 생략하지 않습니다.
4. 초기 IR/Core 연결 시 Lazy CFB/ZIP resolver는 Opaque였습니다. 후속 source 기반 adapter로 HWP5/HWPX resolver를 연결했습니다. 알 수 없는 custom resolver만 Opaque이며 측정할 때 이미지를 materialize하지 않습니다. production persistent Core/admission은 아직 미연결입니다.

논리 retained payload는 peak RSS·allocator overhead·HashMap private control bucket·GPU allocation 상한이 아닙니다. HashMap/HashSet는 공개 capacity의 entry payload와 소유 child를 더합니다. 전체 heap/RSS 안전성을 확보했다는 의미로 사용하지 않습니다.

local helper/derive 외 새 third-party 버전은 추가하지 않았습니다. proc-macro2 1.0.107·quote 1.0.47·syn 2.0.119는 app lock에 이미 존재하는 버전을 재사용합니다. app lock에는 local package 2개만 추가됐고 root manifest/lock·MSRV와 사용자 실기 bundle은 이 경계에서 변경하지 않았습니다.

## 실제 검증

- [x] helper check/offline — 최초 own lock 생성 포함 exit 0, 1.49초.
- [x] helper runtime 4건 — 타입/enum/generic/cfg/Cow/VecDeque/map capacity, Arc alignment/duplicate/slice, 실제 2MiB stack의 깊이 10,000 Box 트리·1MiB scalar vector, budget/overflow/borrow/lock/cycle. compile 0.47초·suite 0.00초, 4 PASS. 보통 Rustdoc 1 PASS와 compile-fail 3 PASS, 문서 검사 0.96초입니다.
- [x] helper strict — 첫 test fixture의 Arc<RefCell>이 `arc_with_non_send_sync`로 실패했습니다. cycle fixture만 Mutex로 수정한 영향 검사 `--test retained 예산` 1 PASS(compile 0.25초·suite 0.00초), 최종 workspace/all-targets clippy `-D warnings` exit 0(0.14초)입니다. 기존 runtime 3건·문서 성공은 반복하지 않았습니다. 이후 Opaque variant/실제 model 연결의 소비 검사는 아래 app에서 수행했습니다.
- [x] 실제 IR — `hwp_retained는_실제_ir_capacity와_중첩_payload를_계산하고_opaque를_거부한다` 1 PASS, compile 19.98초·suite 0.00초입니다. 실제 compressed HWP3 IR에서 text/offset/raw-header/hidden-comment/font/loaded-image의 capacity 증가분 합계와 계산 증가분이 정확히 같습니다. 실제 한계값 허용/한 바이트 초과 거절·Lazy 로드 없이 Opaque 오류를 확인했습니다. 최초 model check는 repr attribute 때문에 derive 추가에서 빠진 LinkLineType을 찾아 실패했고 해당 타입에 동일 조건부 derive를 추가했습니다.
- [x] 실제 Core — `hwp_retained_core는_실제_worker_stack과_캐시_증가_공유_소유권을_계산한다` 1 PASS, compile 15.32초·suite 0.01초입니다. 실제 2MiB stack의 허용 깊이 31 IR/Core, Core 비용이 IR보다 큼, byte cap, Arc<Mutex<Core>> 중복 비용/잠금 오류, HML preserved metadata, 실제 page tree+layer JSON cache 비용 증가와 동일 cache 재사용을 확인했습니다. 모든 Core 소유 field 연결 뒤 app lib check exit 0(8.39초), app lib/bin/해당 test strict `-D warnings` exit 0(8.82초)입니다.

명령은 helper 또는 app의 정확한 Cargo.toml을 지정했고 `--locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. own lock 최초 생성 때만 locked 없이 check했습니다. Cargo는 직렬 실행했고 성공한 예전 HWP3/HML/native/host/parser 검사는 재실행하지 않았습니다. GUI·OS clipboard·input/VoiceOver·사용자 파일은 조작하지 않았습니다.

## 근거와 잔여

Vec capacity/inline+heap 구분은 [Rust Vec guarantees](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees), Arc header/data layout은 설치된 Rust alloc/sync source의 ArcInner 두 AtomicUsize와 data alignment가 근거입니다. derive는 설치된 정확한 syn 2.0.119/quote 1.0.47의 README·Fields::members·generic split API source를 확인했습니다. website 최신 Rust 문서의 버전이 설치된 1.98.1과 다를 수 있으므로 layout은 설치된 source를 기준으로 합니다.

- [x] HWP5/HWPX Lazy resolver의 공유 CFB/ZIP 컨테이너 source 기반 payload adapter와 그 비용·중복 소유 검사. 아래 후속 결과를 참조합니다.
- [x] production persistent Core·retained admission/cache 연결, owner별 close/invalidation/bytes 교체/approval 재확인/종료 검사. `2026-10-02-m8-native-hwp-persistent-core.md`에 후속 구현과 실제 unit/host/strict 결과를 기록했습니다.
- [ ] allocator/HashMap bucket·실제 전체 RSS/CPU/취소/crash isolation, 원본 corpus/font/픽셀·OS/GPU/AX 및 상위 N1~N8 게이트.

production persistent Core/admission 후속의 코드 수명 경계는 연결됐습니다. 실제 RSS/corpus/실기는 입력과 실행 경계가 다른 후속 검증으로 남기며 이번 logical payload 성공으로 면제하지 않습니다.

## 후속: CFB/ZIP 공유 Lazy source

정확한 기존 registry cfb 0.14.0/zip 8.6.0 source를 local optional adapter로 가져왔습니다. fork의 UPSTREAM.md에 revision/license/hash와 조건부 trait 변경 범위를 기록했습니다. CFB는 실제 Cursor 원본·FAT/DIFAT·directory·mini allocator를, ZIP은 실제 Cursor 원본·archive metadata·entry capacity·keys/values·이름·extra-field Arc·OnceLock을 방문합니다. resolver/내부 공유 allocation은 identity별 한 번입니다. reader/decompress/parser semantics를 수정하지 않았고 raw bytes를 container 전체 비용으로 간주하지 않습니다.

- [x] helper Cursor/OnceLock 신규 검사 1 PASS, compile 0.43초·suite 0.00초. 새 shared Arc/opaque 경계까지 workspace/all-targets strict exit 0, 0.31초입니다.
- [x] `hwp_retained_cfb` 실제 lazy HWP5의 원본보다 큰 비용, 정확한 resolver+mini allocator 2 shared allocation, 같은 BinData 중복의 정확한 Vec/key/extension 증가분, read 뒤 비용 유지, cap 거절·실제 Core·unknown Opaque를 검사했습니다. 1 PASS, compile 0.75초·suite 0.03초입니다. 최초 fixture는 DocInfo raw stream dirty가 없어 추가 entry가 serialize되지 않았고, 다음 fixture의 attribute 0은 Link로 해석됐습니다. 실제 serializer/parser 확인 후 dirty=true/Embedding attribute 1로 정정했습니다. product parser를 fixture에 맞춰 바꾸지 않았습니다.
- [x] `hwp_retained_zip` 실제 lazy HWPX의 동일 경계 1 PASS, compile 0.75초·suite 0.03초입니다. 첫 shared count 예상 2와 관찰 15의 차이는 실제 ZIP extra-field Arc 13개였습니다. 확인 후 최소 owner 2와 duplicate의 정확한 증가분/동일 shared count를 검사하도록 정정했습니다. 비용 cap을 완화하지 않았습니다.
- [x] CFB 연결 app check exit 0(8.77초), ZIP 연결 check exit 0(8.92초), 최종 app lib/bin/preview-hwp strict `-D warnings` exit 0(8.80초)입니다. 성공한 helper/IR/Core/HWP3/HML/host 검사는 반복하지 않았습니다.

기존 버전/runtime dependency를 유지했으며 native/root MSRV는 변경하지 않았습니다. optional helper는 native MSRV 1.95가 필요하므로 원본 CFB 1.74/ZIP 1.88을 feature-on 보장으로 주장하지 않습니다. app lock의 registry/local zip 8.6 두 copy는 packaging/성능 gate에 남습니다. 공개 IndexMap entry capacity는 계산하되 private hash/index/control bookkeeping은 포함하지 않습니다. 이것은 logical payload이지 전체 heap/RSS/CPU cap이 아닙니다. 현재 production page별 Core 재파싱은 남으며 다음에 이를 제거합니다.
