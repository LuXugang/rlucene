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
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::doc_values_rewrite_method::DocValuesRewriteMethod;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::{IntoQuery, Query};
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::util::automation::automaton_provider::DefaultProvider;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, is_light_mode, new_directory_shared, new_index_writer_config_with_analyzer,
  new_searcher_with_reader, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::sync::{Arc, LazyLock};
/// Tests the DocValuesRewriteMethod
#[allow(dead_code)] // for quick search
struct TestDocValuesRewriteMethod;

type TestContext = (String, DefaultIndexSearchCR);

static LIGHT_CONTEXT: LazyLock<Arc<TestContext>> = LazyLock::new(|| {
  let mut random = random();
  Arc::new(build_set_up(&mut random).expect("failed to initialize TestDocValuesRewriteMethod"))
});

fn set_up<R>(random: &mut R) -> Result<Arc<TestContext>>
where
  R: Rng + ?Sized,
{
  if is_light_mode() {
    return Ok(LIGHT_CONTEXT.clone());
  }

  Ok(Arc::new(build_set_up(random)?))
}

fn build_set_up<R>(random: &mut R) -> Result<TestContext>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;

  let field_name = if random.random_bool(0.5) {
    "field".to_string()
  } else {
    "".to_string()
  };

  let analyzer = MockAnalyzer::with_automaton(random, mock_tokenizer::KEYWORD.clone(), false);

  let mut iwc = new_index_writer_config_with_analyzer(random, analyzer)?;
  iwc.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));

  let writer = RandomIndexWriter::with_config(random, dir.clone(), iwc);

  let mut terms: Vec<String> = Vec::new();

  let num = at_least(random, 200);

  for i in 0..num {
    let mut doc = Document::new();

    doc.add(StringField::from_string("id", i.to_string(), Store::No)?);

    let num_terms = random.random_range(0..4);

    for _ in 0..num_terms {
      let s = TestUtil::random_unicode_string(random);

      doc.add(StringField::from_string(&field_name, s.clone(), Store::No)?);

      doc.add(SortedSetDocValuesField::new(
        &field_name,
        BytesRef::from_string(&s),
      ));

      doc.add(SortedSetDocValuesField::indexed_field(
        &(field_name.clone() + "_with-skip"),
        BytesRef::from_string(&s),
      ));

      terms.push(s);
    }

    writer.add_document(random, doc)?;
  }

  let num_deletions = random.random_range(0..(num / 10).max(1));

  for _ in 0..num_deletions {
    let id = random.random_range(0..num);
    writer.delete_documents_with_terms(random, vec![Term::from_text("id", id.to_string())])?;
  }

  let reader = writer.get_reader(random)?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close(random)?;

  Ok((field_name, searcher))
}

fn assert_same<IRC>(searcher: &IndexSearcher<IRC>, field_name: &str, regexp: String) -> Result<()>
where
  IRC: crate::core::index::index_reader_context::IndexReaderContext + Sync,
{
  let doc_values = RegexpQuery::with_all(
    Term::from_text(field_name, regexp.clone()),
    RegExp::NONE,
    0,
    &DefaultProvider,
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    DocValuesRewriteMethod,
  )?;
  let doc_values_with_skip = RegexpQuery::with_all(
    Term::from_text(&(field_name.to_string() + "_with-skip"), regexp.clone()),
    RegExp::NONE,
    0,
    &DefaultProvider,
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    DocValuesRewriteMethod,
  )?;
  let inverted = RegexpQuery::with_flags(Term::from_text(field_name, regexp), RegExp::NONE)?;

  let inverted_docs = searcher.search(inverted.clone(), 25)?;
  let doc_values_docs = searcher.search(doc_values, 25)?;
  let doc_values_with_skip_docs = searcher.search(doc_values_with_skip, 25)?;
  let inverted_query: Query = inverted.into_query();

  CheckHits::check_equal(
    &inverted_query,
    &inverted_docs.score_docs,
    &doc_values_docs.score_docs,
  )?;
  CheckHits::check_equal(
    &inverted_query,
    &inverted_docs.score_docs,
    &doc_values_with_skip_docs.score_docs,
  )
}

#[test]
fn test_regexps() -> Result<()> {
  let mut random = random();
  let context = set_up(&mut random)?;

  let num = at_least(&mut random, 1000);
  for _ in 0..num {
    let reg = AutomatonTestUtil::random_regexp(&mut random)?;
    assert_same(&context.1, &context.0, reg)?;
  }
  Ok(())
}

#[test]
fn test_equals() -> Result<()> {
  let mut random = random();
  let context = set_up(&mut random)?;
  let field_name = context.0.clone();

  {
    let a1 = RegexpQuery::with_flags(Term::from_text(&field_name, "[aA]"), RegExp::NONE)?;
    let a2 = RegexpQuery::with_flags(Term::from_text(&field_name, "[aA]"), RegExp::NONE)?;
    let b = RegexpQuery::with_flags(Term::from_text(&field_name, "[bB]"), RegExp::NONE)?;
    QueryUtils::check_equal(&a1, &a2);
    QueryUtils::check_unequal(&a1, &b);
  }

  {
    let a1 = RegexpQuery::with_all(
      Term::from_text(&field_name, "[aA]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    let a2 = RegexpQuery::with_all(
      Term::from_text(&field_name, "[aA]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    let b = RegexpQuery::with_all(
      Term::from_text(&field_name, "[bB]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    QueryUtils::check_from_query(&a1.into_query());
  }
  Ok(())
}
