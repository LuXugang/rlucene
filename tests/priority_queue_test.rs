use rand::Rng;
use std::fmt::Debug;
use RLucene::Compare;
use RLucene::PriorityQueue::PriorityQueue;

struct I32Compare;

impl Compare<i32> for I32Compare {
    fn less_than(&self, a: &i32, b: &i32) -> bool {
        a < b
    }
}
#[test]
fn test_zero_sized_queue() {
    let mut pq = PriorityQueue::new(0, I32Compare).unwrap();
    assert_eq!(1, pq.insert_with_overflow(1).unwrap());
    assert_eq!(0, pq.size());

    pq.add(1);
    assert_eq!(1, *pq.top())
}

struct ObjectCompare {
    index: i32,
    value: i32,
}

impl Default for ObjectCompare {
    fn default() -> Self {
        ObjectCompare { index: 0, value: 0 }
    }
}

impl PartialEq for ObjectCompare {
    fn eq(&self, other: &Self) -> bool {
        if self.index == other.index && self.value == other.value {
            return true;
        }
        false
    }
}

impl ObjectCompare {
    fn new(index: i32, value: i32) -> Self {
        ObjectCompare { index, value }
    }
}

impl Compare<ObjectCompare> for ObjectCompare {
    fn less_than(&self, a: &ObjectCompare, b: &ObjectCompare) -> bool {
        a.value < b.value
    }
}

#[test]
fn test_no_extra_work_on_equal_elements() {
    let mut pq = PriorityQueue::new(5, ObjectCompare::default()).unwrap();
    for i in 0..100 {
        pq.insert_with_overflow(ObjectCompare::new(i, 0));
    }
    let mut indexes: Vec<i32> = Vec::new();
    let iter = pq.iterator();
    for e in iter {
        indexes.push(e.index)
    }
    assert_eq!(indexes, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_pq() {
    let mut gen = rand::thread_rng();
    let count: i32;
    if gen.gen_bool(0.5) {
        if gen.gen_bool(0.5) {
            count = 0;
        } else {
            count = i32::MAX;
        }
    } else {
        count = gen.gen_range(10_000..1000000);
    }
    let pq = PriorityQueue::new(count, I32Compare);
    if let Ok(mut heap) = pq {
        let mut sum: i32 = 0;
        let mut sum2: i32 = 0;
        for _i in 0..count {
            let next: i32 = gen.gen();
            sum = sum.wrapping_add(next);
            heap.add(next);
        }

        let mut last = i32::MIN;
        for _i in 0..count {
            let next = heap.pop().unwrap();
            assert!(next >= last);
            last = next;
            sum2 = sum2.wrapping_add(last);
        }

        assert_eq!(sum, sum2);
    } else {
        assert!(count <= 0 || count >= i32::MAX);
    }
}

#[test]
fn test_clear() {
    let mut pq = PriorityQueue::new(3, I32Compare).unwrap();
    pq.add(2);
    pq.add(3);
    pq.add(1);
    assert_eq!(3, pq.size());
    pq.clear();
    assert_eq!(0, pq.size());
}

#[test]
fn test_fixed_size() {
    let mut pq = PriorityQueue::new(3, I32Compare).unwrap();
    pq.insert_with_overflow(2);
    pq.insert_with_overflow(3);
    pq.insert_with_overflow(1);
    pq.insert_with_overflow(5);
    pq.insert_with_overflow(7);
    pq.insert_with_overflow(1);
    assert_eq!(3, pq.size());
    assert_eq!(3, pq.pop().unwrap());
}

#[test]
fn test_insert_with_overflow() {
    let size = 4;
    let mut pq = PriorityQueue::new(size, I32Compare).unwrap();
    let i1 = 2;
    let i2 = 3;
    let i3 = 1;
    let i4 = 5;
    let i5 = 7;
    let i6 = 1;

    assert_eq!(pq.insert_with_overflow(i1), None);
    assert_eq!(pq.insert_with_overflow(i2), None);
    assert_eq!(pq.insert_with_overflow(i3), None);
    assert_eq!(pq.insert_with_overflow(i4), None);
    assert_eq!(pq.insert_with_overflow(i5).unwrap(), i3);
    assert_eq!(pq.insert_with_overflow(i6).unwrap(), i6);
    assert_eq!(size as usize, pq.size());
    assert_eq!(2, *pq.top());
}

#[test]
fn test_add_all_to_empty_queue() {
    let mut gen = rand::thread_rng();
    let size = 10;
    let mut list: Vec<i32> = Vec::new();
    let mut list2: Vec<i32> = Vec::new();
    let mut value: i32;
    for _i in 0..size {
        value = gen.gen();
        list.push(value);
        list2.push(value);
    }
    let mut pq = PriorityQueue::new(size, I32Compare).unwrap();
    pq.add_all(list);
    check_validity(&pq);
    assert_ordered_when_drained(&mut pq, list2);
}

#[test]
fn test_add_all_to_partially_filled_queue() {
    let mut pq = PriorityQueue::new(20, I32Compare).unwrap();
    let mut one_by_one: Vec<i32> = Vec::new();
    let mut bulk_added: Vec<i32> = Vec::new();
    let mut bulk_added2: Vec<i32> = Vec::new();
    let mut gen = rand::thread_rng();
    for _i in 0..10 {
        let value: i32 = gen.gen();
        bulk_added.push(value);
        bulk_added2.push(value);
        let x: i32 = gen.gen();
        pq.add(x);
        one_by_one.push(x);
    }

    pq.add_all(bulk_added);
    check_validity(&pq);

    one_by_one.append(&mut bulk_added2);
    assert_ordered_when_drained(&mut pq, one_by_one);
}

#[test]
fn test_add_all_does_not_fit_into_queue() {
    let mut pq = PriorityQueue::new(20, I32Compare).unwrap();
    let mut list: Vec<i32> = Vec::new();
    let mut random = rand::thread_rng();
    for _i in 0..11 {
        list.push(random.gen());
        pq.add(random.gen());
    }
    let result = pq.add_all(list).unwrap_err();
    assert_eq!(
        result,
        "Cannot add 11 elements to a queue with remaining capacity: 9"
    );
}

#[test]
fn test_removals_and_insertions() {
    let mut random = rand::thread_rng();
    let num_docs_in_pq = random.gen_range(1..=100);
    let mut pq = PriorityQueue::new(num_docs_in_pq, I32Compare).unwrap();
    let mut last_least: Option<i32> = None;

    // Basic insertion of new content
    let mut sds: Vec<i32> = Vec::with_capacity(num_docs_in_pq as usize);
    for _i in 0..num_docs_in_pq * 10 {
        let new_entry = random.gen::<i32>().abs();
        sds.push(new_entry);
        let evicted = pq.insert_with_overflow(new_entry);
        check_validity(&pq);
        if let Some(evicted_value) = evicted {
            let pos = sds.iter().position(|&x| x == evicted_value);
            assert_ne!(pos, None);
            sds.remove(pos.unwrap());
            if evicted_value != new_entry {
                assert_eq!(evicted_value, last_least.unwrap());
            }
        }
        let new_least = pq.top();
        if last_least != None && *new_least != new_entry && *new_least != last_least.unwrap() {
            // If there has been a change of least entry and it wasn't our new
            // addition we expect the scores to increase
            assert!(*new_least <= new_entry);
            assert!(*new_least >= last_least.unwrap());
        }
        last_least = Some(*new_least);
    }
    // Try many random additions to existing entries - we should always see
    // increasing scores in the lowest entry in the PQ
    for _i in 0..500000 {
        let element = (random.gen::<f32>() * ((sds.len() - 1) as f32)) as i32;
        let object_to_remove = sds[element as usize];
        assert_eq!(sds.remove(element as usize), object_to_remove);
        assert!(pq.remove(&object_to_remove));
        check_validity(&pq);
        let new_entry = random.gen::<i32>().abs();
        sds.push(new_entry);
        assert_eq!(pq.insert_with_overflow(new_entry), None);
        check_validity(&pq);
        let new_least = pq.top();
        if object_to_remove != last_least.unwrap() && last_least != None && *new_least != new_entry
        {
            // If there has been a change of least entry and it wasn't our new
            // addition or the loss of our randomly removed entry we expect the
            // scores to increase
            assert!(*new_least <= new_entry);
            assert!(*new_least >= last_least.unwrap());
        }
        last_least = Some(*new_least);
    }
}

#[test]
fn test_iterator_empty() {
    let mut pq = PriorityQueue::new(3, I32Compare).unwrap();
    let mut it = pq.iterator();
    assert_eq!(*&it.next(), None);
}

#[test]
fn test_iterator_one() {
    let mut pq = PriorityQueue::new(3, I32Compare).unwrap();
    pq.add(1);
    let mut it = pq.iterator();
    assert_eq!(*&it.next(), Some(&1));
}

#[test]
fn test_iterator_two() {
    let mut pq = PriorityQueue::new(3, I32Compare).unwrap();
    pq.add(1);
    pq.add(2);
    let mut it = pq.iterator();
    assert_eq!(*&it.next(), Some(&1));
    assert_eq!(*&it.next(), Some(&2));
}

#[test]
fn test_iterator_random() {
    let mut random = rand::thread_rng();
    let max_size: usize = random.gen_range(1..20);
    let mut queue = PriorityQueue::new(max_size as i32, I32Compare).unwrap();
    let iters: usize = random.gen_range(100..500);
    let mut expected: Vec<i32> = Vec::new();
    for i in 0..iters {
        if queue.size() == 0 || (queue.size() < max_size) {
            // if queue.size() == 0 || (queue.size() < max_size && random.gen::<bool>()) {
            let value: i32 = random.gen_range(0..=10);
            queue.add(value);
            expected.push(value);
        } else {
            let pos = expected.iter().position(|&x| x == queue.pop().unwrap());
            assert_ne!(pos, None);
            expected.remove(pos.unwrap());
        }
        let mut actual: Vec<i32> = Vec::new();
        for value in queue.iterator() {
            actual.push(*value);
        }
        expected.sort();
        actual.sort();
        assert_eq!(actual, expected);
    }
}

#[test]
fn test_max_int_size() {
    let pq = PriorityQueue::new(i32::MAX, I32Compare);
    assert!(pq.is_err());
}

fn assert_ordered_when_drained<T, C>(
    pq: &mut PriorityQueue<T, C>,
    mut reference_data_list: Vec<i32>,
) where
    C: Compare<T>,
    T: Into<i32> + Default + Debug + PartialEq,
{
    reference_data_list.sort();
    let mut i = 0;
    let mut value: i32;
    while pq.size() > 0 {
        value = pq.pop().unwrap().into();
        assert_eq!(reference_data_list[i], value);
        i += 1;
    }
}

fn check_validity<T, C>(pq: &PriorityQueue<T, C>)
where
    C: Compare<T>,
    T: Default + PartialEq + Debug,
{
    let size = pq.size();
    let heap = pq.heap();
    for i in 1..=size {
        let parent = i >> 1;
        if parent > 1 {
            if pq.get_compare().less_than(&heap[parent], &heap[i]) == false {
                assert_eq!(&heap[parent], &heap[i]);
            }
        }
    }
}
