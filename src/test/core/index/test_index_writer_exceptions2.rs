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
use crate::core::codecs::codec::Codecs;
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
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::serial_merge_scheduler::SerialMergeScheduler;
use crate::core::index::term::Term;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::{Directory, MockDirWrapper};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test_framework::core::analysis::cranky_token_filter::CrankyTokenFilter;
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, SIMPLE};
use crate::test_framework::core::analysis::mock_variable_length_payload_filter::MockVariableLengthPayloadFilter;
use crate::test_framework::core::codecs::asserting_codec::AssertingCodec;
use crate::test_framework::core::codecs::cranky::cranky_codec::CrankyCodec;
use crate::test_framework::core::store::mock_directory_wrapper::{
  MockDirectoryWrapper, Throttling,
};
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_field, new_index_writer_config_with_analyzer, new_mock_directory, new_string_field,
  new_text_field, random, random_from_seed, random_multiplier,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::collections::HashMap;
use std::fmt::Write;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

/**
 * Causes a bunch of non-aborting and aborting exceptions and checks that no index corruption is
 * ever created.
 */
#[allow(dead_code)] // for quick search
struct TestIndexWriterExceptions2;

type TestDirectory = MockDirWrapper;

struct Exceptions2Analyzer {
  analyzer_seed: u64,
  stored_value: AnalyzerStoredValue,
}

impl Exceptions2Analyzer {
  fn new(analyzer_seed: u64) -> Self {
    Self {
      analyzer_seed,
      stored_value: AnalyzerStoredValue::global(),
    }
  }
}

impl Analyzer for Exceptions2Analyzer {
  fn create_components(&self, field_name: &str) -> Result<TokenStreamComponents> {
    let mut tokenizer = MockTokenizer::with_default_max_token_length(
      random_from_seed(self.analyzer_seed),
      SIMPLE.clone(),
      false,
    );
    // TODO: can we turn this on? our filter is probably too evil
    tokenizer.set_enable_checks(false);
    let stream: Box<dyn TokenStream + Send + Sync> = if field_name.contains("payloads") {
      // emit some payloads
      Box::new(MockVariableLengthPayloadFilter::new(
        Arc::new(Mutex::new(random_from_seed(self.analyzer_seed))),
        tokenizer,
      ))
    } else {
      Box::new(tokenizer)
    };
    let stream = CrankyTokenFilter::new(
      stream,
      Arc::new(Mutex::new(random_from_seed(self.analyzer_seed))),
    );
    Ok(TokenStreamComponents::new(
      Box::new(stream) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(Exceptions2Analyzer);

// just one thread, serial merge policy, hopefully debuggable
#[test]
fn test_basics() -> Result<()> {
  let mut random = random();

  // disable slow things: we don't rely upon sleeps here.
  let dir: Arc<TestDirectory> = Arc::new(new_mock_directory(&mut random)?);
  dir.set_throttling(Throttling::Never);
  dir.set_use_slow_open_closers(false);

  // log all exceptions we hit, in case we fail (for debugging)
  let mut exception_log = String::new();

  // create lots of non-aborting exceptions with a broken analyzer
  let analyzer_seed = random.random();
  let analyzer = Arc::new(Exceptions2Analyzer::new(analyzer_seed));

  // create lots of aborting exceptions with a broken codec
  // we don't need a random codec, as we aren't trying to find bugs in the codec here.
  let codec_seed = random.random();
  let codec: Codecs = if random_multiplier() > 1 {
    CrankyCodec::new(TestUtil::get_default_codec(), random_from_seed(codec_seed)).into()
  } else {
    CrankyCodec::new(AssertingCodec::new(), random_from_seed(codec_seed)).into()
  };

  let shared_analyzer = Box::new(Arc::clone(&analyzer)) as Box<dyn Analyzer>;
  let mut conf = new_index_writer_config_with_analyzer(&mut random, shared_analyzer)?;
  // just for now, try to keep this test reproducible
  conf.set_merge_scheduler(SerialMergeScheduler::new());
  conf.set_codec(codec.clone());

  let num_docs = at_least(&mut random, 100);

  let mut writer = IndexWriter::new(Arc::clone(&dir), conf)?;
  let result = catch_unwind(AssertUnwindSafe(|| -> Result<()> {
    let mut allow_already_closed = false;
    let mut field_types = HashMap::new();
    for i in 0..num_docs {
      // TODO: add crankyDocValuesFields, etc
      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
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
      let text1 = TestUtil::random_analysis_string(&mut random, 20, true);
      doc.add(new_text_field(
        &mut random,
        "text1",
        text1,
        Store::No,
        &mut field_types,
      )?);
      // ensure we store something
      doc.add(StoredField::from_string("stored1", "foo")?);
      doc.add(StoredField::from_string("stored1", "bar")?);
      // ensure we get some payloads
      let text_payloads = TestUtil::random_analysis_string(&mut random, 6, true);
      doc.add(new_text_field(
        &mut random,
        "text_payloads",
        text_payloads,
        Store::No,
        &mut field_types,
      )?);
      // ensure we get some vectors
      let mut field_type = FieldType::from_ref(&*TYPE_NOT_STORED)?;
      field_type.set_store_term_vectors(true)?;
      let text_vectors = TestUtil::random_analysis_string(&mut random, 6, true);
      doc.add(new_field(
        &mut random,
        "text_vectors",
        text_vectors,
        &field_type,
        &mut field_types,
      )?);
      doc.add(IntPoint::new("point", [random.random::<i32>()])?);
      doc.add(IntPoint::new(
        "point2d",
        [random.random::<i32>(), random.random::<i32>()],
      )?);

      if random.random_range(0..10) > 0 {
        // single doc
        let operation = writer.add_document(doc).and_then(|_| {
          // we made it, sometimes delete our doc, or update a dv
          match random.random_range(0..4) {
            0 => writer
              .delete_documents_with_terms(vec![Term::from_text("id", i.to_string())])
              .map(|_| ()),
            1 => writer
              .update_numeric_doc_value(Term::from_text("id", i.to_string()), "dv", (i + 1) as i64)
              .map(|_| ()),
            2 => writer
              .update_binary_doc_value(
                Term::from_text("id", i.to_string()),
                "dv2",
                BytesRef::from_string(&(i + 1).to_string()),
              )
              .map(|_| ()),
            _ => Ok(()),
          }
        });
        match operation {
          Ok(()) => {},
          Err(LuceneError::AlreadyClosed(_)) => {
            // OK: writer was closed by abort; we just reopen now:
            assert!(writer.is_deleter_closed()?);
            assert!(allow_already_closed);
            allow_already_closed = false;
            let shared_analyzer = Box::new(Arc::clone(&analyzer)) as Box<dyn Analyzer>;
            let mut conf = new_index_writer_config_with_analyzer(&mut random, shared_analyzer)?;
            // just for now, try to keep this test reproducible
            conf.set_merge_scheduler(SerialMergeScheduler::new());
            conf.set_codec(codec.clone());
            writer = IndexWriter::new(Arc::clone(&dir), conf)?;
          },
          Err(error) => {
            let message = match &error {
              LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
                source.to_string()
              },
              _ => error.to_string(),
            };
            if message.starts_with("Fake IOException") {
              let _ = writeln!(
                exception_log,
                "\nTEST: got expected fake exc:{message}\n{error:?}"
              );
              allow_already_closed = true;
            } else {
              return Err(error);
            }
          },
        }
      } else {
        // block docs
        let mut doc2 = Document::new();
        doc2.add(new_string_field(
          &mut random,
          "id",
          (-i).to_string(),
          Store::No,
          &mut field_types,
        )?);
        let text1 = TestUtil::random_analysis_string(&mut random, 20, true);
        doc2.add(new_text_field(
          &mut random,
          "text1",
          text1,
          Store::No,
          &mut field_types,
        )?);
        doc2.add(StoredField::from_string("stored1", "foo")?);
        doc2.add(StoredField::from_string("stored1", "bar")?);
        let text_vectors = TestUtil::random_analysis_string(&mut random, 6, true);
        doc2.add(new_field(
          &mut random,
          "text_vectors",
          text_vectors,
          &field_type,
          &mut field_types,
        )?);

        let operation = writer.add_documents(vec![doc, doc2]).and_then(|_| {
          // we made it, sometimes delete our docs
          if random.random_bool(0.5) {
            writer
              .delete_documents_with_terms(vec![
                Term::from_text("id", i.to_string()),
                Term::from_text("id", (-i).to_string()),
              ])
              .map(|_| ())
          } else {
            Ok(())
          }
        });
        match operation {
          Ok(()) => {},
          Err(LuceneError::AlreadyClosed(_)) => {
            // OK: writer was closed by abort; we just reopen now:
            assert!(writer.is_deleter_closed()?);
            assert!(allow_already_closed);
            allow_already_closed = false;
            let shared_analyzer = Box::new(Arc::clone(&analyzer)) as Box<dyn Analyzer>;
            let mut conf = new_index_writer_config_with_analyzer(&mut random, shared_analyzer)?;
            // just for now, try to keep this test reproducible
            conf.set_merge_scheduler(SerialMergeScheduler::new());
            conf.set_codec(codec.clone());
            writer = IndexWriter::new(Arc::clone(&dir), conf)?;
          },
          Err(error) => {
            let message = match &error {
              LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
                source.to_string()
              },
              _ => error.to_string(),
            };
            if message.starts_with("Fake IOException") {
              let _ = writeln!(
                exception_log,
                "\nTEST: got expected fake exc:{message}\n{error:?}"
              );
              allow_already_closed = true;
            } else {
              return Err(error);
            }
          },
        }
      }

      if random.random_range(0..10) == 0 {
        // trigger flush:
        let flush_result = (|| -> Result<()> {
          if random.random_bool(0.5) {
            let reader =
              directory_reader::open_with_writer_deletes(&writer, random.random_bool(0.5), false)?;
            let check_result = TestUtil::check_reader(&reader);
            // Java's closeWhileHandlingException swallows close failures here.
            let _ = reader.close();
            check_result?;
          } else {
            writer.commit()?;
          }
          if directory_reader::index_exists(dir.as_ref())? {
            TestUtil::check_index(&mut random, dir.as_ref())?;
          }
          Ok(())
        })();
        match flush_result {
          Ok(()) => {},
          Err(LuceneError::AlreadyClosed(_)) => {
            // OK: writer was closed by abort; we just reopen now:
            assert!(writer.is_deleter_closed()?);
            assert!(allow_already_closed);
            allow_already_closed = false;
            let shared_analyzer = Box::new(Arc::clone(&analyzer)) as Box<dyn Analyzer>;
            let mut conf = new_index_writer_config_with_analyzer(&mut random, shared_analyzer)?;
            // just for now, try to keep this test reproducible
            conf.set_merge_scheduler(SerialMergeScheduler::new());
            conf.set_codec(codec.clone());
            writer = IndexWriter::new(Arc::clone(&dir), conf)?;
          },
          Err(error) => {
            let message = match &error {
              LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
                source.to_string()
              },
              _ => error.to_string(),
            };
            if message.starts_with("Fake IOException") {
              let _ = writeln!(
                exception_log,
                "\nTEST: got expected fake exc:{message}\n{error:?}"
              );
              allow_already_closed = true;
            } else {
              return Err(error);
            }
          },
        }
      }
    }

    match writer.close() {
      Ok(()) => {},
      Err(error) => {
        let message = match &error {
          LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
            source.to_string()
          },
          _ => error.to_string(),
        };
        if message.starts_with("Fake IOException") {
          let _ = writeln!(
            exception_log,
            "\nTEST: got expected fake exc:{message}\n{error:?}"
          );
          let _ = catch_unwind(AssertUnwindSafe(|| writer.rollback()));
        } else {
          return Err(error);
        }
      },
    }
    dir.as_ref().close()
  }));

  match result {
    Ok(Ok(())) => {},
    Ok(Err(error)) => {
      println!("Unexpected exception: dumping fake-exception-log:...");
      println!("{exception_log}");
      return Err(error);
    },
    Err(payload) => {
      println!("Unexpected exception: dumping fake-exception-log:...");
      println!("{exception_log}");
      resume_unwind(payload);
    },
  }

  if std::env::var("tests.verbose").is_ok_and(|value| value == "true") {
    println!("TEST PASSED: dumping fake-exception-log:...");
    println!("{exception_log}");
  }
  Ok(())
}
