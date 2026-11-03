use super::*;

fn list(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn 새_검색어를_맨_앞에_추가한다() {
    assert_eq!(
        add_recent_term(&list(&["b", "a"]), "c"),
        Some(list(&["c", "b", "a"]))
    );
}

#[test]
fn 이미_있는_검색어는_중복_제거_후_맨_앞으로_이동한다() {
    assert_eq!(
        add_recent_term(&list(&["a", "b", "c"]), "b"),
        Some(list(&["b", "a", "c"]))
    );
}

#[test]
fn 앞뒤_공백은_잘라내고_저장한다() {
    assert_eq!(add_recent_term(&[], "  hello  "), Some(list(&["hello"])));
}

#[test]
fn 빈_문자열이나_공백만_있는_검색어는_무시한다() {
    assert_eq!(add_recent_term(&list(&["a"]), ""), None);
    assert_eq!(add_recent_term(&list(&["a"]), "   "), None);
}

#[test]
fn 이미_맨_앞에_있는_검색어를_다시_추가하면_바뀌지_않는다() {
    let history = list(&["a", "b", "c"]);
    assert_eq!(add_recent_term(&history, "a"), None);
    assert_eq!(add_recent_term(&history, "  a  "), None);
}

#[test]
fn 상한_20개를_넘으면_가장_오래된_항목을_버린다() {
    let full = (0..SEARCH_HISTORY_LIMIT)
        .map(|index| format!("term-{index}"))
        .collect::<Vec<_>>();
    let result = add_recent_term(&full, "new-term").unwrap();
    assert_eq!(result.len(), SEARCH_HISTORY_LIMIT);
    assert_eq!(result[0], "new-term");
    assert!(!result.contains(&format!("term-{}", SEARCH_HISTORY_LIMIT - 1)));
}

#[test]
fn 설정_저장이_끝나기_전에_다음_검색어를_넣어도_앞선_검색어가_유실되지_않는다() {
    let mut queue = Queue::default();
    assert_eq!(queue.enqueue(&[], "foo"), Some(list(&["foo"])));
    assert_eq!(queue.enqueue(&[], "bar"), Some(list(&["bar", "foo"])));
    assert_eq!(queue.list(&[]), list(&["bar", "foo"]).as_slice());
}

#[test]
fn 저장이_모두_끝나면_서버가_돌려준_값을_다시_기준으로_삼는다() {
    let mut queue = Queue::default();
    queue.enqueue(&[], "foo");
    queue.settle();
    let settled = list(&["foo", "external"]);
    assert_eq!(queue.list(&settled), settled.as_slice());
    assert_eq!(
        queue.enqueue(&settled, "bar"),
        Some(list(&["bar", "foo", "external"]))
    );
}

#[test]
fn 저장이_여러_건_남아_있으면_마지막_하나가_끝날_때까지_기준을_유지한다() {
    let mut queue = Queue::default();
    queue.enqueue(&[], "foo");
    queue.enqueue(&[], "bar");
    queue.settle();
    assert_eq!(
        queue.enqueue(&[], "baz"),
        Some(list(&["baz", "bar", "foo"]))
    );
}

#[test]
fn 이미_맨_앞인_검색어는_저장을_예약하지_않는다() {
    let mut queue = Queue::default();
    assert_eq!(queue.enqueue(&list(&["foo"]), "foo"), None);
    assert_eq!(queue.in_flight, 0);
    queue.settle();
    assert_eq!(queue.in_flight, 0);
}
