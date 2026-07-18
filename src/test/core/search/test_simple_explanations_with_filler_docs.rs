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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::document::document::Document;
use crate::core::document::field::Store;
use crate::core::document::string_field::StringField;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder as BooleanQueryBuilder;
use crate::core::search::boolean_scorer::SIZE;
use crate::core::search::query::IntoQuery;
use crate::core::search::term_query::TermQuery;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::test_simple_explanations::SimpleExplanations;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::search::base_explanation_test_case::{
  BaseExplanationTestCase, BaseExplanationTestContext, DOC_FIELDS, before_class_test_explanations,
  create_doc,
};
use crate::test_framework::core::util::DefaultIndexSearchCRShared;
use crate::test_framework::core::util::lucene_test_case::{
  is_night_mode, new_directory_shared, new_index_writer_config_with_analyzer, new_log_merge_policy,
  new_searcher_with_reader, random,
};
use rand::prelude::StdRng;
use rand::{Rng, RngExt};
use std::sync::Arc;

const NUM_FILLER_DOCS_DEFAULT: usize = 4;
const EXTRA: &str = "extra";

/// TestSimpleExplanations implementation that adds a lot of filler docs which will be ignored at
/// query time. These filler docs will either all be empty in which case the queries will be
/// unmodified, or they will all use terms from same set of source data as our regular docs (to
/// emphasize the DocFreq factor in scoring), in which case the queries will be wrapped so they can
/// be excluded.
#[allow(dead_code)] // for quick search
struct TestSimpleExplanationsWithFillerDocs {
  context: BaseExplanationTestContext,
  num_filler_docs: usize,
  pre_filler_docs: usize,
  extra: Option<&'static str>,
}

impl TestSimpleExplanationsWithFillerDocs {
  fn new<R>(random: &mut R) -> Result<Self>
  where
    R: Rng + ?Sized,
  {
    let context = before_class_test_explanations(random)?;
    let mut test = Self {
      context,
      num_filler_docs: if is_night_mode() {
        SIZE
      } else {
        NUM_FILLER_DOCS_DEFAULT
      },
      pre_filler_docs: 0,
      extra: None,
    };
    test.replace_index(random)?;
    Ok(test)
  }

  /// Replaces the index created by the base context with a new one that includes a lot of filler
  /// docs. [`BaseExplanationTestCase::q_test`] will account for these extra filler docs.
  fn replace_index<R>(&mut self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    self.extra = if random.random_bool(0.5) {
      None
    } else {
      Some(EXTRA)
    };
    self.pre_filler_docs = random.random_range(0..=(self.num_filler_docs / 2));

    self.context.reader.close()?;
    self.context.directory.close()?;

    let directory = new_directory_shared(random)?;
    let shared_analyzer: Box<dyn Analyzer> = Box::new(self.context.analyzer.clone());
    let mut config = new_index_writer_config_with_analyzer(random, shared_analyzer)?;
    config.set_merge_policy(new_log_merge_policy(random)?);
    let writer = RandomIndexWriter::with_config(random, directory.clone(), config);

    for _ in 0..self.pre_filler_docs {
      let doc = self.make_filler_doc(random)?;
      writer.add_document(random, doc)?;
    }
    for i in 0..DOC_FIELDS.len() {
      writer.add_document(random, create_doc(i)?)?;

      for _ in 0..self.num_filler_docs {
        let doc = self.make_filler_doc(random)?;
        writer.add_document(random, doc)?;
      }
    }
    let reader = Arc::new(writer.get_reader(random)?);
    writer.close(random)?;
    let searcher = new_searcher_with_reader(reader.clone())?;

    self.context.directory = directory;
    self.context.reader = reader;
    self.context.searcher = searcher;
    Ok(())
  }

  fn make_filler_doc<R>(&self, random: &mut R) -> Result<Document>
  where
    R: Rng + ?Sized,
  {
    if let Some(extra) = self.extra {
      let mut doc = create_doc(random.random_range(0..DOC_FIELDS.len()))?;
      doc.add(StringField::from_string(extra, extra, Store::No)?);
      Ok(doc)
    } else {
      Ok(Document::new())
    }
  }
}

impl BaseExplanationTestCase for TestSimpleExplanationsWithFillerDocs {
  /// Adjusts `exp_doc_nrs` based on the filler docs injected in the index, and if necessary wraps
  /// `q` in a BooleanQuery that will filter out all filler docs using the `EXTRA` field.
  ///
  /// See [`Self::replace_index`].
  fn q_test<R, Q>(
    &self,
    random: &mut R,
    searcher: &DefaultIndexSearchCRShared,
    q: Q,
    exp_doc_nrs: &[i32],
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    Q: IntoQuery,
  {
    let exp_doc_nrs: Vec<i32> = exp_doc_nrs
      .iter()
      .map(|doc| self.pre_filler_docs as i32 + ((self.num_filler_docs + 1) as i32 * *doc))
      .collect();

    let mut q = q.into_query();
    if let Some(extra) = self.extra {
      let mut builder = BooleanQueryBuilder::new();
      builder.add(q, Occur::Must)?;
      builder.add(
        TermQuery::new(Term::from_text(extra, extra)),
        Occur::MustNot,
      )?;
      q = builder.build().into();
    }
    self.default_q_test(random, searcher, q, exp_doc_nrs.as_ref())
  }
}

impl SimpleExplanations for TestSimpleExplanationsWithFillerDocs {
  fn context(&self) -> &BaseExplanationTestContext {
    &self.context
  }

  fn test_ma1<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    if self.extra.is_none() {
      return Ok(());
    }
    self.default_test_ma1(random)
  }

  fn test_ma2<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    if self.extra.is_none() {
      return Ok(());
    }
    self.default_test_ma2(random)
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestSimpleExplanationsWithFillerDocs, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestSimpleExplanationsWithFillerDocs::new(&mut random)?;
  f(&case, &mut random)
}

mod simple_explanations_tests {
  use super::{SimpleExplanations, run_case};
  use crate::core::util::error::lucene_error::Result;

  #[test]
  fn test_t1() -> Result<()> {
    run_case(|case, random| case.test_t1(random))
  }

  #[test]
  fn test_t2() -> Result<()> {
    run_case(|case, random| case.test_t2(random))
  }

  #[test]
  fn test_ma1() -> Result<()> {
    run_case(|case, random| case.test_ma1(random))
  }

  #[test]
  fn test_ma2() -> Result<()> {
    run_case(|case, random| case.test_ma2(random))
  }

  #[test]
  fn test_p1() -> Result<()> {
    run_case(|case, random| case.test_p1(random))
  }

  #[test]
  fn test_p2() -> Result<()> {
    run_case(|case, random| case.test_p2(random))
  }

  #[test]
  fn test_p3() -> Result<()> {
    run_case(|case, random| case.test_p3(random))
  }

  #[test]
  fn test_p4() -> Result<()> {
    run_case(|case, random| case.test_p4(random))
  }

  #[test]
  fn test_p5() -> Result<()> {
    run_case(|case, random| case.test_p5(random))
  }

  #[test]
  fn test_p6() -> Result<()> {
    run_case(|case, random| case.test_p6(random))
  }

  #[test]
  fn test_p7() -> Result<()> {
    run_case(|case, random| case.test_p7(random))
  }

  #[test]
  fn test_csq1() -> Result<()> {
    run_case(|case, random| case.test_csq1(random))
  }

  #[test]
  fn test_csq2() -> Result<()> {
    run_case(|case, random| case.test_csq2(random))
  }

  #[test]
  fn test_csq3() -> Result<()> {
    run_case(|case, random| case.test_csq3(random))
  }

  #[test]
  fn test_dmq1() -> Result<()> {
    run_case(|case, random| case.test_dmq1(random))
  }

  #[test]
  fn test_dmq2() -> Result<()> {
    run_case(|case, random| case.test_dmq2(random))
  }

  #[test]
  fn test_dmq3() -> Result<()> {
    run_case(|case, random| case.test_dmq3(random))
  }

  #[test]
  fn test_dmq4() -> Result<()> {
    run_case(|case, random| case.test_dmq4(random))
  }

  #[test]
  fn test_dmq5() -> Result<()> {
    run_case(|case, random| case.test_dmq5(random))
  }

  #[test]
  fn test_dmq6() -> Result<()> {
    run_case(|case, random| case.test_dmq6(random))
  }

  #[test]
  fn test_dmq7() -> Result<()> {
    run_case(|case, random| case.test_dmq7(random))
  }

  #[test]
  fn test_dmq8() -> Result<()> {
    run_case(|case, random| case.test_dmq8(random))
  }

  #[test]
  fn test_dmq9() -> Result<()> {
    run_case(|case, random| case.test_dmq9(random))
  }

  #[test]
  fn test_mpq1() -> Result<()> {
    run_case(|case, random| case.test_mpq1(random))
  }

  #[test]
  fn test_mpq2() -> Result<()> {
    run_case(|case, random| case.test_mpq2(random))
  }

  #[test]
  fn test_mpq3() -> Result<()> {
    run_case(|case, random| case.test_mpq3(random))
  }

  #[test]
  fn test_mpq4() -> Result<()> {
    run_case(|case, random| case.test_mpq4(random))
  }

  #[test]
  fn test_mpq5() -> Result<()> {
    run_case(|case, random| case.test_mpq5(random))
  }

  #[test]
  fn test_mpq6() -> Result<()> {
    run_case(|case, random| case.test_mpq6(random))
  }

  #[test]
  fn test_bq1() -> Result<()> {
    run_case(|case, random| case.test_bq1(random))
  }

  #[test]
  fn test_bq2() -> Result<()> {
    run_case(|case, random| case.test_bq2(random))
  }

  #[test]
  fn test_bq3() -> Result<()> {
    run_case(|case, random| case.test_bq3(random))
  }

  #[test]
  fn test_bq4() -> Result<()> {
    run_case(|case, random| case.test_bq4(random))
  }

  #[test]
  fn test_bq5() -> Result<()> {
    run_case(|case, random| case.test_bq5(random))
  }

  #[test]
  fn test_bq6() -> Result<()> {
    run_case(|case, random| case.test_bq6(random))
  }

  #[test]
  fn test_bq7() -> Result<()> {
    run_case(|case, random| case.test_bq7(random))
  }

  #[test]
  fn test_bq8() -> Result<()> {
    run_case(|case, random| case.test_bq8(random))
  }

  #[test]
  fn test_bq9() -> Result<()> {
    run_case(|case, random| case.test_bq9(random))
  }

  #[test]
  fn test_bq10() -> Result<()> {
    run_case(|case, random| case.test_bq10(random))
  }

  #[test]
  fn test_bq11() -> Result<()> {
    run_case(|case, random| case.test_bq11(random))
  }

  #[test]
  fn test_bq14() -> Result<()> {
    run_case(|case, random| case.test_bq14(random))
  }

  #[test]
  fn test_bq15() -> Result<()> {
    run_case(|case, random| case.test_bq15(random))
  }

  #[test]
  fn test_bq16() -> Result<()> {
    run_case(|case, random| case.test_bq16(random))
  }

  #[test]
  fn test_bq17() -> Result<()> {
    run_case(|case, random| case.test_bq17(random))
  }

  #[test]
  fn test_bq19() -> Result<()> {
    run_case(|case, random| case.test_bq19(random))
  }

  #[test]
  fn test_bq20() -> Result<()> {
    run_case(|case, random| case.test_bq20(random))
  }

  #[test]
  fn test_bq21() -> Result<()> {
    run_case(|case, random| case.test_bq21(random))
  }

  #[test]
  fn test_bq23() -> Result<()> {
    run_case(|case, random| case.test_bq23(random))
  }

  #[test]
  fn test_bq24() -> Result<()> {
    run_case(|case, random| case.test_bq24(random))
  }

  #[test]
  fn test_bq25() -> Result<()> {
    run_case(|case, random| case.test_bq25(random))
  }

  #[test]
  fn test_bq26() -> Result<()> {
    run_case(|case, random| case.test_bq26(random))
  }

  #[test]
  fn test_multi_field_bq1() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq1(random))
  }

  #[test]
  fn test_multi_field_bq2() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq2(random))
  }

  #[test]
  fn test_multi_field_bq3() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq3(random))
  }

  #[test]
  fn test_multi_field_bq4() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq4(random))
  }

  #[test]
  fn test_multi_field_bq5() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq5(random))
  }

  #[test]
  fn test_multi_field_bq6() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq6(random))
  }

  #[test]
  fn test_multi_field_bq7() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq7(random))
  }

  #[test]
  fn test_multi_field_bq8() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq8(random))
  }

  #[test]
  fn test_multi_field_bq9() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq9(random))
  }

  #[test]
  fn test_multi_field_bq10() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bq10(random))
  }

  #[test]
  fn test_multi_field_bqof_pq1() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq1(random))
  }

  #[test]
  fn test_multi_field_bqof_pq2() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq2(random))
  }

  #[test]
  fn test_multi_field_bqof_pq3() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq3(random))
  }

  #[test]
  fn test_multi_field_bqof_pq4() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq4(random))
  }

  #[test]
  fn test_multi_field_bqof_pq5() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq5(random))
  }

  #[test]
  fn test_multi_field_bqof_pq6() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq6(random))
  }

  #[test]
  fn test_multi_field_bqof_pq7() -> Result<()> {
    run_case(|case, random| case.test_multi_field_bqof_pq7(random))
  }

  #[test]
  fn test_synonym_query() -> Result<()> {
    run_case(|case, random| case.test_synonym_query(random))
  }

  #[test]
  fn test_equality() -> Result<()> {
    run_case(|case, _random| case.test_equality())
  }
}
