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
use crate::core::document::fields::Fields;
use crate::core::document::knn_float_vector_field::KnnFloatVectorField;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::search::abstract_knn_vector_query::{DocAndScoreQuery, find_segment_starts};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::search::knn_byte_vector_query::KnnByteVectorQuery;
use crate::core::search::knn_float_vector_query::KnnFloatVectorQuery;
use crate::core::search::match_all_docs_query::MatchAllDocsQuery;
use crate::core::search::query::Query;
use crate::core::search::query::QueryBase;
use crate::core::search::scorable::Scorable;
use crate::core::search::score_doc::ScoreDoc;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer::Scorer;
use crate::core::search::term_query::TermQuery;
use crate::core::search::total_hits::Relation::EqualTo;
use crate::core::search::weight::Weight;
use crate::core::store::directory::DirEnum;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::vector_util::VectorUtil;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;
use crate::test_framework::core::util::lucene_test_case::{new_searcher_with_reader, random};
use crate::test_framework::core::util::test_vector_util::random_vector_dim;
use rand::rngs::StdRng;
use rand::{Rng, RngExt};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub(crate) struct TestKnnFloatVectorQuery;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestKnnFloatVectorQuery, &mut StdRng) -> Result<()>,
{
  let case = TestKnnFloatVectorQuery;
  let mut random = random();
  f(&case, &mut random)
}

mod base_knn_vector_query_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::search::test_knn_float_vector_query::run_case;
  use crate::test_framework::core::search::base_knn_vector_query_test_case::BaseKnnVectorQueryTestCase;

  #[test]
  fn test_equals() -> Result<()> {
    run_case(|case, _random| case.test_equals())
  }

  #[test]
  fn test_get_field() -> Result<()> {
    run_case(|case, _random| case.test_get_field())
  }

  #[test]
  fn test_get_k() -> Result<()> {
    run_case(|case, _random| case.test_get_k())
  }

  #[test]
  fn test_get_filter() -> Result<()> {
    run_case(|case, _random| case.test_get_filter())
  }

  #[test]
  fn test_empty_index() -> Result<()> {
    run_case(|case, _random| case.test_empty_index())
  }

  #[test]
  fn test_find_all() -> Result<()> {
    run_case(|case, random| case.test_find_all(random))
  }

  #[test]
  fn test_find_fewer() -> Result<()> {
    run_case(|case, random| case.test_find_fewer(random))
  }

  #[test]
  fn test_search_boost() -> Result<()> {
    run_case(|case, random| case.test_search_boost(random))
  }

  #[test]
  fn test_simple_filter() -> Result<()> {
    run_case(|case, random| case.test_simple_filter(random))
  }

  #[test]
  fn test_filter_with_no_vector_matches() -> Result<()> {
    run_case(|case, random| case.test_filter_with_no_vector_matches(random))
  }

  #[test]
  fn test_dimension_mismatch() -> Result<()> {
    run_case(|case, random| case.test_dimension_mismatch(random))
  }

  #[test]
  fn test_non_vector_field() -> Result<()> {
    run_case(|case, random| case.test_non_vector_field(random))
  }

  #[test]
  fn test_illegal_arguments() -> Result<()> {
    run_case(|case, _random| case.test_illegal_arguments())
  }

  #[test]
  fn test_different_reader() -> Result<()> {
    run_case(|case, random| case.test_different_reader(random))
  }

  #[test]
  fn test_score_euclidean() -> Result<()> {
    run_case(|case, random| case.test_score_euclidean(random))
  }

  #[test]

  fn test_score_cosine() -> Result<()> {
    run_case(|case, random| case.test_score_cosine(random))
  }

  #[test]
  fn test_score_mip() -> Result<()> {
    run_case(|case, random| case.test_score_mip(random))
  }

  #[test]
  fn test_explain() -> Result<()> {
    run_case(|case, random| case.test_explain(random))
  }

  #[test]
  fn test_explain_multiple_segments() -> Result<()> {
    run_case(|case, random| case.test_explain_multiple_segments(random))
  }

  #[test]
  fn test_skewed_index() -> Result<()> {
    run_case(|case, random| case.test_skewed_index(random))
  }

  #[test]
  fn test_random() -> Result<()> {
    run_case(|case, random| case.test_random(random))
  }
  #[test]
  fn test_random_with_filter() -> Result<()> {
    run_case(|case, random| case.test_random_with_filter(random))
  }
  #[test]
  fn test_filter_with_same_score() -> Result<()> {
    run_case(|case, random| case.test_filter_with_same_score(random))
  }

  #[test]
  fn test_deletes() -> Result<()> {
    run_case(|case, random| case.test_deletes(random))
  }

  #[test]
  fn test_all_deletes() -> Result<()> {
    run_case(|case, random| case.test_all_deletes(random))
  }

  #[test]
  fn test_merge_away_all_values() -> Result<()> {
    run_case(|case, random| case.test_merge_away_all_values(random))
  }
  #[test]
  fn test_no_live_docs_reader() -> Result<()> {
    run_case(|case, random| case.test_no_live_docs_reader(random))
  }
  #[test]
  #[ignore = "BitSet filter reuse is not implemented"]
  fn test_bit_set_query() -> Result<()> {
    run_case(|case, random| case.test_bit_set_query(random))
  }
  #[test]
  fn test_time_limiting_knn_collector_manager() -> Result<()> {
    run_case(|case, random| case.test_time_limiting_knn_collector_manager(random))
  }

  #[test]
  fn test_timeout() -> Result<()> {
    run_case(|case, random| case.test_timeout(random))
  }
  #[test]
  fn test_same_field_different_formats() -> Result<()> {
    run_case(|case, random| case.test_same_field_different_formats(random))
  }
}

impl BaseKnnVectorQueryTestCase for TestKnnFloatVectorQuery {
  type KnnVectorQuery = KnnFloatVectorQuery;

  fn get_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery> {
    KnnFloatVectorQuery::with_filter(field, query, k, query_filter)
  }

  fn get_throwing_knn_vector_query(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
    query_filter: Option<Query>,
  ) -> Result<Self::KnnVectorQuery> {
    KnnFloatVectorQuery::throwing_with_filter(field, query, k, query_filter)
  }

  fn get_knn_vector_query_no_filter(
    &self,
    field: &str,
    query: Vec<f32>,
    k: usize,
  ) -> Result<Self::KnnVectorQuery> {
    self.get_knn_vector_query(field, query, k, None)
  }

  fn random_vector<R>(&self, random: &mut R, dim: usize) -> Vec<f32>
  where
    R: Rng + ?Sized,
  {
    random_vector_dim(random, dim)
  }

  fn get_knn_vector_field_with_similarity(
    &self,
    name: &str,
    vector: Vec<f32>,
    similarity_function: VectorSimilarityFunction,
  ) -> Result<Fields> {
    Ok(KnnFloatVectorField::with_similarity_function(name, vector, similarity_function)?.into())
  }

  fn get_knn_vector_field(&self, name: &str, vector: Vec<f32>) -> Result<Fields> {
    Ok(KnnFloatVectorField::new(name, vector)?.into())
  }

  type Directory = Arc<DirEnum>;

  fn new_directory_for_test<R>(&self, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    self.default_new_directory_for_test(random)
  }
}
#[test]
fn test_to_string() -> Result<()> {
  run_case(|case, random| {
    let index_store = case.get_index_store(
      random,
      "field",
      &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]],
    )?;
    let reader = directory_reader::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;

    let query = case.get_knn_vector_query_no_filter("field", vec![0.0, 1.0], 10)?;
    assert_eq!(
      "KnnFloatVectorQuery:field[0,...][10]",
      query.to_string("ignored")?
    );

    let rewritten = searcher.rewrite(query.clone())?;
    case.assert_doc_score_query_to_string(&rewritten)?;

    let filter: Query = TermQuery::new(Term::from_text("id", "text")).into();
    let query = case.get_knn_vector_query("field", vec![0.0, 1.0], 10, Some(filter))?;
    assert_eq!(
      "KnnFloatVectorQuery:field[0,...][10][id:text]",
      query.to_string("ignored")?
    );
    Ok(())
  })
}

#[test]
fn test_vector_encoding_mismatch() -> Result<()> {
  run_case(|case, random| {
    let index_store = case.get_index_store(
      random,
      "field",
      &[vec![0.0, 1.0], vec![1.0, 2.0], vec![0.0, 0.0]],
    )?;
    let reader = directory_reader::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;
    let filter = if random.random_bool(0.5) {
      Some(MatchAllDocsQuery::new().into())
    } else {
      None
    };
    let query = KnnByteVectorQuery::with_filter("field", vec![0, 1], 10, filter)?;
    match searcher.search(query, 10) {
      Err(error) if error.is_illegal_state_error() => Ok(()),
      _ => unreachable!(""),
    }
  })
}

#[test]
fn test_get_target() -> Result<()> {
  let query_vector = vec![0.0, 1.0];
  let query = KnnFloatVectorQuery::new("f1", query_vector.clone(), 10)?;
  let copy = query.get_target_copy();
  assert_eq!(query_vector, copy);
  assert_ne!(query_vector.as_ptr(), copy.as_ptr());
  Ok(())
}

#[test]
fn test_score_negative_dot_product() -> Result<()> {
  run_case(|case, random| {
    let directory = case.new_directory_for_test(random)?;
    let writer = IndexWriter::new(directory.clone(), IndexWriterConfig::new()?)?;

    let mut doc = Document::new();
    doc.add(case.get_knn_vector_field_with_similarity(
      "field",
      vec![-1.0, 0.0],
      VectorSimilarityFunction::DotProduct,
    )?);
    writer.add_document(doc)?;

    let mut doc = Document::new();
    doc.add(case.get_knn_vector_field_with_similarity(
      "field",
      vec![1.0, 0.0],
      VectorSimilarityFunction::DotProduct,
    )?);
    writer.add_document(doc)?;
    writer.close()?;

    let reader = directory_reader::open(directory)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(1, searcher.get_leaf_contexts()?.len());
    let query = case.get_knn_vector_query_no_filter("field", vec![1.0, 0.0], 2)?;
    let rewritten = searcher.rewrite(query)?;
    let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
    let leaf = &searcher.get_leaf_contexts()?[0];
    let mut scorer = weight.scorer(leaf, &searcher)?.unwrap();

    assert_eq!(2, scorer.iterator().cost()?);
    assert_eq!(0, scorer.iterator_mut().next_doc()?);
    assert!(scorer.score()? >= 0.0);
    assert_eq!(1, scorer.iterator_mut().advance(1)?);
    assert_eq!(1.0, scorer.score()?);
    Ok(())
  })
}

#[test]
fn test_score_dot_product() -> Result<()> {
  run_case(|case, random| {
    let directory = case.new_directory_for_test(random)?;
    let writer = IndexWriter::new(directory.clone(), IndexWriterConfig::new()?)?;
    for j in 1..=5 {
      let mut vector = vec![j as f32, (j * j) as f32];
      VectorUtil::l2normalize(&mut vector)?;
      let mut doc = Document::new();
      doc.add(case.get_knn_vector_field_with_similarity(
        "field",
        vector,
        VectorSimilarityFunction::DotProduct,
      )?);
      writer.add_document(doc)?;
    }
    writer.close()?;

    let reader = directory_reader::open(directory)?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(1, searcher.get_leaf_contexts()?.len());

    let mut query_vector = vec![2.0, 3.0];
    VectorUtil::l2normalize(&mut query_vector)?;
    let query = case.get_knn_vector_query_no_filter("field", query_vector, 3)?;
    let rewritten = searcher.rewrite(query)?;
    let weight = searcher.create_weight(rewritten, ScoreMode::Complete, 1.0)?;
    let leaf = &searcher.get_leaf_contexts()?[0];
    let mut scorer = weight.scorer(leaf, &searcher)?.unwrap();

    assert_eq!(-1, scorer.doc_id()?);
    assert!(matches!(
      scorer.score(),
      Err(LuceneError::ArrayIndexOutOfBounds(_))
    ));

    let score0 = (1.0 + (2.0 * 1.0 + 3.0 * 1.0) / ((13.0_f32 * 2.0_f32).sqrt())) / 2.0;
    let score1 = (1.0 + (2.0 * 2.0 + 3.0 * 4.0) / ((13.0_f32 * 20.0_f32).sqrt())) / 2.0;

    assert!((score1 - scorer.get_max_score(2)?).abs() <= 0.0001);
    assert!((score1 - scorer.get_max_score(i32::MAX)?).abs() <= 0.0001);

    {
      let mut it = scorer.iterator_mut();
      assert_eq!(3, it.cost()?);
      assert_eq!(0, it.next_doc()?);
    }
    assert!((score0 - scorer.score()?).abs() <= 0.0001);
    assert_eq!(1, scorer.iterator_mut().advance(1)?);
    assert!((score1 - scorer.score()?).abs() <= 0.0001);
    assert_eq!(NO_MORE_DOCS, scorer.iterator_mut().advance(4)?);
    assert!(matches!(
      scorer.score(),
      Err(LuceneError::ArrayIndexOutOfBounds(_))
    ));
    Ok(())
  })
}

#[test]
fn test_doc_and_score_query_basics() -> Result<()> {
  run_case(|case, random| {
    let directory = case.new_directory_for_test(random)?;
    let reader = {
      let writer = RandomIndexWriter::new(random, directory.clone())?;
      for i in 0..50 {
        let mut doc = Document::new();
        doc.add(StringField::from_string(
          "field",
          format!("value{i}"),
          Store::No,
        )?);
        writer.add_document(random, doc)?;
        if i % 10 == 0 {
          writer.flush()?;
        }
      }
      let reader = writer.get_reader(random)?;
      writer.close(random)?;
      reader
    };

    let searcher = new_searcher_with_reader(reader)?;
    let mut score_docs = Vec::new();
    let mut doc = 0i32;
    while doc < 30 {
      score_docs.push(ScoreDoc::new(doc, random.random::<f32>()));
      doc += 1 + random.random_range(0..5);
    }

    let docs = score_docs.iter().map(|sd| sd.doc).collect::<Vec<_>>();
    let scores = score_docs.iter().map(|sd| sd.score).collect::<Vec<_>>();
    let max_score = scores.iter().copied().fold(f32::MIN, f32::max);
    let leaves = searcher.get_leaf_contexts()?;
    let segments = find_segment_starts(leaves, &docs)?;
    let _index_reader = searcher.get_index_reader();

    let query = DocAndScoreQuery::new(
      docs,
      scores,
      max_score,
      segments,
      searcher.get_top_reader_context().base().id().clone(),
    );
    let weight = searcher.create_weight(query.clone(), ScoreMode::TopScores, 1.0)?;
    let mut top_docs = searcher.search(query.clone(), 100)?;
    assert_eq!(score_docs.len(), top_docs.total_hits.value());
    assert_eq!(EqualTo, top_docs.total_hits.relation());
    top_docs.score_docs.sort_by_key(|a| a.doc);

    assert_eq!(score_docs.len(), top_docs.score_docs.len());
    for (expected, actual) in score_docs.iter().zip(top_docs.score_docs.iter()) {
      assert_eq!(expected.doc, actual.doc);
      assert!((expected.score - actual.score).abs() <= 0.0001);
      assert!(searcher.explain(query.clone(), expected.doc)?.is_match());
    }

    for leaf in leaves {
      let scorer = weight.scorer(leaf, &searcher)?;
      let count = weight.count(leaf, &searcher)?;
      match scorer {
        None => {
          assert_eq!(0, count);
        },
        Some(mut scorer) => {
          assert!(scorer.get_max_score(NO_MORE_DOCS)? > 0.0);
          assert!(count > 0);
          let mut iterator_count = 0;
          while scorer.iterator_mut().next_doc()? != NO_MORE_DOCS {
            iterator_count += 1;
          }
          assert_eq!(iterator_count, count);
        },
      }
    }
    Ok(())
  })
}
