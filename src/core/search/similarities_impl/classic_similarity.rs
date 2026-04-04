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
use crate::core::search::collection_statistics::CollectionStatistics;
use crate::core::search::explanation::Explanation;
use crate::core::search::similarities_impl::tf_idf_similarity::{
  TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
};
use crate::core::search::term_statistics::TermStatistics;
/// Expert: Historical scoring implementation. You might want to consider using
/// [`BM25Similarity`](crate::core::search::similarities_impl::bm25_similarity::BM25Similarity) instead, which is generally considered superior to TF-IDF.
#[derive(Clone)]
pub struct ClassicSimilarity;
impl ClassicSimilarity {
  #[allow(clippy::new_ret_no_self)]
  pub fn new() -> TFIDFSimilarity {
    let v = TFIDFSubEnum::Classic(ClassicSimilarity);
    TFIDFSimilarity::new(v)
  }
  #[allow(clippy::new_ret_no_self)]
  pub fn with_discount_overlaps(discount_overlaps: bool) -> TFIDFSimilarity {
    let v = TFIDFSubEnum::Classic(ClassicSimilarity);
    TFIDFSimilarity::with_discount_overlaps(v, discount_overlaps)
  }
}
impl TFIDFSimilarityBase for ClassicSimilarity {
  fn tf(&self, freq: f32) -> f32 {
    freq.sqrt()
  }

  fn idf_explain(
    &self,
    collection_stats: &CollectionStatistics,
    term_stats: &TermStatistics,
  ) -> Explanation {
    idf_explain(self, collection_stats, term_stats)
  }

  fn idf(&self, doc_freq: i64, doc_count: i64) -> f32 {
    (((doc_count + 1) as f64 / (doc_freq + 1) as f64).ln() + 1.0) as f32
  }

  fn length_norm(&self, num_terms: i32) -> f32 {
    (1.0f64 / (num_terms as f64).sqrt()) as f32
  }
}
pub fn idf_explain<T>(
  s: &T,
  collection_stats: &CollectionStatistics,
  term_stats: &TermStatistics,
) -> Explanation
where
  T: TFIDFSimilarityBase,
{
  let df = term_stats.get_doc_freq();
  let doc_count = collection_stats.get_doc_count();
  let idf = s.idf(df, doc_count);

  Explanation::match_(
    idf,
    "idf, computed as log((docCount+1)/(docFreq+1)) + 1 from:".to_string(),
    vec![
      Explanation::match_(
        df,
        "docFreq, number of documents containing term".to_string(),
        vec![],
      ),
      Explanation::match_(
        doc_count,
        "docCount, total number of documents with field".to_string(),
        vec![],
      ),
    ],
  )
}
#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::string_field::StringField;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;

  use crate::core::index::field_invert_state::FieldInvertState;
  use crate::core::index::index_options::IndexOptions::DocsAndFreqs;
  use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
  use crate::core::search::similarities_impl::bm25_similarity::BM25Similarity;
  use crate::core::search::similarities_impl::classic_similarity::ClassicSimilarity;
  use crate::core::search::similarities_impl::similarities::Similarity;
  use crate::core::search::similarities_impl::tf_idf_similarity::TFIDFSimilarity;
  use crate::core::search::term_query::TermQuery;
  use crate::core::util::LATEST;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::similarities::base_similarity_test_case::BaseSimilarityTestCase;
  use crate::test::core::util::DefaultIndexSearchCR;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, new_searcher_with_reader, random,
  };
  use crate::test::core::util::test_util::TestUtil;
  use rand::Rng;

  #[allow(dead_code)] // for quick search
  struct TestClassicSimilarity;

  fn test_set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;

    let index_writer = IndexWriter::new(directory.clone(), new_index_writer_config(random))?;

    let mut document = Document::new();
    document.add(StringField::from_string("test", "hit", Store::No)?);
    index_writer.add_document(document)?;
    index_writer.commit()?;

    let index_reader = directory_reader_util::open(directory)?;
    let mut index_searcher = new_searcher_with_reader(index_reader)?;
    index_searcher.set_similarity(ClassicSimilarity::new());
    Ok(index_searcher)
  }
  #[test]
  fn test_hit() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = TermQuery::new(Term::from_text("test", "hit"));
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);
    Ok(())
  }
  #[test]
  fn test_miss() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = TermQuery::new(Term::from_text("test", "miss"));
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(0, top_docs.total_hits.value());
    Ok(())
  }

  #[test]
  fn test_empty() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = TermQuery::new(Term::from_text("empty", "miss"));
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(0, top_docs.total_hits.value());
    Ok(())
  }

  #[test]
  fn test_bq_hit() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;

    let mut builder = Builder::new();
    builder.add(
      TermQuery::new(Term::from_text("test", "hit")),
      Occur::Should,
    )?;
    let query = builder.build();

    let top_docs = index_searcher.search(query, 1)?;

    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);

    Ok(())
  }
  #[test]
  fn test_bq_hit_or_miss() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;

    let mut builder = Builder::new();
    builder.add(
      TermQuery::new(Term::from_text("test", "hit")),
      Occur::Should,
    )?;
    builder.add(
      TermQuery::new(Term::from_text("test", "miss")),
      Occur::Should,
    )?;
    let query = builder.build();

    let top_docs = index_searcher.search(query, 1)?;

    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);

    Ok(())
  }

  #[test]
  fn test_bq_hit_or_empty() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;

    let mut builder = Builder::new();
    builder.add(
      TermQuery::new(Term::from_text("test", "hit")),
      Occur::Should,
    )?;
    builder.add(
      TermQuery::new(Term::from_text("empty", "miss")),
      Occur::Should,
    )?;
    let query = builder.build();

    let top_docs = index_searcher.search(query, 1)?;

    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);

    Ok(())
  }

  #[test]
  fn test_dmq_hit() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = DisjunctionMaxQuery::new(
      vec![TermQuery::new(Term::from_text("test", "hit")).into()],
      0.0,
    )?;
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);
    Ok(())
  }
  #[test]
  fn test_dmq_hit_or_miss() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text("test", "hit")).into(),
        TermQuery::new(Term::from_text("test", "miss")).into(),
      ],
      0.0,
    )?;
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);
    Ok(())
  }

  #[test]
  fn test_dmq_hit_or_empty() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;
    let query = DisjunctionMaxQuery::new(
      vec![
        TermQuery::new(Term::from_text("test", "hit")).into(),
        TermQuery::new(Term::from_text("empty", "miss")).into(),
      ],
      0.0,
    )?;
    let top_docs = index_searcher.search(query, 1)?;
    assert_eq!(1, top_docs.total_hits.value());
    assert_eq!(1, top_docs.score_docs.len());
    assert_ne!(top_docs.score_docs[0].score, 0.0);
    Ok(())
  }
  #[test]
  fn test_sane_norm_values() -> Result<()> {
    let mut random = random();
    let index_searcher = test_set_up(&mut random)?;

    let sim = ClassicSimilarity::new();
    let collection_stats = index_searcher.collection_statistics("test")?;
    let stats = sim.scorer(1.0, collection_stats.as_ref().unwrap(), &[])?;

    for i in 0..256 {
      let boost = stats.norm_table[i];

      assert!(boost >= 0.0, "negative boost: {}, byte={}", boost, i);
      assert!(!boost.is_infinite(), "inf boost: {}, byte={}", boost, i);
      assert!(!boost.is_nan(), "nan boost for byte={}", i);

      if i > 0 {
        assert!(
          boost < stats.norm_table[i - 1],
          "boost is not decreasing: {}, byte={}",
          boost,
          i
        );
      }
    }

    Ok(())
  }
  #[test]
  fn test_same_norms_as_bm25() -> Result<()> {
    let mut random = random();

    let sim1 = ClassicSimilarity::new();
    let sim2 = BM25Similarity::new()?;

    for _ in 0..100 {
      let length = TestUtil::next_int(&mut random, 1, 1000);
      let position = TestUtil::next_int(&mut random, 0, length - 1);
      let num_overlaps = TestUtil::next_int(&mut random, 0, length - 1);
      let max_term_frequency = 1;
      let unique_term_count = 1;

      let state = FieldInvertState::with_states(
        LATEST.major,
        "foo".to_string(),
        DocsAndFreqs,
        position,
        length,
        num_overlaps,
        100,
        max_term_frequency,
        unique_term_count,
      );

      assert_eq!(sim2.compute_norm(&state)?, sim1.compute_norm(&state)?);
    }

    Ok(())
  }

  impl BaseSimilarityTestCase for TestClassicSimilarity {
    type Similarity = TFIDFSimilarity;

    fn get_similarity<R>(&self, _random: &mut R) -> Result<Self::Similarity>
    where
      R: Rng + ?Sized,
    {
      Ok(ClassicSimilarity::new())
    }
  }
  #[test]
  fn test_random_scoring() -> Result<()> {
    let mut random = random();
    let case = TestClassicSimilarity;
    case.test_random_scoring(&mut random)
  }
}
