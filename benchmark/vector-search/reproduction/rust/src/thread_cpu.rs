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
// macOS standalone consumer measurement only. No production unsafe code added.
use std::{io, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[repr(C)]
#[derive(Default)]
struct Basic {
  user_sec: i32,
  user_us: i32,
  sys_sec: i32,
  sys_us: i32,
  extra: [i32; 6],
}
unsafe extern "C" {
  static mach_task_self_: u32;
  fn task_threads(task: u32, list: *mut *mut u32, count: *mut u32) -> i32;
  fn thread_info(thread: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
  fn mach_port_deallocate(task: u32, port: u32) -> i32;
  fn vm_deallocate(task: u32, address: usize, size: usize) -> i32;
}
pub struct Snapshot {
  pub at: Instant,
  pub rows: Vec<(u64, u64)>,
}
impl Snapshot {
  pub fn capture() -> Result<Self> {
    let at = Instant::now();
    let mut ptr = std::ptr::null_mut();
    let mut count = 0;
    // Kernel initializes the returned port array on success; each acquired right and
    // the kernel-allocated array are released after collecting initialized records.
    let task = unsafe { mach_task_self_ };
    let code = unsafe { task_threads(task, &mut ptr, &mut count) };
    if code != 0 {
      return Err(io::Error::other(format!("task_threads {code}")).into());
    }
    let ports = unsafe { std::slice::from_raw_parts(ptr, count as usize) };
    let result = (|| {
      let mut rows = Vec::new();
      for &port in ports {
        let mut basic = Basic::default();
        let mut n = (std::mem::size_of::<Basic>() / 4) as u32;
        let code = unsafe { thread_info(port, 3, (&mut basic as *mut Basic).cast(), &mut n) };
        if code != 0 {
          return Err(io::Error::other(format!("basic thread_info {code}")).into());
        }
        let mut id = [0u64; 3];
        let mut n = 6;
        let code = unsafe { thread_info(port, 4, id.as_mut_ptr().cast(), &mut n) };
        if code != 0 {
          return Err(io::Error::other(format!("identifier thread_info {code}")).into());
        }
        let cpu = (i64::from(basic.user_sec) + i64::from(basic.sys_sec)) * 1_000_000_000
          + (i64::from(basic.user_us) + i64::from(basic.sys_us)) * 1000;
        rows.push((id[0], u64::try_from(cpu)?));
      }
      rows.sort_unstable_by_key(|x| x.0);
      Ok(Self { at, rows })
    })();
    for &port in ports {
      unsafe {
        mach_port_deallocate(task, port);
      }
    }
    unsafe {
      vm_deallocate(task, ptr as usize, count as usize * 4);
    }
    result
  }
  pub fn delta(&self, start: &Self) -> Option<u64> {
    if self.rows.len() != start.rows.len() {
      return None;
    }
    let mut total = 0;
    for (a, b) in self.rows.iter().zip(&start.rows) {
      if a.0 != b.0 || a.1 < b.1 {
        return None;
      }
      total += a.1 - b.1;
    }
    Some(total)
  }
  pub fn json(&self) -> String {
    format!(
      "[{}]",
      self
        .rows
        .iter()
        .map(|(id, cpu)| format!("[{id},{cpu}]"))
        .collect::<Vec<_>>()
        .join(",")
    )
  }
}
