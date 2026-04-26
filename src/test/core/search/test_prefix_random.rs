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
  ConstantScoreBlendedRewrite, MultiTermQuery, RewriteMethod,
};
use crate::core::search::prefix_query::PrefixQuery;
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{HasIdentity, StringHelper};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::check_hits::CheckHits;
use crate::test::core::util::DefaultIndexSearchCR;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
  new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
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
  // TODO IMPORTANT 要使用MockAnalyzer带分词器
  let mock = MockAnalyzer::new(random);
  let mut iwc = new_index_writer_config_with_analyzer(random, mock);
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
    writer.add_document(doc.clone())?;
    doc = Document::new();
  }

  let reader = writer.get_reader()?;
  let searcher = new_searcher_with_reader(reader)?;

  writer.close()?;
  Ok(searcher)
}

/// check that the # of hits is the same as from a very simple prefixquery implementation.
fn assert_same<IRC>(searcher: &IndexSearcher<IRC>, prefix: String) -> Result<()>
where
  IRC: IndexReaderContext,
{
  let smart = PrefixQuery::new(Term::from_text("field", prefix.clone()))?;
  let dumb = DumbPrefixQuery::new(Term::from_text("field", prefix));

  let smart_docs = searcher.search(smart.clone(), 25)?;
  let dumb_docs = searcher.search(dumb, 25)?;
  CheckHits::check_equal(&smart.into(), &smart_docs.score_docs, &dumb_docs.score_docs)
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

/// A simple prefix query that scans through all terms.
#[derive(Clone)]
pub struct DumbPrefixQuery {
  field: String,
  prefix: BytesRef<Vec<u8>>,
  id: Identity,
}

impl DumbPrefixQuery {
  pub fn new(term: Term) -> Self {
    Self {
      field: term.field().to_string(),
      prefix: term.bytes().clone(),
      id: Identity::default(),
    }
  }
}

impl QueryBase for DumbPrefixQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    if self.field == field {
      Ok(format!("{}", self.prefix))
    } else {
      Ok(format!("{}:{}", self.field, self.prefix))
    }
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    ConstantScoreBlendedRewrite.rewrite(searcher, self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
  }
}

impl Debug for DumbPrefixQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.as_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for DumbPrefixQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for DumbPrefixQuery {
  fn get_field(&self) -> &str {
    &self.field
  }

  type TermsEnum<T>
    = FilteredTermsEnum<T::TermsEnum, SimplePrefixTermsEnum>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    let mut terms_enum = FilteredTermsEnum::new(
      terms.iterator()?,
      SimplePrefixTermsEnum {
        prefix: self.prefix.clone(),
      },
    );
    terms_enum.set_initial_seek_term(BytesRef::from(""));
    Ok(terms_enum)
  }

  fn as_query(&self) -> Query {
    self.clone().into()
  }
}

impl Eq for DumbPrefixQuery {}

impl PartialEq for DumbPrefixQuery {
  fn eq(&self, other: &Self) -> bool {
    self.field == other.field && self.prefix == other.prefix
  }
}

impl Hash for DumbPrefixQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.field.hash(state);
    self.prefix.hash(state);
  }
}

pub struct SimplePrefixTermsEnum {
  prefix: BytesRef<Vec<u8>>,
}

impl FilteredTermsEnumBase for SimplePrefixTermsEnum {
  fn accept(&mut self, term: &BytesRef<Vec<u8>>, _ord: i64) -> Result<AcceptStatus> {
    if StringHelper::starts_with_byte_ref(term, &self.prefix) {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::No)
    }
  }
}
