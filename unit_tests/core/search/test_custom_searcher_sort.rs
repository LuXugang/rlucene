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
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::string_field::StringField;
use crate::core::document::text_field::TextField;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::composite_reader_context::CompositeReaderContext;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::query::Query;
use crate::core::search::score_doc::{ScoreDoc, ScoreDocLike};
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::{SortField, SortFieldType};
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocs;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::search::top_field_docs::TopFieldDocs;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::DefaultCRReader;
use crate::test::support::core::util::lucene_test_case::{
  at_least_usize, new_directory_shared, random,
};
use rand::{Rng, RngExt};
use std::collections::BTreeMap;

#[allow(dead_code)] // for quick search
pub struct TestCustomSearcherSort;

fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<(DefaultCRReader, Query, usize)> {
  let index_size = at_least_usize(random, 2000);
  let index = new_directory_shared(random)?;

  let writer = RandomIndexWriter::new(random, index)?;
  let random_gen = RandomGen;

  for i in 0..index_size {
    let mut doc = Document::new();

    if i % 5 != 0 {
      doc.add(SortedDocValuesField::new(
        "publicationDate_",
        BytesRef::from_string(&random_gen.get_lucene_date(random)),
      ));
    }

    if i % 7 == 0 {
      doc.add(TextField::from_string("content", "test", Store::Yes)?);
    }

    doc.add(StringField::from_string(
      "mandant",
      (i % 3).to_string(),
      Store::Yes,
    )?);

    writer.add_document(random, doc)?;
  }
  let reader = writer.get_reader(random)?;
  writer.close(random)?;
  let query = TermQuery::new(Term::from_text("content", "test")).into();
  Ok((reader, query, index_size))
}

#[test]
fn test_field_sort_custom_searcher() -> Result<()> {
  let mut random = random();
  let (reader, query, _) = set_up(&mut random)?;
  let cust_sort = Sort::with_fields(vec![
    SortField::new(Some("publicationDate_"), SortFieldType::String)?,
    SortField::get_field_score()?,
  ])?;
  let searcher = CustomSearcher::new(reader, 2);
  match_hits(&searcher, &query, cust_sort)
}

#[test]
fn test_field_sort_single_searcher() -> Result<()> {
  let mut random = random();
  let (reader, query, _) = set_up(&mut random)?;
  let cust_sort = Sort::with_fields(vec![
    SortField::new(Some("publicationDate_"), SortFieldType::String)?,
    SortField::get_field_score()?,
  ])?;
  let searcher = CustomSearcher::new(reader, 2);
  match_hits(&searcher, &query, cust_sort)
}
fn match_hits(searcher: &CustomSearcher<DefaultCRReader>, query: &Query, sort: Sort) -> Result<()> {
  let hits_by_rank = searcher.search(query.clone(), usize::MAX)?.score_docs;
  check_hits(hits_by_rank.as_slice(), "Sort by rank: ");

  let mut result_map = BTreeMap::new();
  for (hit_id, hit) in hits_by_rank.iter().enumerate() {
    result_map.insert(hit.doc, hit_id);
  }

  let result_sort = searcher.search_with_sort(query.clone(), usize::MAX, sort)?;
  let v = result_sort.base.score_docs();
  check_hits(v, "Sort by custom criteria: ");

  for hit in result_sort.score_docs() {
    assert!(
      result_map.remove(&hit.doc()).is_some(),
      "sorted hit doc {} was not present in rank-sorted hits",
      hit.doc()
    );
  }

  assert_eq!(0, result_map.len());
  Ok(())
}

fn check_hits<T>(hits: &[T], prefix: &str)
where
  T: ScoreDocLike,
{
  let mut id_map = BTreeMap::new();
  for (doc_num, sd) in hits.iter().enumerate() {
    if let Some(previous) = id_map.insert(sd.doc(), doc_num) {
      panic!(
        "{prefix}Duplicate key for hit index = {doc_num}, previous index = {previous}, Lucene ID = {sd}"
      );
    }
  }
}

pub struct CustomSearcher<CR>
where
  CR: CompositeReader + Sync + 'static,
  <CR as CompositeReader>::LeafReader: Sync,
{
  searcher: IndexSearcher<CompositeReaderContext<CR>>,
  switcher: i32,
}

impl<CR> CustomSearcher<CR>
where
  CR: CompositeReader + Sync + 'static,
  <CR as CompositeReader>::LeafReader: Sync,
{
  pub fn new(cr: CR, switcher: i32) -> Self {
    let s = IndexSearcher::from_cr(cr).unwrap();
    Self {
      searcher: s,
      switcher,
    }
  }
  pub fn search_with_sort<Q>(&self, query: Q, n_docs: usize, sort: Sort) -> Result<TopFieldDocs>
  where
    Q: Into<Query>,
  {
    let mut bq = Builder::new();
    bq.add(query, Occur::Must)?;
    bq.add(
      TermQuery::new(Term::from_text("mandant", self.switcher.to_string())),
      Occur::Must,
    )?;
    let q = bq.build();
    self.searcher.search_with_sort(q, n_docs, sort)
  }
  pub fn search<Q>(&self, query: Q, n_docs: usize) -> Result<TopDocs<ScoreDoc>>
  where
    Q: Into<Query>,
  {
    let mut bq = Builder::new();
    bq.add(query, Occur::Must)?;
    bq.add(
      TermQuery::new(Term::from_text("mandant", self.switcher.to_string())),
      Occur::Must,
    )?;
    let q = bq.build();
    self.searcher.search(q, n_docs)
  }
}
#[derive(Default)]
struct RandomGen;

impl RandomGen {
  fn new() -> Self {
    Self
  }
  fn get_lucene_date<R: Rng + ?Sized>(&self, random: &mut R) -> String {
    //  TODO IMPORTANT DateTools未实现
    let day_offset = random.random_range(0..50);

    let (month, day) = if day_offset < 29 {
      (2, day_offset + 1)
    } else {
      (3, day_offset - 29 + 1)
    };

    format!("1980{month:02}{day:02}")
  }
}
