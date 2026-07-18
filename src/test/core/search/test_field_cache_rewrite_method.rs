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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::doc_values_rewrite_method::DocValuesRewriteMethod;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{ConstantScoreBlendedRewrite, ConstantScoreRewrite};
use crate::core::search::query::IntoQuery;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::util::automation::automaton_provider::DefaultProvider;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::search::test_regexp_random2::TestRegexpRandom2;
use crate::test_framework::core::search::check_hits::CheckHits;
use crate::test_framework::core::search::query_utils::QueryUtils;
use crate::test_framework::core::util::lucene_test_case::random;
use rand::prelude::StdRng;

/// Tests the FieldcacheRewriteMethod with random regular expressions
#[allow(dead_code)] // for quick search
struct TestFieldCacheRewriteMethod;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestFieldCacheRewriteMethod, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestFieldCacheRewriteMethod;
  f(&case, &mut random)
}

impl TestRegexpRandom2 for TestFieldCacheRewriteMethod {
  /// Test fieldcache rewrite against filter rewrite
  fn assert_same<IRC>(
    &self,
    searcher1: &IndexSearcher<IRC>,
    searcher2: &IndexSearcher<IRC>,
    _searcher3: &IndexSearcher<IRC>,
    field_name: &str,
    regexp: String,
  ) -> Result<()>
  where
    IRC: IndexReaderContext + Sync,
  {
    let field_cache = RegexpQuery::with_all(
      Term::from_text(field_name, regexp.clone()),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;

    let filter = RegexpQuery::with_all(
      Term::from_text(field_name, regexp.clone()),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      ConstantScoreRewrite,
    )?;

    let filter2 = RegexpQuery::with_all(
      Term::from_text(field_name, regexp),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      ConstantScoreBlendedRewrite,
    )?;

    let field_cache_docs = searcher1.search(field_cache.clone(), 25)?;
    let filter_docs = searcher2.search(filter, 25)?;
    let filter2_docs = searcher2.search(filter2, 25)?;

    CheckHits::check_equal(
      &field_cache.clone().into_query(),
      &field_cache_docs.score_docs,
      &filter_docs.score_docs,
    )?;
    CheckHits::check_equal(
      &field_cache.into_query(),
      &field_cache_docs.score_docs,
      &filter2_docs.score_docs,
    )
  }
}

#[test]
fn test_regexps() -> Result<()> {
  run_case(|case, random| case.test_regexps(random))
}

#[test]
fn test_equals() -> Result<()> {
  let field_name = "field";

  {
    let a1 = RegexpQuery::with_flags(Term::from_text(field_name, "[aA]"), RegExp::NONE)?;
    let a2 = RegexpQuery::with_flags(Term::from_text(field_name, "[aA]"), RegExp::NONE)?;
    let b = RegexpQuery::with_flags(Term::from_text(field_name, "[bB]"), RegExp::NONE)?;
    QueryUtils::check_equal(&a1, &a2);
    QueryUtils::check_unequal(&a1, &b);
    QueryUtils::check_from_query(&a1.into_query());
  }

  {
    let a1 = RegexpQuery::with_all(
      Term::from_text(field_name, "[aA]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    let a2 = RegexpQuery::with_all(
      Term::from_text(field_name, "[aA]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    let b = RegexpQuery::with_all(
      Term::from_text(field_name, "[bB]"),
      RegExp::NONE,
      0,
      &DefaultProvider,
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      DocValuesRewriteMethod,
    )?;
    QueryUtils::check_equal(&a1, &a2);
    QueryUtils::check_unequal(&a1, &b);
    QueryUtils::check_from_query(&a1.into_query());
  }

  Ok(())
}
