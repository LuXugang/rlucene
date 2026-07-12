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
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::text_field::TextField;
use crate::core::index::directory_reader::{self, DirectoryReader};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::standard_directory_reader::StandardDirectoryReader;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::{DirEnum, Directory};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::RngExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread;

const VERBOSE: bool = false;

#[allow(dead_code)] // for quick search
struct TestIndexWriterNRTIsCurrent;

struct ReaderHolder {
  reader: RwLock<Option<Arc<StandardDirectoryReader<DirEnum>>>>,
  stop: AtomicBool,
}

impl ReaderHolder {
  fn new() -> Self {
    Self {
      reader: RwLock::new(None),
      stop: AtomicBool::new(false),
    }
  }
}

#[test]
fn test_is_current_with_threads() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = IndexWriter::new(dir.clone(), conf)?;
  let holder = Arc::new(ReaderHolder::new());
  let num_reader_threads = if is_night_mode() {
    TestUtil::next_int(&mut random, 2, 5) as usize
  } else {
    2
  };
  let mut threads = Vec::with_capacity(num_reader_threads);
  let latch = Arc::new(CountDownLatch::new(1));
  let writer_thread = WriterThread::new(
    holder.clone(),
    writer.clone(),
    at_least(&mut random, 500),
    &mut random,
    latch.clone(),
  );
  for _ in 0..num_reader_threads {
    let reader_thread = ReaderThread::new(holder.clone(), latch.clone());
    threads.push(thread::spawn(move || reader_thread.run()));
  }
  let writer_thread = thread::spawn(move || writer_thread.run());

  let mut failed = match writer_thread.join() {
    Ok(Ok(())) => false,
    Ok(Err(error)) => {
      eprintln!("{error:?}");
      true
    },
    Err(error) => {
      eprintln!("writer thread panicked: {error:?}");
      true
    },
  };
  for reader_thread in threads {
    match reader_thread.join() {
      Ok(Ok(())) => {},
      Ok(Err(error)) => {
        eprintln!("{error:?}");
        failed = true;
      },
      Err(error) => {
        eprintln!("reader thread panicked: {error:?}");
        failed = true;
      },
    }
  }
  assert!(!failed);
  writer.close()?;
  dir.close()?;
  Ok(())
}

struct WriterThread {
  holder: Arc<ReaderHolder>,
  writer: Arc<IndexWriter<DirEnum>>,
  num_ops: i32,
  countdown: bool,
  latch: Arc<CountDownLatch>,
}

impl WriterThread {
  fn new<R>(
    holder: Arc<ReaderHolder>,
    writer: Arc<IndexWriter<DirEnum>>,
    num_ops: i32,
    _random: &mut R,
    latch: Arc<CountDownLatch>,
  ) -> Self {
    Self {
      holder,
      writer,
      num_ops,
      countdown: true,
      latch,
    }
  }

  fn run(mut self) -> Result<()> {
    let mut current_reader: Option<Arc<StandardDirectoryReader<DirEnum>>> = None;
    let mut random = random();
    let result = (|| -> Result<()> {
      let mut doc = Document::new();
      doc.add(TextField::from_string("id", "1", Store::No)?);
      self.writer.add_document(doc.clone())?;
      current_reader = Some(Arc::new(directory_reader::open_from_writer(&self.writer)?));
      *self
        .holder
        .reader
        .write()
        .expect("reader holder lock poisoned") = current_reader.clone();
      for _ in 0..self.num_ops {
        if self.holder.stop.load(Ordering::SeqCst) {
          break;
        }
        let next_op = random.random::<f32>();
        if next_op < 0.3 {
          self
            .writer
            .update_document_with_term(Term::from_text("id", "1"), doc.clone())?;
        } else if next_op < 0.5 {
          self.writer.add_document(doc.clone())?;
        } else {
          self
            .writer
            .delete_documents_with_terms(vec![Term::from_text("id", "1")])?;
        }
        let current_reader_ref = current_reader
          .as_ref()
          .expect("current reader should be initialized");
        let mut holder_reader = self
          .holder
          .reader
          .write()
          .expect("reader holder lock poisoned");
        if !holder_reader
          .as_ref()
          .is_some_and(|reader| Arc::ptr_eq(reader, current_reader_ref))
        {
          *holder_reader = current_reader.clone();
          if self.countdown {
            self.countdown = false;
            self.latch.count_down();
          }
        }
        drop(holder_reader);
        if random.random_bool(0.5) {
          self.writer.commit()?;
          if let Some(new_reader) = directory_reader::open_if_changed(current_reader_ref)? {
            current_reader_ref.dec_ref()?;
            current_reader = Some(Arc::new(new_reader));
          }
          if current_reader
            .as_ref()
            .expect("current reader should be initialized")
            .num_docs()?
            == 0
          {
            self.writer.add_document(doc.clone())?;
          }
        }
      }
      Ok(())
    })();

    *self
      .holder
      .reader
      .write()
      .expect("reader holder lock poisoned") = None;
    if self.countdown {
      self.latch.count_down();
    }
    if let Some(current_reader) = current_reader {
      let _ = current_reader.dec_ref();
    }
    if VERBOSE {
      println!(
        "writer stopped - forced by reader: {}",
        self.holder.stop.load(Ordering::SeqCst)
      );
    }
    result
  }
}

struct ReaderThread {
  holder: Arc<ReaderHolder>,
  latch: Arc<CountDownLatch>,
}

impl ReaderThread {
  fn new(holder: Arc<ReaderHolder>, latch: Arc<CountDownLatch>) -> Self {
    Self { holder, latch }
  }

  fn run(self) -> Result<()> {
    self.latch.wait();
    let mut failed = None;
    loop {
      let reader = self
        .holder
        .reader
        .read()
        .expect("reader holder lock poisoned")
        .clone();
      let Some(reader) = reader else {
        break;
      };
      if reader.try_inc_ref() {
        let result = (|| -> Result<()> {
          let current = reader.is_current()?;
          if VERBOSE {
            println!(
              "Thread: {:?} Reader: {} isCurrent:{}",
              thread::current(),
              reader,
              current
            );
          }
          if current {
            return Err(LuceneError::illegal_state("reader must not be current"));
          }
          Ok(())
        })();
        if let Err(error) = result {
          if VERBOSE {
            println!(
              "FAILED Thread: {:?} Reader: {} isCurrent: false",
              thread::current(),
              reader
            );
          }
          self.holder.stop.store(true, Ordering::SeqCst);
          let _ = reader.dec_ref();
          return Err(error);
        }
        if let Err(error) = reader.dec_ref()
          && failed.is_none()
        {
          failed = Some(error);
        }
      }
    }
    match failed {
      Some(error) => Err(error),
      None => Ok(()),
    }
  }
}

struct CountDownLatch {
  count: Mutex<usize>,
  condvar: Condvar,
}

impl CountDownLatch {
  fn new(count: usize) -> Self {
    Self {
      count: Mutex::new(count),
      condvar: Condvar::new(),
    }
  }

  fn count_down(&self) {
    let mut count = self.count.lock().expect("latch mutex poisoned");
    if *count > 0 {
      *count -= 1;
      if *count == 0 {
        self.condvar.notify_all();
      }
    }
  }

  fn wait(&self) {
    let mut count = self.count.lock().expect("latch mutex poisoned");
    while *count > 0 {
      count = self.condvar.wait(count).expect("latch mutex poisoned");
    }
  }
}
