use std::cmp::max;
use std::mem;
use std::ptr::null;

struct PriorityQueue<T> {
    size: i32,
    max_size: i32,
    // maybe we should ues Vec
    heap: Box<[T]>,
}

impl<T> PriorityQueue<T> {
    fn with_sentinel_object<F>(
        max_size: i32,
        sentinel_object_supplier: F,
    ) -> Result<PriorityQueue<T>, String>
    where
        F: Fn() -> Option<T>,
        T: Default + Clone + PartialOrd,
    {
        let heap_size = if 0 == max_size {
            // We allocate 1 extra to avoid if statement in top()
            2
        } else {
            if max_size < 0 || max_size >= i32::MAX {
                return Err(format!(
                    "maxSize must be >= 0 and < {}; got: {}",
                    i32::MAX,
                    max_size
                ));
            }
            // NOTE: we add +1 because all access to heap is
            // 1-based not 0-based.  heap[0] is unused.
            max_size + 1
        };
        let mut heap: Box<[T]> = vec![T::default(); heap_size as usize].into_boxed_slice();
        if let Some(sentinel) = sentinel_object_supplier() {
            heap[1] = sentinel;
            for i in 2..heap.len() {
                heap[i] = sentinel_object_supplier().unwrap();
            }
            return Ok(PriorityQueue {
                max_size: heap_size,
                size: heap_size,
                heap,
            });
        }
        Ok(PriorityQueue {
            max_size: heap_size,
            size: 0,
            heap,
        })
    }

    // construct
    fn new(max_size: i32) -> Result<PriorityQueue<T>, String>
    where
        T: Default + Clone + PartialOrd,
    {
        Self::with_sentinel_object(max_size, || None)
    }

    fn add_all<I>(&self, elements: I) -> Result<(), String>
    where
        I: AsRef<[T]>,
    {
        if (self.size + elements.as_ref().len() as i32) > self.max_size {
            return Err(format!(
                "Cannot add {} elements to a queue with remaining capacity: {}",
                elements.as_ref().len(),
                self.max_size - self.size
            ));
        }
        // Heap with size S always takes first S elements of the array,
        // and thus it's safe to fill array further - no actual non-sentinel value will be overwritten.

        todo!()
    }
}
