# Native retained payload 계산

`RetainedBytes`는 실제 타입의 모든 필드를 순회해 소유 payload의 논리 보유 비용을 계산합니다. RSS·allocator overhead·HashMap 내부 control bucket·GPU allocation 값이 아닙니다. opaque 외부 컨테이너는 임의로 0을 반환하지 말고 별도의 source 기반 adapter를 구현해야 합니다.

root inline 크기, Vec/VecDeque/String capacity, Box allocation과 중첩 field, Arc allocation layout/중복 참조, HashMap/HashSet의 공개 capacity에 해당하는 entry payload를 합산합니다. Arc counter/header layout은 설치된 Rust alloc source의 두 atomic counter와 data alignment를 기준으로 합니다. borrow된 reference/Cow payload는 소유 비용을 더하지 않습니다.

field 방문은 명시적 work queue이며 primitive-only vector는 element별 방문을 생략합니다. RefCell/Mutex/RwLock의 guard가 필요한 subtree는 guard 수명 안에서만 순회합니다. mutable borrow·잠금·poison·byte/visit cap·내부 allocation 실패는 오류로 반환하며 unsafe·강제 잠금 해제는 사용하지 않습니다. custom RetainedBytes 구현은 방문하는 field와 공유 allocation identity의 정확성을 보장해야 합니다.

## 사용

```rust
use taide_native_retained::{Limits, RetainedBytes, measure};

#[derive(RetainedBytes)]
struct Document {
    text: String,
    lines: Vec<String>,
}

let value = Document { text: "synthetic".to_owned(), lines: vec!["line".to_owned()] };
let report = measure(&value, Limits { bytes: 4096, visits: 64 }).unwrap();
assert!(report.bytes >= size_of::<Document>() + value.text.capacity());
```

## 지원하지 않는 field는 compile-time 오류

모든 field가 trait을 구현해야 하며, 이름·tuple 위치·enum variant를 조용히 생략하는 옵션은 제공하지 않습니다.

```compile_fail
use taide_native_retained::RetainedBytes;

struct Opaque;

#[derive(RetainedBytes)]
struct Document {
    unaccounted: Opaque,
}
```

```compile_fail
use taide_native_retained::RetainedBytes;

struct Opaque;

#[derive(RetainedBytes)]
enum Document {
    Empty,
    Hidden { unaccounted: Opaque },
}
```

```compile_fail
use taide_native_retained::RetainedBytes;

#[derive(RetainedBytes)]
union Document {
    data: usize,
}
```

rhwp의 선택적 `native-retained` feature가 실제 IR·Core·style/composed/pagination/measured/render/layout cache의 모든 소유 field를 연결합니다. source 기반 local CFB/ZIP adapter는 Lazy resolver의 원본·FAT·directory·archive metadata·이름·extra-field payload를 방문하며 공유 resolver는 한 번만 계산합니다. 알 수 없는 custom resolver는 materialize하지 않고 `Error::Opaque`로 실패합니다. ZIP IndexMap의 공개 entry capacity는 계산하지만 private hash/index bookkeeping은 포함하지 않습니다. PathBuf는 실제 소유 capacity를 계산합니다. native preview는 이 비용을 persistent Core의 post-load/post-render 논리 admission에 사용합니다. 이것은 할당 전 peak/RSS cap이 아닙니다.
