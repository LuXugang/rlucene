/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::store::base_directory_wrapper::BaseDirectoryWrapper;
use crate::test::core::util::throttled_index_output::ThrottledIndexOutput;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};

pub(crate) enum Throttling {
  Always,
  Sometimes,
  Never,
}

pub(crate) trait Failure<D>: Send
where
  D: Directory,
{
  /// Called at each potential failure point.
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    Ok(())
  }

  fn reset(&mut self) -> &mut Self
  where
    Self: Sized,
  {
    self
  }

  fn do_fail_mut(&mut self) -> &mut bool;

  fn set_do_fail(&mut self) {
    *self.do_fail_mut() = true;
  }

  fn clear_do_fail(&mut self) {
    *self.do_fail_mut() = false;
  }
}

pub(crate) struct MockDirectoryWrapper<D>
where
  D: Directory,
{
  base: BaseDirectoryWrapper<D>,
  max_size: AtomicI64,

  // Max actual bytes used. This is set by MockIndexOutputWrapper.
  max_used_size: AtomicI64,
  random_io_exception_rate: Mutex<f64>,
  random_io_exception_rate_on_open: Mutex<f64>,
  random_state: Mutex<StdRng>,
  assert_no_delete_open_file: AtomicBool,
  track_disk_usage: AtomicBool,
  use_slow_open_closers: AtomicBool,
  allow_random_file_not_found_exception: AtomicBool,
  allow_reading_files_still_open_for_write: AtomicBool,
  unsynced_files: Mutex<HashSet<String>>,
  created_files: Mutex<HashSet<String>>,
  open_files_for_write: Mutex<HashSet<String>>,
  open_locks: Mutex<HashMap<String, String>>,
  crashed: AtomicBool,
  throttled_output: Mutex<Option<ThrottledIndexOutput<D::IndexOutput>>>,
  throttling: Mutex<Throttling>,

  // For testing.
  always_corrupt: AtomicBool,
  input_clone_count: AtomicI32,

  // The key will be an identity assigned to an open input/output handle. The
  // value stores diagnostic information captured when the handle was opened.
  open_file_handles: Mutex<HashMap<usize, String>>,
  open_files: Mutex<HashMap<String, i32>>,
  open_files_deleted: Mutex<HashSet<String>>,

  verbose_clone: AtomicBool,
  fail_on_create_output: AtomicBool,
  fail_on_open_input: AtomicBool,
  assert_no_unreferenced_files_on_close: AtomicBool,
  failures: Mutex<Vec<Box<dyn Failure<D>>>>,
}

impl<D> MockDirectoryWrapper<D>
where
  D: Directory,
{
  pub(crate) fn get_verbose_clone(&self) -> bool {
    self.verbose_clone.load(Ordering::SeqCst)
  }

  pub(crate) fn increment_input_clone_count(&self) {
    self.input_clone_count.fetch_add(1, Ordering::SeqCst);
  }

  pub(crate) fn remove_index_input(&self, _handle_id: usize, _name: &str) {
    todo!()
  }

  pub(crate) fn maybe_throw_deterministic_exception(&self) -> Result<()> {
    todo!()
  }
}
