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
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::collector::Collector;
use crate::core::search::doc_id_stream::DocIdStream;

use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::search::leaf_collector::LeafCollector;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::weight::Weight;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Display, Formatter};

/// Just counts the total number of hits. This is the collector behind [`IndexSearcher::count`](crate::core::search::index_searcher::IndexSearcher::count).
/// When the [`Weight`] implements [`Weight::count`], this collector will skip collecting segments.
pub struct TotalHitCountCollector {
  pub(crate) total_hit: i32,
}
impl Default for TotalHitCountCollector {
  fn default() -> Self {
    Self::new()
  }
}

impl TotalHitCountCollector {
  pub fn new() -> Self {
    Self { total_hit: 0 }
  }
  /// Returns how many hits matched the search.
  pub fn get_total_hits(&self) -> i32 {
    self.total_hit
  }
}
impl Collector for TotalHitCountCollector {
  type LeafCollector<'a, IRC>
    = TotalHitCountLeafCollector<'a>
  where
    Self: 'a,
    IRC: IndexReaderContext;

  fn get_leaf_collector<'a, W, IRC>(
    &'a mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    weight: Option<&W>,
  ) -> Result<Self::LeafCollector<'a, IRC>>
  where
    IRC: IndexReaderContext,
    W: Weight<IRC> + ?Sized,
  {
    let leaf_count = match weight {
      Some(w) => w.count(context)?,
      None => -1,
    };
    if leaf_count != -1 {
      self.total_hit += leaf_count;
      return Err(LuceneError::collection_terminated(""));
    }
    Ok(TotalHitCountLeafCollector::new(self))
  }

  fn score_mode(&self) -> ScoreMode {
    ScoreMode::CompleteNoScores
  }
}

pub struct TotalHitCountLeafCollector<'a> {
  collector: &'a mut TotalHitCountCollector,
}

impl<'a> TotalHitCountLeafCollector<'a> {
  fn new(collector: &'a mut TotalHitCountCollector) -> Self {
    Self { collector }
  }
}

impl Display for TotalHitCountLeafCollector<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", std::any::type_name::<Self>())
  }
}

impl<'a> LeafCollector for TotalHitCountLeafCollector<'a> {
  fn collect(&mut self, _doc: i32, _scorer: &mut dyn Scorable) -> Result<()> {
    self.collector.total_hit += 1;
    Ok(())
  }

  fn collect_stream(&mut self, stream: &mut dyn DocIdStream) -> Result<()> {
    self.collector.total_hit += stream.count()?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::string_field::StringField;

  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;

  use crate::core::search::match_all_docs_query::MatchAllDocsQuery;

  use crate::core::search::term_query::TermQuery;
  use crate::core::search::total_hit_count_collector_manager::TotalHitCountCollectorManager;

  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_searcher_with_reader, random,
  };

  #[allow(dead_code)] // for quick search
  struct TestTotalHitCountCollector;

  #[test]
  fn test_basics() -> Result<()> {
    let mut random = random();
    let index_store = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, index_store.clone());

    for i in 0..5 {
      let mut doc = Document::new();
      doc.add(StringField::from_string(
        "string",
        format!("a{}", i),
        Store::No,
      )?);
      doc.add(StringField::from_string(
        "string",
        format!("b{}", i),
        Store::No,
      )?);
      writer.add_document(doc)?;
    }

    let reader = writer.get_reader()?;
    writer.close()?;

    // TODO IMPORTANT 多线程未实现
    let searcher = new_searcher_with_reader(reader)?;
    let collector_manager = TotalHitCountCollectorManager::new(searcher.get_slices()?.as_slice());
    let mut total_hits =
      searcher.search_with_collector_manager(MatchAllDocsQuery::new(), &collector_manager)?;
    assert_eq!(5, total_hits);

    let mut builder = Builder::new();
    builder.add(
      TermQuery::new(Term::from_text("string", "a1")),
      Occur::Should,
    )?;
    builder.add(
      TermQuery::new(Term::from_text("string", "b3")),
      Occur::Should,
    )?;
    let query = builder.build();

    total_hits = searcher.search_with_collector_manager(query, &collector_manager)?;
    assert_eq!(2, total_hits);

    Ok(())
  }
}
