/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::util::dummy::dummy_counter::DummyCounter;

pub trait Counter {
    /// Adds the given delta to the counter's current value.
    ///
    /// # Arguments
    /// * `delta` - The delta to add.
    ///
    /// # Returns
    /// The counter's updated value.
    fn add_and_get(&mut self, delta: i64) -> i64;
    /// Returns the counter's current value.
    ///
    /// # Returns
    /// The counter's current value.
    fn get(&self) -> i64;
}

pub struct AtomicCounter {
    count: AtomicI64,
}
impl Default for AtomicCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl AtomicCounter {
    pub fn new() -> AtomicCounter {
        AtomicCounter {
            count: AtomicI64::new(0),
        }
    }
}
impl Counter for AtomicCounter {
    fn add_and_get(&mut self, delta: i64) -> i64 {
        self.count
            .fetch_add(delta, std::sync::atomic::Ordering::SeqCst)
            + delta
    }
    fn get(&self) -> i64 {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }
}
pub struct SerialCounter {
    count: i64,
}
impl Default for SerialCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialCounter {
    pub fn new() -> SerialCounter {
        SerialCounter { count: 0 }
    }
}
impl Counter for SerialCounter {
    fn add_and_get(&mut self, delta: i64) -> i64 {
        self.count += delta;
        self.count
    }
    fn get(&self) -> i64 {
        self.count
    }
}

pub enum CounterEnum {
    Atomic(AtomicCounter),
    Serial(SerialCounter),
    Dummy(DummyCounter),
}
impl CounterEnum {
    /// Returns a new counter.
    ///
    /// # Arguments
    /// * `thread_safe` - `true` if the returned counter can be used by multiple
    ///   threads concurrently.
    ///
    /// # Returns
    /// A new counter.
    pub fn new_counter(thread_safe: bool) -> CounterEnum {
        if thread_safe {
            CounterEnum::Atomic(AtomicCounter::new())
        } else {
            CounterEnum::Serial(SerialCounter::new())
        }
    }
}
impl Counter for CounterEnum {
    fn add_and_get(&mut self, delta: i64) -> i64 {
        match self {
            CounterEnum::Atomic(c) => c.add_and_get(delta),
            CounterEnum::Serial(c) => c.add_and_get(delta),
            CounterEnum::Dummy(c) => c.add_and_get(delta),
        }
    }
    fn get(&self) -> i64 {
        match self {
            CounterEnum::Atomic(c) => c.get(),
            CounterEnum::Serial(c) => c.get(),
            CounterEnum::Dummy(c) => c.get(),
        }
    }
}

/// for single-threaded scenarios
pub type CounterEnumBorrow = Rc<RefCell<CounterEnum>>;
/// for multi-threaded scenarios
pub type CounterEnumLock = Arc<Mutex<CounterEnum>>;
