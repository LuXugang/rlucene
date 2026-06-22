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
use crate::core::document::field::{FieldBase, Store};
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::phrase_query::PhraseQuery;
use crate::core::search::query::{IntoQuery, Query};
use crate::core::search::score_doc::ScoreDocLike;
use crate::core::search::sort::Sort;
use crate::core::search::term_range_query::TermRangeQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::analysis::mock_tokenizer::WHITESPACE;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::search::query_utils::QueryUtils;
use crate::test::core::util::DefaultIndexSearchCRShared;
use crate::test::core::util::lucene_test_case::{
  at_least, is_night_mode, new_directory_shared, new_searcher_with_reader,
};
use crate::test::core::util::test_util::TestUtil;
use rand::{Rng, RngExt};
use std::sync::Arc;
/// Base test support for checking search equivalence. Implement it and write tests that create [`
/// random_term()`]s (all terms are single characters a-z), and use [`assert_same_set(Query,
/// Query)`] and [`assert_subset_of(Query, Query)`]
pub trait SearchEquivalenceTestBase {
  fn get_meta(&self) -> &SearchEquivalenceTestBaseMeta;
  /// Returns a term suitable for searching. Terms are single characters in lowercase (`a-z`).
  fn random_term<R>(&self, random: &mut R) -> Term
  where
    R: Rng + ?Sized,
  {
    Term::from_text("field", random_char(random).to_string())
  }
  /// Asserts that the documents returned by q1 are the same as of those returned by q2
  fn assert_same_set<R>(&self, random: &mut R, q1: &Query, q2: &Query) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.assert_subset_of(random, q1, q2)?;
    self.assert_subset_of(random, q2, q1)
  }

  /// Asserts that the documents returned by q1 are a subset of those returned by q2
  fn assert_subset_of<R>(&self, random: &mut R, q1: &Query, q2: &Query) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.assert_subset_of_with_filter(q1, q2, None)?;

    let num_filters = if is_night_mode() {
      at_least(random, 10)
    } else {
      at_least(random, 3)
    };
    for _ in 0..num_filters {
      let filter = random_filter(random)?;
      self.assert_subset_of_with_filter(q1, q2, Some(filter.clone()))?;
      self.assert_subset_of_with_filter(
        &filtered_query(q1, &filter)?,
        &filtered_query(q2, &filter)?,
        None,
      )?;
    }

    Ok(())
  }

  /**
   * Asserts that the documents returned by `q1` are a subset of those returned by `q2`.
   *
   * Both queries will be filtered by `filter`.
   */
  fn assert_subset_of_with_filter(
    &self,
    q1: &Query,
    q2: &Query,
    filter: Option<Query>,
  ) -> Result<()> {
    QueryUtils::check_from_query(q1);
    QueryUtils::check_from_query(q2);

    let q1 = if let Some(filter) = filter.clone() {
      let mut builder = Builder::new();
      builder.add(q1.clone(), Occur::Must)?;
      builder.add(filter, Occur::Filter)?;
      builder.build().into()
    } else {
      q1.clone()
    };

    let q2 = if let Some(filter) = filter {
      let mut builder = Builder::new();
      builder.add(q2.clone(), Occur::Must)?;
      builder.add(filter, Occur::Filter)?;
      builder.build().into()
    } else {
      q2.clone()
    };

    let meta = self.get_meta();
    let max_doc = meta.s1.get_index_reader().max_doc()? as usize;
    assert_eq!(max_doc, meta.s2.get_index_reader().max_doc()? as usize);
    for sort in [Sort::get_index_order()?, Sort::get_relevance()?] {
      let td1 = meta
        .s1
        .search_with_sort(q1.clone(), max_doc, sort.clone())?;
      let td2 = meta.s2.search_with_sort(q2.clone(), max_doc, sort)?;

      assert!(
        td1.total_hits().value() <= td2.total_hits().value(),
        "too many hits: {} > {}",
        td1.total_hits().value(),
        td2.total_hits().value()
      );

      let mut bitset = FixedBitSet::new(max_doc);
      for score_doc in td2.score_docs() {
        bitset.set(score_doc.doc() as usize);
      }

      for score_doc in td1.score_docs() {
        assert!(bitset.get(score_doc.doc() as usize)?);
      }
    }

    Ok(())
  }

  /// Assert that two queries return the same documents and with the same scores.
  fn assert_same_scores<R>(&self, random: &mut R, q1: &Query, q2: &Query) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.assert_same_set(random, q1, q2)?;
    self.assert_same_scores_with_filter(q1, q2, None)?;
    let num_filters = if is_night_mode() {
      at_least(random, 10)
    } else {
      at_least(random, 3)
    };
    for _ in 0..num_filters {
      let filter = random_filter(random)?;
      self.assert_same_scores_with_filter(q1, q2, Some(filter.clone()))?;
      self.assert_same_scores_with_filter(
        &filtered_query(q1, &filter)?,
        &filtered_query(q2, &filter)?,
        None,
      )?;
    }

    Ok(())
  }

  fn assert_same_scores_with_filter(
    &self,
    q1: &Query,
    q2: &Query,
    filter: Option<Query>,
  ) -> Result<()> {
    let q1 = if let Some(filter) = filter.clone() {
      let mut builder = Builder::new();
      builder.add(q1.clone(), Occur::Must)?;
      builder.add(filter, Occur::Filter)?;
      builder.build().into()
    } else {
      q1.clone()
    };

    let q2 = if let Some(filter) = filter {
      let mut builder = Builder::new();
      builder.add(q2.clone(), Occur::Must)?;
      builder.add(filter, Occur::Filter)?;
      builder.build().into()
    } else {
      q2.clone()
    };

    let meta = self.get_meta();
    let max_doc = meta.s1.get_index_reader().max_doc()? as usize;
    let td1 = meta.s1.search(q1.clone(), max_doc)?;
    let td2 = meta.s2.search(q2.clone(), max_doc)?;

    assert_eq!(td1.total_hits().value(), td2.total_hits().value());
    for i in 0..td1.score_docs().len() {
      assert_eq!(td1.score_docs()[i].doc(), td2.score_docs()[i].doc());
      assert!((td1.score_docs()[i].score() - td2.score_docs()[i].score()).abs() <= 10e-5);
    }
    Ok(())
  }
}
pub struct SearchEquivalenceTestBaseMeta {
  s1: DefaultIndexSearchCRShared,
  s2: DefaultIndexSearchCRShared,
}
impl SearchEquivalenceTestBaseMeta {
  pub fn new<R>(random: &mut R) -> Result<SearchEquivalenceTestBaseMeta>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;
    let stopword = random_char(random).to_string();
    let stopset = CharacterRunAutomaton::new(Automata::make_string(&stopword)?)?;
    let analyzer = MockAnalyzer::with_filter(random, WHITESPACE.clone(), false, stopset);
    let iw = RandomIndexWriter::with_analyzer(random, directory.clone(), analyzer);

    let id = StringField::from_string("id", "", Store::No)?;
    let field = TextField::from_string("field", "", Store::No)?;

    let num_docs = if is_night_mode() {
      at_least(random, 1000)
    } else {
      at_least(random, 100)
    };

    for i in 0..num_docs {
      let mut doc = Document::new();
      let mut id = id.clone();
      let mut field = field.clone();
      id.set_string_value(i.to_string())?;
      field.set_string_value(random_field_contents(random))?;
      doc.add(id);
      doc.add(field);
      iw.add_document(random, doc)?;
    }

    let num_deletes = num_docs / 20;
    for _ in 0..num_deletes {
      let to_delete = Term::from_text("id", random.random_range(0..num_docs).to_string());
      if random.random_bool(0.5) {
        iw.delete_documents_with_terms(random, vec![to_delete])?;
      } else {
        iw.delete_documents_with_terms(random, vec![to_delete])?;
        // TODO delete by query 未实现
        // iw.delete_documents(TermQuery::new(to_delete))?;
      }
    }

    let reader = Arc::new(iw.get_reader(random)?);
    let mut s1 = new_searcher_with_reader(reader.clone())?;
    s1.set_query_cache(None);
    let mut s2 = new_searcher_with_reader(reader)?;
    s2.set_query_cache(None);
    iw.close(random)?;

    Ok(SearchEquivalenceTestBaseMeta { s1, s2 })
  }
}
/// Populate a field with random contents. Terms should be single characters in lowercase (`a-z`).
/// Tokenization can be assumed to be on whitespace.
fn random_field_contents<R>(random: &mut R) -> String
where
  R: Rng + ?Sized,
{
  let mut sb = String::new();
  let num_terms = random.random_range(0..15);
  for _ in 0..num_terms {
    if !sb.is_empty() {
      sb.push(' ');
    }
    sb.push(random_char(random));
  }
  sb
}

/// Returns a random character (`a-z`).
fn random_char<R>(random: &mut R) -> char
where
  R: Rng + ?Sized,
{
  let mut c = char::from_u32(TestUtil::next_int(random, 'a' as i32, 'z' as i32) as u32).unwrap();
  if random.random_bool(0.5) {
    c = char::from_u32(TestUtil::next_int(random, 'a' as i32, c as i32) as u32).unwrap();
  }
  c
}

/// Returns a random filter over the document set.
fn random_filter<R>(random: &mut R) -> Result<Query>
where
  R: Rng + ?Sized,
{
  let query = if random.random_bool(0.5) {
    TermRangeQuery::new(
      "field",
      Some(BytesRef::from_string("a")),
      Some(BytesRef::from_string(&random_char(random).to_string())),
      true,
      true,
    )?
    .into_query()
  } else {
    PhraseQuery::from_bytes(
      100,
      "field",
      vec![
        BytesRef::from_string(&random_char(random).to_string()),
        BytesRef::from_string(&random_char(random).to_string()),
      ],
    )?
    .into()
  };
  Ok(query)
}
fn filtered_query(query: &Query, filter: &Query) -> Result<Query> {
  let mut builder = Builder::new();
  builder.add(query.clone(), Occur::Must)?;
  builder.add(filter.clone(), Occur::Filter)?;
  Ok(builder.build().into())
}
