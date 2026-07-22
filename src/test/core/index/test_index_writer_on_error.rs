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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::document::int_point::IntPoint;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::text_field::TYPE_NOT_STORED;
use crate::core::index::bytes_ref::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::index_file_deleter::IndexFileDeleter;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::{Directory, MaybeNrtDirEnum, MockDirWrapper};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::analysis::mock_variable_length_payload_filter::MockVariableLengthPayloadFilter;
use crate::test_framework::core::store::mock_directory_wrapper::{
  Failure, MockDirectoryWrapper, Throttling,
};
use crate::test_framework::core::util::lucene_test_case::{
  at_least, call_stack_contains, call_stack_contains_type, is_night_mode, new_field,
  new_index_writer_config_with_analyzer, new_mock_directory, new_string_field, new_text_field,
  random, random_from_seed,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Write;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

/// Causes a bunch of fake VM errors and checks that no other exceptions are delivered instead, no
/// index corruption is ever created.
#[allow(dead_code)] // for quick search
struct TestIndexWriterOnError;

type TestDirectory = MockDirWrapper;

struct OnErrorAnalyzer {
  analyzer_seed: u64,
  stored_value: AnalyzerStoredValue,
}

impl OnErrorAnalyzer {
  fn new(analyzer_seed: u64) -> Self {
    Self {
      analyzer_seed,
      stored_value: AnalyzerStoredValue::per_field(),
    }
  }
}

impl Analyzer for OnErrorAnalyzer {
  fn create_components(&self, field_name: &str) -> Result<TokenStreamComponents> {
    let mut tokenizer = MockTokenizer::with_default_max_token_length(
      random_from_seed(self.analyzer_seed),
      WHITESPACE.clone(),
      false,
    );
    tokenizer.set_enable_checks(false); // we are going to make it angry
    // emit some payloads
    if field_name.contains("payloads") {
      let stream =
        MockVariableLengthPayloadFilter::new(tokenizer, random_from_seed(self.analyzer_seed));
      Ok(TokenStreamComponents::new(
        Box::new(stream) as Box<dyn TokenStream + Send + Sync>,
        None,
      ))
    } else {
      Ok(TokenStreamComponents::new(
        Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
        None,
      ))
    }
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(OnErrorAnalyzer);

enum Disaster {
  Error(LuceneError),
  Panic(Box<dyn Any + Send>),
}

// just one thread, serial merge policy, hopefully debuggable
fn do_test<F>(random: &mut StdRng, fail_on: F) -> Result<()>
where
  F: Clone + Send + 'static,
  F: Failure<MaybeNrtDirEnum>,
{
  // log all exceptions we hit, in case we fail (for debugging)
  let mut exception_log = String::new();

  let analyzer_seed = random.random();
  let mut dir: Option<Arc<TestDirectory>> = None;
  let mut field_types = HashMap::new();

  let num_iterations = if is_night_mode() {
    at_least(random, 100)
  } else {
    at_least(random, 5)
  };

  'start_over: for _ in 0..num_iterations {
    // close from last run
    if let Some(previous_dir) = dir.take() {
      previous_dir.as_ref().close()?;
    }
    // disable slow things: we don't rely upon sleeps here.
    let new_dir = Arc::new(new_mock_directory(random)?);
    new_dir.set_throttling(Throttling::Never);
    new_dir.set_use_slow_open_closers(false);
    dir = Some(new_dir.clone());

    let analyzer = Box::new(OnErrorAnalyzer::new(analyzer_seed)) as Box<dyn Analyzer>;
    let mut conf = new_index_writer_config_with_analyzer(random, analyzer)?;
    // just for now, try to keep this test reproducible
    conf.set_merge_scheduler(SerialMergeScheduler::new());

    // test never makes it this far...
    let num_docs = at_least(random, 2000);

    let writer = IndexWriter::new(new_dir.clone(), conf)?;
    writer.commit()?; // ensure there is always a commit

    new_dir.fail_on(Box::new(fail_on.clone()));

    for i in 0..num_docs {
      let mut doc = Document::new();
      doc.add(new_string_field(
        random,
        "id",
        i.to_string(),
        Store::No,
        &mut field_types,
      )?);
      doc.add(NumericDocValuesField::new("dv", i as i64));
      doc.add(BinaryDocValuesField::new(
        "dv2",
        BytesRef::from_string(&i.to_string()),
      ));
      doc.add(SortedDocValuesField::new(
        "dv3",
        BytesRef::from_string(&i.to_string()),
      ));
      doc.add(SortedSetDocValuesField::new(
        "dv4",
        BytesRef::from_string(&i.to_string()),
      ));
      doc.add(SortedSetDocValuesField::new(
        "dv4",
        BytesRef::from_string(&(i - 1).to_string()),
      ));
      doc.add(SortedNumericDocValuesField::new("dv5", i as i64));
      doc.add(SortedNumericDocValuesField::new("dv5", (i - 1) as i64));
      let text1 = TestUtil::random_analysis_string(random, 20, true);
      doc.add(new_text_field(
        random,
        "text1",
        text1,
        Store::No,
        &mut field_types,
      )?);
      // ensure we store something
      doc.add(StoredField::from_string("stored1", "foo")?);
      doc.add(StoredField::from_string("stored1", "bar")?);
      // ensure we get some payloads
      let text_payloads = TestUtil::random_analysis_string(random, 6, true);
      doc.add(new_text_field(
        random,
        "text_payloads",
        text_payloads,
        Store::No,
        &mut field_types,
      )?);
      // ensure we get some vectors
      let mut ft = FieldType::from_ref(&*TYPE_NOT_STORED)?;
      ft.set_store_term_vectors(true)?;
      let text_vectors = TestUtil::random_analysis_string(random, 6, true);
      doc.add(new_field(
        random,
        "text_vectors",
        text_vectors,
        &ft,
        &mut field_types,
      )?);
      doc.add(IntPoint::new("point", [random.random::<i32>()])?);
      doc.add(IntPoint::new(
        "point2d",
        [random.random::<i32>(), random.random::<i32>()],
      )?);

      if random.random_range(0..10) > 0 {
        // single doc
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
          writer.add_document(doc)?;
          // we made it, sometimes delete our doc, or update a dv
          match random.random_range(0..4) {
            0 => {
              writer.delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])?;
            },
            1 => {
              writer.update_numeric_doc_value(
                Term::from_text("id", i.to_string()),
                "dv",
                (i + 1) as i64,
              )?;
            },
            2 => {
              writer.update_binary_doc_value(
                Term::from_text("id", i.to_string()),
                "dv2",
                BytesRef::from_string(&(i + 1).to_string()),
              )?;
            },
            _ => {},
          }
          Ok(())
        }));
        match result {
          Ok(Ok(())) => {},
          Ok(Err(error)) => {
            get_tragedy(Disaster::Error(error), &writer, &mut exception_log)?;
            continue 'start_over;
          },
          Err(payload) => {
            get_tragedy(Disaster::Panic(payload), &writer, &mut exception_log)?;
            continue 'start_over;
          },
        }
      } else {
        // block docs
        let mut doc2 = Document::new();
        doc2.add(new_string_field(
          random,
          "id",
          (-i).to_string(),
          Store::No,
          &mut field_types,
        )?);
        let text1 = TestUtil::random_analysis_string(random, 20, true);
        doc2.add(new_text_field(
          random,
          "text1",
          text1,
          Store::No,
          &mut field_types,
        )?);
        doc2.add(StoredField::from_string("stored1", "foo")?);
        doc2.add(StoredField::from_string("stored1", "bar")?);
        let text_vectors = TestUtil::random_analysis_string(random, 6, true);
        doc2.add(new_field(
          random,
          "text_vectors",
          text_vectors,
          &ft,
          &mut field_types,
        )?);

        let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
          writer.add_documents(vec![doc, doc2])?;
          // we made it, sometimes delete our docs
          if random.random_bool(0.5) {
            writer.delete_documents_with_terms(vec![
              Term::from_text("id", i.to_string()),
              Term::from_text("id", (-i).to_string()),
            ])?;
          }
          Ok(())
        }));
        match result {
          Ok(Ok(())) => {},
          Ok(Err(error)) => {
            get_tragedy(Disaster::Error(error), &writer, &mut exception_log)?;
            continue 'start_over;
          },
          Err(payload) => {
            get_tragedy(Disaster::Panic(payload), &writer, &mut exception_log)?;
            continue 'start_over;
          },
        }
      }

      if random.random_range(0..10) == 0 {
        // trigger flush:
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
          if random.random_bool(0.5) {
            let reader =
              directory_reader::open_with_writer_deletes(&writer, random.random_bool(0.5), false)?;
            let check_result = TestUtil::check_reader(&reader);
            let close_result = reader.close();
            check_result?;
            close_result?;
          } else {
            writer.commit()?;
          }
          if directory_reader::index_exists(new_dir.as_ref())? {
            TestUtil::check_index(random, new_dir.as_ref())?;
          }
          Ok(())
        }));
        match result {
          Ok(Ok(())) => {},
          Ok(Err(error)) => {
            get_tragedy(Disaster::Error(error), &writer, &mut exception_log)?;
            continue 'start_over;
          },
          Err(payload) => {
            get_tragedy(Disaster::Panic(payload), &writer, &mut exception_log)?;
            continue 'start_over;
          },
        }
      }
    }

    let result = catch_unwind(AssertUnwindSafe(|| writer.close()));
    match result {
      Ok(Ok(())) => {},
      Ok(Err(error)) => {
        get_tragedy(Disaster::Error(error), &writer, &mut exception_log)?;
        continue 'start_over;
      },
      Err(payload) => {
        get_tragedy(Disaster::Panic(payload), &writer, &mut exception_log)?;
        continue 'start_over;
      },
    }
  }

  if let Some(dir) = dir {
    dir.as_ref().close()?;
  }
  Ok(())
}

fn get_tragedy<D>(disaster: Disaster, writer: &Arc<IndexWriter<D>>, log: &mut String) -> Result<()>
where
  D: Directory + 'static,
{
  let message = match &disaster {
    Disaster::Error(error) => error.to_string(),
    Disaster::Panic(payload) => LuceneError::panic_payload_message(payload.as_ref()),
  };

  if message.contains("Fake") {
    let _ = writeln!(log, "\nTEST: got expected fake exc:{message}");
    // TODO: remove rollback here, and add this assert to ensure "full OOM protection" anywhere IW
    // does writes
    // assert!(writer.is_open() == false, "hit OOM but writer is still open, WTF: ");
    let rollback_result = catch_unwind(AssertUnwindSafe(|| writer.rollback()));
    match rollback_result {
      Ok(Ok(())) => {},
      Ok(Err(error)) => {
        let _ = writeln!(log, "{error:?}");
      },
      Err(payload) => {
        let _ = writeln!(
          log,
          "{}",
          LuceneError::panic_payload_message(payload.as_ref())
        );
      },
    }
    Ok(())
  } else {
    match disaster {
      Disaster::Error(error) => Err(error),
      Disaster::Panic(payload) => resume_unwind(payload),
    }
  }
}

#[derive(Clone)]
struct OomFailure {
  random: Arc<Mutex<StdRng>>,
}

impl OomFailure {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
    }
  }
}

impl<D> Failure<D> for OomFailure
where
  D: Directory + 'static,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.random.lock().random_range(0..3000) == 0
      && call_stack_contains_type::<IndexWriter<MockDirectoryWrapper<D>>>()
      && !std::thread::panicking()
    {
      panic!("Fake OutOfMemoryError");
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {}

  fn clear_do_fail(&mut self) {}
}

#[test]
fn test_oom() -> Result<()> {
  let mut random = random();
  let failure = OomFailure::new(random.random());
  do_test(&mut random, failure)
}

#[derive(Clone)]
struct UnknownErrorFailure {
  random: Arc<Mutex<StdRng>>,
}

impl UnknownErrorFailure {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
    }
  }
}

impl<D> Failure<D> for UnknownErrorFailure
where
  D: Directory + 'static,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.random.lock().random_range(0..3000) == 0
      && call_stack_contains_type::<IndexWriter<MockDirectoryWrapper<D>>>()
      && !std::thread::panicking()
    {
      panic!("Fake UnknownError");
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {}

  fn clear_do_fail(&mut self) {}
}

#[test]
fn test_unknown_error() -> Result<()> {
  let mut random = random();
  let failure = UnknownErrorFailure::new(random.random());
  do_test(&mut random, failure)
}

#[derive(Clone)]
struct LinkageErrorFailure {
  random: Arc<Mutex<StdRng>>,
}

impl LinkageErrorFailure {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
    }
  }
}

impl<D> Failure<D> for LinkageErrorFailure
where
  D: Directory + 'static,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.random.lock().random_range(0..3000) == 0
      && call_stack_contains_type::<IndexWriter<MockDirectoryWrapper<D>>>()
      && !std::thread::panicking()
    {
      panic!("Fake LinkageError");
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {}

  fn clear_do_fail(&mut self) {}
}

#[test]
fn test_linkage_error() -> Result<()> {
  let mut random = random();
  let failure = LinkageErrorFailure::new(random.random());
  do_test(&mut random, failure)
}

#[derive(Clone)]
struct IoErrorFailure {
  random: Arc<Mutex<StdRng>>,
}

impl IoErrorFailure {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
    }
  }
}

impl<D> Failure<D> for IoErrorFailure
where
  D: Directory + 'static,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.random.lock().random_range(0..3000) == 0
      && call_stack_contains_type::<IndexWriter<MockDirectoryWrapper<D>>>()
      && !std::thread::panicking()
    {
      panic!("Fake IOError");
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {}

  fn clear_do_fail(&mut self) {}
}

#[test]
fn test_io_error() -> Result<()> {
  let mut random = random();
  let failure = IoErrorFailure::new(random.random());
  do_test(&mut random, failure)
}

#[derive(Clone)]
struct CheckpointFailure {
  random: Arc<Mutex<StdRng>>,
}

impl CheckpointFailure {
  fn new(seed: u64) -> Self {
    Self {
      random: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
    }
  }
}

impl<D> Failure<D> for CheckpointFailure
where
  D: Directory + 'static,
{
  fn eval(&mut self, _dir: &MockDirectoryWrapper<D>) -> Result<()> {
    if self.random.lock().random_range(0..4) == 0
      && call_stack_contains::<IndexFileDeleter<MockDirectoryWrapper<D>>>("checkpoint")
      && !std::thread::panicking()
    {
      panic!("Fake OutOfMemoryError");
    }
    Ok(())
  }

  fn set_do_fail(&mut self) {}

  fn clear_do_fail(&mut self) {}
}

#[cfg(feature = "nightly")]
#[test]
#[ignore = "nightly"]
fn test_checkpoint() -> Result<()> {
  let mut random = random();
  let failure = CheckpointFailure::new(random.random());
  do_test(&mut random, failure)
}
