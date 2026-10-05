use std::{
    alloc::Layout,
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock, atomic::AtomicUsize},
};

use taide_native_retained::{Error, Limits, RetainedBytes, Visitor, measure};

const BYTES: usize = 4 * 1024 * 1024;
const VISITS: usize = 100_000;
const LARGE_CAPACITY: usize = 1024 * 1024;
const DEPTH: usize = 10_000;
const STACK_BYTES: usize = 2 * 1024 * 1024;
const CAPACITY: usize = 32;
const PREFIX: usize = 16;

#[test]
fn path_buf는_실제_소유_capacity를_계산한다() {
    let mut path = std::path::PathBuf::with_capacity(CAPACITY);
    path.push("synthetic");
    assert_eq!(
        measure(&path, limits()).unwrap().bytes,
        size_of_val(&path) + path.capacity()
    );
    let before = path.capacity();
    path.reserve(LARGE_CAPACITY);
    assert!(path.capacity() > before);
    assert_eq!(
        measure(&path, limits()).unwrap().bytes,
        size_of_val(&path) + path.capacity()
    );
}

fn limits() -> Limits {
    Limits {
        bytes: BYTES,
        visits: VISITS,
    }
}

#[derive(RetainedBytes)]
struct Generic<'a, T> {
    items: Vec<T>,
    borrowed: &'a str,
    marker: PhantomData<T>,
    #[cfg(any())]
    absent: Unsupported,
}

#[derive(RetainedBytes)]
struct Tuple(String, Box<[u8]>);

#[derive(RetainedBytes)]
enum Variant {
    Empty,
    Named {
        text: String,
        payload: [Option<Tuple>; 1],
    },
    Tuple(VecDeque<String>, Cow<'static, str>),
}

#[test]
fn derive는_struct_tuple_enum_generic과_공개_capacity를_빠짐없이_계산한다() {
    let mut text = String::with_capacity(CAPACITY);
    text.push_str("synthetic");
    let mut payload = Vec::with_capacity(PREFIX);
    payload.push(Variant::Named {
        text: text.clone(),
        payload: [Some(Tuple(
            text.clone(),
            vec![1; PREFIX].into_boxed_slice(),
        ))],
    });
    payload.push(Variant::Empty);
    let queue = VecDeque::from([text.clone()]);
    let queue_capacity = queue.capacity();
    let owned = String::from("owned");
    let owned_capacity = owned.capacity();
    payload.push(Variant::Tuple(queue, Cow::Owned(owned)));
    let value = Generic {
        items: payload,
        borrowed: "borrowed",
        marker: PhantomData,
    };
    let expected_children = value
        .items
        .iter()
        .map(|variant| match variant {
            Variant::Empty => 0,
            Variant::Named { text, payload } => {
                let Tuple(inner, bytes) = payload[0].as_ref().unwrap();
                text.capacity() + inner.capacity() + bytes.len()
            }
            Variant::Tuple(_, _) => {
                queue_capacity * size_of::<String>() + text.len() + owned_capacity
            }
        })
        .sum::<usize>();
    let report = measure(&value, limits()).unwrap();
    assert_eq!(
        report.bytes,
        size_of_val(&value) + value.items.capacity() * size_of::<Variant>() + expected_children
    );
    assert_eq!(value.borrowed, "borrowed");
    assert_eq!(
        measure(&Cow::<str>::Borrowed("not owned"), limits())
            .unwrap()
            .bytes,
        size_of::<Cow<'_, str>>()
    );

    let mut map = HashMap::with_capacity(PREFIX);
    map.insert(text.clone(), vec![1u8; PREFIX]);
    let expected = size_of_val(&map)
        + map.capacity() * size_of::<(String, Vec<u8>)>()
        + map.keys().map(String::capacity).sum::<usize>()
        + map.values().map(Vec::capacity).sum::<usize>();
    assert_eq!(measure(&map, limits()).unwrap().bytes, expected);
}

#[derive(RetainedBytes)]
struct Shared {
    first: Arc<Vec<String>>,
    second: Arc<Vec<String>>,
    bytes: Arc<[u8]>,
    duplicate: Arc<[u8]>,
}

#[test]
fn cursor와_once_lock은_실제_소유_payload만_계산한다() {
    let cursor = std::io::Cursor::new(Vec::<u8>::with_capacity(CAPACITY));
    let text = std::sync::OnceLock::new();
    let before = measure(&text, limits()).unwrap();
    let owned = String::with_capacity(CAPACITY);
    text.set(owned).unwrap();
    let after = measure(&text, limits()).unwrap();
    assert_eq!(after.bytes - before.bytes, CAPACITY);
    assert_eq!(
        measure(&cursor, limits()).unwrap().bytes,
        size_of_val(&cursor) + cursor.get_ref().capacity()
    );
}

#[test]
fn shared_arc는_header_alignment와_payload를_한번만_계산한다() {
    let inner = Arc::new(vec![String::from("shared")]);
    let bytes: Arc<[u8]> = Arc::from([1u8, 2, 3]);
    let value = Shared {
        first: inner.clone(),
        second: inner,
        bytes: bytes.clone(),
        duplicate: bytes,
    };
    let layout = |size| {
        Layout::new::<(AtomicUsize, AtomicUsize)>()
            .extend(Layout::from_size_align(size, align_of::<Vec<String>>()).unwrap())
            .unwrap()
            .0
            .pad_to_align()
            .size()
    };
    let byte_layout = Layout::new::<(AtomicUsize, AtomicUsize)>()
        .extend(Layout::for_value(value.bytes.as_ref()))
        .unwrap()
        .0
        .pad_to_align()
        .size();
    let expected = size_of::<Shared>()
        + layout(size_of::<Vec<String>>())
        + value.first.capacity() * size_of::<String>()
        + value.first.iter().map(String::capacity).sum::<usize>()
        + byte_layout;
    let report = measure(&value, limits()).unwrap();
    assert_eq!(report.bytes, expected);
    assert_eq!(report.shared_allocations, 2);
}

#[derive(RetainedBytes)]
struct Link {
    payload: Vec<u8>,
    next: Option<Box<Link>>,
}

#[test]
fn work_queue는_깊은_owned_tree를_작은_stack에서_읽고_scalar_vector를_건너뛴다() {
    let thread = std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(|| {
            let bytes = Vec::<u8>::with_capacity(LARGE_CAPACITY);
            let report = measure(
                &bytes,
                Limits {
                    bytes: BYTES,
                    visits: 1,
                },
            )
            .unwrap();
            assert_eq!(report.visits, 1);
            assert_eq!(report.bytes, size_of::<Vec<u8>>() + bytes.capacity());
            let mut chain = None;
            for _ in 0..DEPTH {
                chain = Some(Box::new(Link {
                    payload: Vec::new(),
                    next: chain,
                }));
            }
            let report = measure(&chain, limits()).unwrap();
            assert_eq!(
                report.bytes,
                size_of_val(&chain) + DEPTH * size_of::<Link>()
            );
            while let Some(mut value) = chain {
                chain = value.next.take();
            }
        })
        .unwrap();
    thread.join().unwrap();
}

struct Overflow;
impl RetainedBytes for Overflow {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        visitor.capacity::<u64>(usize::MAX)
    }
}

#[derive(RetainedBytes)]
struct Cyclic {
    text: String,
    next: Mutex<Option<Arc<Cyclic>>>,
}

#[test]
fn 예산_잠금_borrow와_공유_cycle은_우회없이_오류나_한번_방문으로_종료한다() {
    let text = String::from("synthetic");
    assert_eq!(
        measure(
            &text,
            Limits {
                bytes: size_of::<String>(),
                visits: VISITS
            }
        ),
        Err(Error::ByteBudget)
    );
    assert_eq!(
        measure(
            &text,
            Limits {
                bytes: BYTES,
                visits: 0
            }
        ),
        Err(Error::VisitBudget)
    );
    assert_eq!(measure(&Overflow, limits()), Err(Error::ByteBudget));
    let value = RefCell::new(text.clone());
    let guard = value.borrow_mut();
    assert_eq!(measure(&value, limits()), Err(Error::Borrowed));
    drop(guard);
    assert_eq!(
        measure(&value, limits()).unwrap().bytes,
        size_of_val(&value) + text.capacity()
    );
    let mutex = Mutex::new(text.clone());
    let guard = mutex.lock().unwrap();
    assert_eq!(measure(&mutex, limits()), Err(Error::Locked));
    drop(guard);
    assert_eq!(
        measure(&mutex, limits()).unwrap().bytes,
        size_of_val(&mutex) + text.capacity()
    );
    let lock = RwLock::new(text.clone());
    let guard = lock.write().unwrap();
    assert_eq!(measure(&lock, limits()), Err(Error::Locked));
    drop(guard);
    assert_eq!(
        measure(&lock, limits()).unwrap().bytes,
        size_of_val(&lock) + text.capacity()
    );
    let root = Arc::new(Cyclic {
        text,
        next: Mutex::new(None),
    });
    *root.next.lock().unwrap() = Some(root.clone());
    let report = measure(&root, limits()).unwrap();
    root.next.lock().unwrap().take();
    assert_eq!(report.shared_allocations, 1);
    assert!(report.visits < PREFIX);
}
