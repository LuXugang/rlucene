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
use crate::core::index::BytesRef;
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, MultiTermQuerySet, RewriteMethod,
};
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{IntoQuery, Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{HasIdentity, StringHelper};
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::analysis::mock_tokenizer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::check_hits::CheckHits;
pub use crate::test_framework::core::search::multi_term::DumbPrefixQuery;
use crate::test_framework::core::util::DefaultIndexSearchCR;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_string_field, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};

/// Create an index with random unicode terms Generates random prefix queries,
/// and validates against a simple impl.
#[allow(dead_code)] // for quick search
pub struct TestPrefixRandom;

fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
where
  R: Rng + ?Sized,
{
  let dir = new_directory_shared(random)?;
  let mock = MockAnalyzer::with_automaton(random, mock_tokenizer::KEYWORD.clone(), false);
  let mut iwc = new_index_writer_config_with_analyzer(random, mock)?;
  iwc.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));

  let writer = RandomIndexWriter::with_config(random, dir, iwc);

  let mut field_to_type = HashMap::new();

  let mut doc = Document::new();

  let num = at_least(random, 1000);
  for _ in 0..num {
    let value = TestUtil::random_unicode_string_with_len(random, 10);
    doc.add(new_string_field(
      random,
      "field",
      &value,
      Store::No,
      &mut field_to_type,
    )?);
    writer.add_document(random, doc.clone())?;
    doc = Document::new();
  }

  let reader = writer.get_reader(random)?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close(random)?;
  Ok(searcher)
}

/// check that the # of hits is the same as from a very simple prefixquery implementation.
fn assert_same<IRC>(searcher: &IndexSearcher<IRC>, prefix: String) -> Result<()>
where
  IRC: IndexReaderContext + Sync,
{
  let smart = PrefixQuery::new(Term::from_text("field", prefix.clone()))?;
  let dumb = DumbPrefixQuery::new(Term::from_text("field", prefix));

  let smart_docs = searcher.search(smart.clone(), 25)?;
  let dumb_docs = searcher.search(dumb, 25)?;
  CheckHits::check_equal(
    &smart.into_query(),
    &smart_docs.score_docs,
    &dumb_docs.score_docs,
  )
}

#[test]
fn test_prefixes() -> Result<()> {
  let mut random = random();
  let searcher = set_up(&mut random)?;

  let num = at_least(&mut random, 100);
  for _ in 0..num {
    assert_same(
      &searcher,
      TestUtil::random_unicode_string_with_len(&mut random, 5),
    )?;
  }

  Ok(())
}
