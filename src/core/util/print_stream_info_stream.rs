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
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::info_stream::{InfoStream, InfoStreamEnum};
use chrono::Utc;
use parking_lot::Mutex;
use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};

/// InfoStream implementation over a writable stream such as stdout.
pub struct PrintStreamInfoStream<W> {
  message_id: i32,
  pub(crate) stream: Mutex<W>,
  is_system_stream: bool,
}

static MESSAGE_ID: AtomicI32 = AtomicI32::new(0);

impl<W> PrintStreamInfoStream<W> {
  pub fn new(stream: W) -> Self {
    Self::with_message_id(stream, MESSAGE_ID.fetch_add(1, Ordering::SeqCst))
  }

  pub fn with_message_id(stream: W, message_id: i32) -> Self {
    Self {
      message_id,
      stream: Mutex::new(stream),
      is_system_stream: false,
    }
  }

  pub fn is_system_stream(&self) -> bool {
    self.is_system_stream
  }

  /// Returns the current time as string for insertion into log messages.
  pub fn get_timestamp(&self) -> String {
    Utc::now().to_rfc3339()
  }
}

impl PrintStreamInfoStream<io::Stdout> {
  pub fn stdout() -> Self {
    Self {
      message_id: MESSAGE_ID.fetch_add(1, Ordering::SeqCst),
      stream: Mutex::new(io::stdout()),
      is_system_stream: true,
    }
  }
}

impl PrintStreamInfoStream<io::Stderr> {
  pub fn stderr() -> Self {
    Self {
      message_id: MESSAGE_ID.fetch_add(1, Ordering::SeqCst),
      stream: Mutex::new(io::stderr()),
      is_system_stream: true,
    }
  }
}

impl<W> CloseableRef for PrintStreamInfoStream<W>
where
  W: Write + Send + 'static,
{
  fn close(&self) -> Result<()> {
    if !self.is_system_stream() {
      self.stream.lock().flush()?;
    }
    Ok(())
  }
}

impl<W> InfoStream for PrintStreamInfoStream<W>
where
  W: Write + Send + 'static,
{
  fn message(&self, component: &str, message: &str) -> Result<()> {
    let current_thread = std::thread::current();
    let thread_name = current_thread.name().unwrap_or("<unnamed>");
    let timestamp = self.get_timestamp();
    let mut stream = self.stream.lock();
    writeln!(
      stream,
      "{} {} [{}; {}]: {}",
      component, self.message_id, timestamp, thread_name, message
    )?;
    stream.flush()?;
    Ok(())
  }

  fn is_enabled(&self, _component: &str) -> bool {
    true
  }
}

impl<W> From<PrintStreamInfoStream<W>> for InfoStreamEnum
where
  W: Write + Send + 'static,
{
  fn from(info_stream: PrintStreamInfoStream<W>) -> Self {
    InfoStreamEnum::Custom(Box::new(info_stream))
  }
}
