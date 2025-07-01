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
use crate::store::directory::Directory;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct FileDeleter<D>
where
    D: Directory,
{
    directory: Arc<Mutex<D>>,
}

/// Tracks the reference count for a single index file:
pub struct RefCount {
    // fileName used only for better assert error messages
    file_name: String,
    init_done: bool,
    count: usize,
}
impl RefCount {
    pub fn new(file_name: &str) -> Self {
        Self {
            file_name: file_name.to_string(),
            init_done: false,
            count: 0,
        }
    }

    pub fn inc_ref(&mut self) -> usize {
        if !self.init_done {
            self.init_done = true;
        } else {
            debug_assert!(
                self.count > 0,
                "{}: RefCount is 0 pre-increment for file `{}`",
                std::thread::current()
                    .name()
                    .unwrap_or("Thread name is None"),
                self.file_name
            );
        }
        self.count.saturating_add(1)
    }

    pub fn dec_ref(&mut self) -> usize {
        debug_assert!(
            self.count > 0,
            "{}: RefCount is 0 pre-increment for file `{}`",
            std::thread::current()
                .name()
                .unwrap_or("Thread name is None"),
            self.file_name
        );
        self.count.saturating_sub(1)
    }
}
