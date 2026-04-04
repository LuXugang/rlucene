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
use crate::core::index::index_reader::Identity;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::search::abstract_multi_term_query_constant_score_wrapper::BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boolean_query::Builder;
use crate::core::search::disjunction_max_bulk_scorer::DisjunctionMaxBulkScorer;
use crate::core::search::disjunction_max_scorer::DisjunctionMaxScorer;
use crate::core::search::disjunction_scorer::DisjunctionScorer;
use crate::core::search::explanation::Explanation;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
use crate::core::search::matches_utils::MatchWithNoTerms;
use crate::core::search::query::{
  Query, QueryBase, QueryWeight, QueryWeightSs, QueryWeightSsBulkScorer, QueryWeightSsScorer,
};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::search::scorer_supplier::ScorerSupplier;
use crate::core::search::segment_cacheable::SegmentCacheable;
use crate::core::search::weight::Weight;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

/// A query that generates the union of documents produced by its subqueries, and that scores each
/// document with the maximum score for that document as produced by any subquery, plus a tie
/// breaking increment for any additional matching subqueries. This is useful when searching for a
/// word in multiple fields with different boost factors (so that the fields cannot be combined
/// equivalently into a single search field). We want the primary score to be the one associated with
/// the highest boost, not the sum of the field scores (as BooleanQuery would give). If the query is
/// "albino elephant" this ensures that "albino" matching one field and "elephant" matching another
/// gets a higher score than "albino" matching both fields. To get this result, use both BooleanQuery
/// and DisjunctionMaxQuery: for each term a DisjunctionMaxQuery searches for it in each field, while
/// the set of these DisjunctionMaxQuery's is combined into a BooleanQuery. The tie breaker
/// capability allows results that include the same term in multiple fields to be judged better than
/// results that include this term in only the best of those multiple fields, without confusing this
/// with the better case of two different terms in the multiple fields.
#[derive(Clone, Debug)]
pub struct DisjunctionMaxQuery {
  disjuncts: HashMap<Query, usize>,
  tie_breaker_multiplier: f32,
  ordered_queries: Vec<Query>,
  id: Identity,
}
// TODO IMPORTANT fix this warning
#[allow(clippy::mutable_key_type)]
impl DisjunctionMaxQuery {
  /// Creates a new DisjunctionMaxQuery
  ///
  /// # Parameters
  ///
  /// - `disjuncts`: a `Collection<Query>` of all the disjuncts to add
  /// - `tie_breaker_multiplier`: the score of each non-maximum disjunct for a document is multiplied
  ///   by this weight and added into the final score. If non-zero, the value should be small, on
  ///   the order of 0.1, which says that 10 occurrences of word in a lower-scored field that is
  ///   also in a higher scored field is just as good as a unique word in the lower scored field
  ///   (i.e., one that is not in any higher scored field.
  pub fn new(disjuncts: Vec<Query>, tie_breaker_multiplier: f32) -> Result<Self> {
    if !(0.0..=1.0).contains(&tie_breaker_multiplier) {
      return Err(LuceneError::illegal_argument(
        "tie_breaker_multiplier must be in [0, 1]",
      ));
    }

    let mut multiset = HashMap::new();
    for query in disjuncts.iter() {
      *multiset.entry(query.clone()).or_insert(0usize) += 1;
    }

    Ok(Self {
      disjuncts: multiset,
      tie_breaker_multiplier,
      ordered_queries: disjuncts,
      id: Identity::new(),
    })
  }
  pub fn get_disjuncts(&self) -> &HashMap<Query, usize> {
    &self.disjuncts
  }
  pub fn get_tie_breaker_multiplier(&self) -> f32 {
    self.tie_breaker_multiplier
  }
}

impl HasIdentity for DisjunctionMaxQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for DisjunctionMaxQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    let mut parts = Vec::with_capacity(self.ordered_queries.len());

    for subquery in &self.ordered_queries {
      let s = if matches!(subquery, Query::Boolean(_)) {
        format!("({})", subquery.as_string(field)?)
      } else {
        subquery.as_string(field)?
      };
      parts.push(s);
    }

    let mut result = format!("({})", parts.join(" | "));

    if self.tie_breaker_multiplier != 0.0 {
      result.push('~');
      result.push_str(&format!("{:.1}", self.tie_breaker_multiplier));
    }

    Ok(result)
  }

  fn create_weight<IRC>(
    self,
    searcher: &IndexSearcher<IRC>,
    score_mode: &ScoreMode,
    boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Ok(Box::new(DisjunctionMaxWeight::new(
      searcher,
      self,
      *score_mode,
      boost,
    )))
  }

  fn rewrite<IRC>(mut self, index_searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    if self.ordered_queries.is_empty() {
      return Ok(MatchNoDocsQuery::with_reason("empty DisjunctionMaxQuery").into());
    }

    if self.ordered_queries.len() == 1 {
      return Ok(self.ordered_queries.pop().unwrap());
    }

    if self.tie_breaker_multiplier == 1.0 {
      let mut builder = Builder::new();
      for sub in self.ordered_queries {
        builder.add(sub, Occur::Should)?;
      }
      return Ok(builder.build().into());
    }

    let mut actually_rewritten = false;
    let mut rewritten_disjuncts = Vec::with_capacity(self.ordered_queries.len());
    for sub in self.ordered_queries {
      let sub_id = sub.identity().clone();
      let rewritten_sub = sub.rewrite(index_searcher)?;
      actually_rewritten |= rewritten_sub.identity() != &sub_id;
      rewritten_disjuncts.push(rewritten_sub);
    }

    if actually_rewritten {
      Ok(DisjunctionMaxQuery::new(rewritten_disjuncts, self.tie_breaker_multiplier)?.into())
    } else {
      self.ordered_queries = rewritten_disjuncts;
      Ok(self.into())
    }
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}
impl PartialEq for DisjunctionMaxQuery {
  fn eq(&self, other: &Self) -> bool {
    self.tie_breaker_multiplier == other.tie_breaker_multiplier && self.disjuncts == other.disjuncts
  }
}

impl Eq for DisjunctionMaxQuery {}

impl Hash for DisjunctionMaxQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.tie_breaker_multiplier.to_bits().hash(state);

    let mut entries: Vec<_> = self.disjuncts.iter().collect();
    entries.sort_by(|a, b| {
      // compare name
      let cmp = a.0.name().cmp(b.0.name());
      if cmp != std::cmp::Ordering::Equal {
        return cmp;
      }
      // compare hash
      let mut ah = DefaultHasher::new();
      a.0.hash(&mut ah);
      let ah = ah.finish();

      let mut bh = DefaultHasher::new();
      b.0.hash(&mut bh);
      let bh = bh.finish();

      // compare count
      let cmp = ah.cmp(&bh);
      if cmp != std::cmp::Ordering::Equal {
        return cmp;
      }
      a.1.cmp(b.1)
    });

    for (query, count) in entries {
      query.hash(state);
      count.hash(state);
    }
  }
}
/// the Weight for DisjunctionMaxQuery, used to normalize, score and explain these queries
pub struct DisjunctionMaxWeight<IRC>
where
  IRC: IndexReaderContext,
{
  parent_query: Arc<Query>,
  tie_breaker_multiplier: f32,
  score_mode: ScoreMode,
  weights: Vec<QueryWeight<IRC>>,
}
impl<IRC> DisjunctionMaxWeight<IRC>
where
  IRC: IndexReaderContext,
{
  pub fn new(
    searcher: &IndexSearcher<IRC>,
    query: DisjunctionMaxQuery,
    score_mode: ScoreMode,
    boost: f32,
  ) -> Self {
    let mut weights = Vec::with_capacity(query.get_disjuncts().len());
    for (query, _) in query.disjuncts.clone() {
      let weight = query.create_weight(searcher, &score_mode, boost).unwrap();
      weights.push(weight);
    }
    let tie_breaker_multiplier = query.get_tie_breaker_multiplier();
    Self {
      parent_query: Arc::new(query.into()),
      tie_breaker_multiplier,
      score_mode,
      weights,
    }
  }
}

impl<IRC> SegmentCacheable<IRC> for DisjunctionMaxWeight<IRC>
where
  IRC: IndexReaderContext,
{
  fn is_cacheable(&self, ctx: &LeafReaderContext<IRCLeafReader<IRC>>) -> Result<bool> {
    if self.weights.len() > BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD {
      // Disallow caching large dismax queries to not encourage users
      // to build large dismax queries as a workaround to the fact that
      // we disallow caching large TermInSetQueries.
      return Ok(false);
    }

    for w in &self.weights {
      if !w.is_cacheable(ctx)? {
        return Ok(false);
      }
    }

    Ok(true)
  }
}

impl<IRC> Weight<IRC> for DisjunctionMaxWeight<IRC>
where
  IRC: IndexReaderContext,
{
  type Matches = MatchWithNoTerms;

  fn matches(
    &self,
    _context: &LeafReaderContext<IRCLeafReader<IRC>>,
    _doc: i32,
    _searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::Matches>> {
    todo!()
  }

  fn explain(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    doc: i32,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Explanation> {
    let mut matched = false;
    let mut max = 0.0f64;
    let mut other_sum = 0.0f64;
    let mut subs_on_match = Vec::new();
    let mut subs_on_no_match = Vec::new();

    for wt in &self.weights {
      let e = wt.explain(context, doc, searcher)?;
      if e.is_match() {
        matched = true;
        subs_on_match.push(e.clone());
        let score = e.get_value().to_f64().ok_or_else(|| {
          LuceneError::illegal_state(format!(
            "Explanation value is not a number: {:?}",
            e.get_value()
          ))
        })?;
        if score >= max {
          other_sum += max;
          max = score;
        } else {
          other_sum += score;
        }
      } else if !matched {
        subs_on_no_match.push(e);
      }
    }

    if matched {
      let score = (max + other_sum * self.tie_breaker_multiplier as f64) as f32;
      let desc = if self.tie_breaker_multiplier == 0.0 {
        "max of:".to_string()
      } else {
        format!("max plus {} times others of:", self.tie_breaker_multiplier)
      };
      Ok(Explanation::match_(score, desc, subs_on_match))
    } else {
      Ok(Explanation::no_match(
        "No matching clause",
        subs_on_no_match,
      ))
    }
  }

  fn get_query(&self) -> Arc<Query> {
    self.parent_query.clone()
  }

  type ScorerSupplier = QueryWeightSs<IRC>;

  fn scorer_supplier(
    &self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::ScorerSupplier>> {
    let mut scorer_suppliers = Vec::new();
    for w in &self.weights {
      let ss = w.scorer_supplier(context, searcher)?;
      if let Some(ss) = ss {
        scorer_suppliers.push(ss);
      }
    }

    if scorer_suppliers.is_empty() {
      Ok(None)
    } else if scorer_suppliers.len() == 1 {
      Ok(Some(scorer_suppliers.pop().unwrap()))
    } else {
      let v = ScorerSupplierImpl::new(
        -1,
        scorer_suppliers,
        self.tie_breaker_multiplier,
        self.score_mode,
      );
      Ok(Some(Box::new(v)))
    }
  }
}

pub struct ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  cost: i64,
  scorer_suppliers: Vec<QueryWeightSs<IRC>>,
  tie_breaker_multiplier: f32,
  score_mode: ScoreMode,
}
impl<IRC> ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  pub fn new(
    cost: i64,
    scorer_suppliers: Vec<QueryWeightSs<IRC>>,
    tie_breaker_multiplier: f32,
    score_mode: ScoreMode,
  ) -> Self {
    Self {
      cost,
      scorer_suppliers,
      tie_breaker_multiplier,
      score_mode,
    }
  }
}
impl<IRC> ScorerSupplier<IRC> for ScorerSupplierImpl<IRC>
where
  IRC: IndexReaderContext,
{
  type Scorer = QueryWeightSsScorer;
  type BulkScorer = QueryWeightSsBulkScorer;

  fn get(
    &mut self,
    lead_cost: i64,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Self::Scorer> {
    let mut scorers = Vec::with_capacity(self.scorer_suppliers.len());
    for ss in self.scorer_suppliers.iter_mut() {
      scorers.push(ss.get(lead_cost, context, searcher)?);
    }
    let sub =
      DisjunctionMaxScorer::new(self.tie_breaker_multiplier, &mut scorers, self.score_mode)?;
    let v = DisjunctionScorer::new(scorers, self.score_mode, sub)?;
    Ok(Box::new(v))
  }

  fn bulk_scorer(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<Option<Self::BulkScorer>> {
    if self.tie_breaker_multiplier == 0.0 && self.score_mode == ScoreMode::TopScores {
      let mut scorers = Vec::with_capacity(self.scorer_suppliers.len());
      for ss in self.scorer_suppliers.iter_mut() {
        if let Some(scorer) = ss.bulk_scorer(context, searcher)? {
          scorers.push(scorer);
        }
      }
      return Ok(Some(Box::new(DisjunctionMaxBulkScorer::new(scorers)?)));
    }
    Ok(Some(Box::new(self.default_bulk_scorer(context, searcher)?)))
  }

  fn cost(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
    searcher: &IndexSearcher<IRC>,
  ) -> Result<i64> {
    if self.cost == -1 {
      let mut cost = 0i64;
      for ss in self.scorer_suppliers.iter_mut() {
        cost += ss.cost(context, searcher)?;
      }
      self.cost = cost;
    }
    Ok(self.cost)
  }

  fn set_top_level_scoring_clause(&mut self) -> Result<()> {
    if self.tie_breaker_multiplier == 0.0 {
      for ss in self.scorer_suppliers.iter_mut() {
        // sub scorers need to be able to skip too as calls to setMinCompetitiveScore get
        // propagated
        ss.set_top_level_scoring_clause()?;
      }
    }
    Ok(())
  }
}
#[cfg(test)]
pub(crate) mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::document::text_field::{TextField, text_field_type};
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::stored_fields::StoredFields;
  use crate::core::index::term::Term;
  use crate::core::search::boost_query::BoostQuery;
  use crate::core::search::collection_statistics::CollectionStatistics;
  use crate::core::search::disjunction_max_query::DisjunctionMaxQuery;
  use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
  use crate::core::search::explanation::Explanation;
  use crate::core::search::index_searcher::IndexSearcher;
  use crate::core::search::query::{Query, QueryBase};

  use crate::core::document::string_field::StringField;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::IndexWriterConfig;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::match_no_docs_query::MatchNoDocsQuery;
  use crate::core::search::score_mode::ScoreMode;
  use crate::core::search::scorer::Scorer;
  use crate::core::search::similarities_impl::classic_similarity::idf_explain;
  use crate::core::search::similarities_impl::tf_idf_similarity::{
    TFIDFSimilarity, TFIDFSimilarityBase, TFIDFSubEnum,
  };
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::term_statistics::TermStatistics;
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::search::check_hits::CheckHits;
  use crate::test::core::search::query_utils::QueryUtils;
  use crate::test::core::util::DefaultIndexSearchLR;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    at_least, get_only_leaf_reader, is_night_mode, new_directory_shared, new_field,
    new_index_writer_config_with_analyzer, new_log_merge_policy, new_searcher_with_reader,
    new_text_field, random,
  };
  use rand::{Rng, RngExt};
  use std::collections::HashMap;

  #[allow(dead_code)] //for quick search
  struct TestDisjunctionMaxQuery;
  const SCORE_COMP_THRESH: f32 = 0.0000f32;
  fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchLR>
  where
    R: Rng + ?Sized,
  {
    let index = new_directory_shared(random)?;

    let analyzer = MockAnalyzer::new(random);
    let mut iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let sim = TestSimilarity::new();
    iwc.set_similarity(sim.clone());
    iwc.set_merge_policy(new_log_merge_policy(random)?);

    let writer = RandomIndexWriter::with_config(random, index, iwc);
    let mut field_to_type = HashMap::new();
    let mut non_analyzed_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    non_analyzed_type.set_tokenized(false)?;

    {
      let mut d1 = Document::new();
      d1.add(new_field(
        random,
        "id",
        "d1",
        &non_analyzed_type.clone(),
        &mut field_to_type,
      )?);
      d1.add(new_text_field(
        random,
        "hed",
        "elephant",
        Store::Yes,
        &mut field_to_type,
      )?);
      d1.add(new_text_field(
        random,
        "dek",
        "elephant",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(d1)?;
    }

    // d2
    {
      let mut d2 = Document::new();
      d2.add(new_field(
        random,
        "id",
        "d2",
        &non_analyzed_type.clone(),
        &mut field_to_type,
      )?);
      d2.add(new_text_field(
        random,
        "hed",
        "elephant",
        Store::Yes,
        &mut field_to_type,
      )?);
      d2.add(new_text_field(
        random,
        "dek",
        "albino",
        Store::Yes,
        &mut field_to_type,
      )?);
      d2.add(new_text_field(
        random,
        "dek",
        "elephant",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(d2)?;
    }

    // d3
    {
      let mut d3 = Document::new();
      d3.add(new_field(
        random,
        "id",
        "d3",
        &non_analyzed_type.clone(),
        &mut field_to_type,
      )?);
      d3.add(new_text_field(
        random,
        "hed",
        "albino",
        Store::Yes,
        &mut field_to_type,
      )?);
      d3.add(new_text_field(
        random,
        "hed",
        "elephant",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(d3)?;
    }

    // d4
    {
      let mut d4 = Document::new();
      d4.add(new_field(
        random,
        "id",
        "d4",
        &non_analyzed_type.clone(),
        &mut field_to_type,
      )?);
      d4.add(new_text_field(
        random,
        "hed",
        "albino",
        Store::Yes,
        &mut field_to_type,
      )?);
      d4.add(new_field(
        random,
        "hed",
        "elephant",
        &non_analyzed_type.clone(),
        &mut field_to_type,
      )?);
      d4.add(new_text_field(
        random,
        "dek",
        "albino",
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(d4)?;
    }

    writer.force_merge(1)?;

    let reader = writer.get_reader()?;
    let r = get_only_leaf_reader(&reader)?;
    writer.close()?;

    let mut s = IndexSearcher::from_lr(r)?;
    s.set_similarity(sim);

    Ok(s)
  }
  #[test]
  fn test_skip_to_firsttime_miss() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;
    let dq = DisjunctionMaxQuery::new(
      vec![tq("id", "d1").into(), tq("dek", "DOES_NOT_EXIST").into()],
      0.0,
    )?;

    QueryUtils::check_from_searcher(&mut random, dq.clone(), &s)?;

    let leaves = s.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let rewritten = s.rewrite(dq)?;
    let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let mut scorer = weight.scorer(ctx, &s)?.unwrap();

    let skip_ok = scorer.iterator_mut().advance(3)? != NO_MORE_DOCS;

    if skip_ok {
      let doc = scorer.doc_id()?;
      let stored = s.reader_context.reader().stored_fields()?.document(doc)?;
      unreachable!(
        "firsttime skipTo found a match? ... {}",
        stored.get("id")?.unwrap()
      );
    }

    Ok(())
  }

  #[test]
  fn test_skip_to_firsttime_hit() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let dq = DisjunctionMaxQuery::new(
      vec![
        tq("dek", "albino").into(),
        tq("dek", "DOES_NOT_EXIST").into(),
      ],
      0.0,
    )?;

    QueryUtils::check_from_searcher(&mut random, dq.clone(), &s)?;

    let leaves = s.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let rewritten = s.rewrite(dq)?;
    let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let mut ds = weight.scorer(ctx, &s)?.unwrap();

    let hit = ds.iterator_mut().advance(3)? != NO_MORE_DOCS;
    assert!(hit, "firsttime skipTo found no match");

    let doc = ds.doc_id()?;
    let stored = s.reader_context.reader().stored_fields()?.document(doc)?;
    assert_eq!(
      "d4",
      stored.get("id")?.unwrap().as_ref(),
      "found wrong docid"
    );

    Ok(())
  }
  #[test]
  fn test_simple_equal_scores1() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let q = DisjunctionMaxQuery::new(
      vec![tq("hed", "albino").into(), tq("hed", "elephant").into()],
      0.0,
    )?;

    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(4, h.len(), "all docs should match {}", q.as_string("")?);
    let score = h[0].score;
    for (i, item) in h.iter().enumerate().skip(1) {
      assert!(
        (score - item.score).abs() <= SCORE_COMP_THRESH,
        "score #{} is not the same",
        i
      );
    }
    Ok(())
  }
  #[test]
  fn test_simple_equal_scores2() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let q = DisjunctionMaxQuery::new(
      vec![tq("dek", "albino").into(), tq("dek", "elephant").into()],
      0.0,
    )?;

    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(3, h.len(), "3 docs should match {}", q.as_string("")?);
    let score = h[0].score;
    for (i, item) in h.iter().enumerate().skip(1) {
      assert!(
        (score - item.score).abs() <= SCORE_COMP_THRESH,
        "score #{} is not the same",
        i
      );
    }

    Ok(())
  }

  #[test]
  fn test_simple_equal_scores3() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let q = DisjunctionMaxQuery::new(
      vec![
        tq("hed", "albino").into(),
        tq("hed", "elephant").into(),
        tq("dek", "albino").into(),
        tq("dek", "elephant").into(),
      ],
      0.0,
    )?;

    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(4, h.len(), "all docs should match {}", q.as_string("")?);
    let score = h[0].score;
    for (i, sd) in h.iter().enumerate().skip(1) {
      assert!(
        (score - sd.score).abs() <= SCORE_COMP_THRESH,
        "score #{} is not the same",
        i
      );
    }

    Ok(())
  }

  #[test]
  fn test_simple_tiebreaker() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let q = DisjunctionMaxQuery::new(
      vec![tq("dek", "albino").into(), tq("dek", "elephant").into()],
      0.01,
    )?;

    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(3, h.len(), "3 docs should match {}", q.as_string("")?);

    let mut stored_fields = s.stored_fields()?;
    let first_doc = stored_fields.document(h[0].doc)?;
    assert_eq!("d2", first_doc.get("id")?.unwrap().as_ref(), "wrong first");

    let score0 = h[0].score;
    let score1 = h[1].score;
    let score2 = h[2].score;

    assert!(
      score0 > score1,
      "d2 does not have better score then others: {} >? {}",
      score0,
      score1
    );

    assert!(
      (score1 - score2).abs() <= SCORE_COMP_THRESH,
      "d4 and d1 don't have equal scores"
    );

    Ok(())
  }

  #[test]
  fn test_boolean_required_equal_scores() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let mut builder = Builder::new();

    {
      let q1 = DisjunctionMaxQuery::new(
        vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
        0.0,
      )?;
      builder.add(q1.clone(), Occur::Must)?;
      QueryUtils::check_from_searcher(&mut random, q1.clone(), &s)?;
    }

    {
      let q2 = DisjunctionMaxQuery::new(
        vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
        0.0,
      )?;
      builder.add(q2.clone(), Occur::Must)?;
      QueryUtils::check_from_searcher(&mut random, q2.clone(), &s)?;
    }

    let q = builder.build();
    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(3, h.len(), "3 docs should match {}", q.as_string("")?);

    let score = h[0].score;
    for (i, sd) in h.iter().enumerate().skip(1) {
      assert!(
        (score - sd.score).abs() <= SCORE_COMP_THRESH,
        "score #{} is not the same",
        i
      );
    }

    Ok(())
  }

  #[test]
  fn test_boolean_optional_no_tiebreaker() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let mut builder = Builder::new();

    {
      let q1 = DisjunctionMaxQuery::new(
        vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
        0.0,
      )?;
      builder.add(q1.clone(), Occur::Should)?;
    }

    {
      let q2 = DisjunctionMaxQuery::new(
        vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
        0.0,
      )?;
      builder.add(q2.clone(), Occur::Should)?;
    }

    let q = builder.build();
    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(4, h.len(), "4 docs should match {}", q.as_string("")?);

    let score = h[0].score;
    for (i, sd) in h.iter().enumerate().skip(1).take(h.len().saturating_sub(2)) {
      assert!(
        (score - sd.score).abs() <= SCORE_COMP_THRESH,
        "score #{} is not the same",
        i
      );
    }

    let mut stored_fields = s.stored_fields()?;
    let last_doc = stored_fields.document(h[h.len() - 1].doc)?;
    assert_eq!("d1", last_doc.get("id")?.unwrap().as_ref(), "wrong last");

    let score1 = h[h.len() - 1].score;
    assert!(
      score > score1,
      "d1 does not have worse score then others: {} >? {}",
      score,
      score1
    );

    Ok(())
  }

  #[test]
  fn test_boolean_optional_with_tiebreaker() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let mut builder = Builder::new();

    {
      let q1 = DisjunctionMaxQuery::new(
        vec![tq("hed", "albino").into(), tq("dek", "albino").into()],
        0.01,
      )?;
      builder.add(q1, Occur::Should)?;
    }

    {
      let q2 = DisjunctionMaxQuery::new(
        vec![tq("hed", "elephant").into(), tq("dek", "elephant").into()],
        0.01,
      )?;
      builder.add(q2, Occur::Should)?;
    }

    let q = builder.build();
    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(4, h.len(), "4 docs should match {}", q.as_string("")?);

    let score0 = h[0].score;
    let score1 = h[1].score;
    let score2 = h[2].score;
    let score3 = h[3].score;

    let mut stored_fields = s.stored_fields()?;
    let doc0 = stored_fields
      .document(h[0].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc1 = stored_fields
      .document(h[1].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc2 = stored_fields
      .document(h[2].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc3 = stored_fields
      .document(h[3].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();

    assert!(
      doc0 == "d2" || doc0 == "d4",
      "doc0 should be d2 or d4: {}",
      doc0
    );
    assert!(
      doc1 == "d2" || doc1 == "d4",
      "doc1 should be d2 or d4: {}",
      doc1
    );

    assert!(
      (score0 - score1).abs() <= SCORE_COMP_THRESH,
      "score0 and score1 should match"
    );

    assert_eq!("d3", doc2, "wrong third");
    assert!(
      score1 > score2,
      "d3 does not have worse score then d2 and d4: {} >? {}",
      score1,
      score2
    );

    assert_eq!("d1", doc3, "wrong fourth");
    assert!(
      score2 > score3,
      "d1 does not have worse score then d3: {} >? {}",
      score2,
      score3
    );

    Ok(())
  }

  #[test]
  fn test_boolean_optional_with_tiebreaker_and_boost() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let mut builder = Builder::new();

    {
      let q1 = DisjunctionMaxQuery::new(
        vec![
          tq_with_boost("hed", "albino", 1.5)?.into(),
          tq("dek", "albino").into(),
        ],
        0.01,
      )?;
      builder.add(q1, Occur::Should)?;
    }

    {
      let q2 = DisjunctionMaxQuery::new(
        vec![
          tq_with_boost("hed", "elephant", 1.5)?.into(),
          tq("dek", "elephant").into(),
        ],
        0.01,
      )?;
      builder.add(q2, Occur::Should)?;
    }

    let q = builder.build();
    QueryUtils::check_from_searcher(&mut random, q.clone(), &s)?;

    let h = s.search(q.clone(), 1000)?.score_docs;

    assert_eq!(4, h.len(), "4 docs should match {}", q.as_string("")?);

    let score0 = h[0].score;
    let score1 = h[1].score;
    let score2 = h[2].score;
    let score3 = h[3].score;

    let mut stored_fields = s.stored_fields()?;
    let doc0 = stored_fields
      .document(h[0].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc1 = stored_fields
      .document(h[1].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc2 = stored_fields
      .document(h[2].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();
    let doc3 = stored_fields
      .document(h[3].doc)?
      .get("id")?
      .unwrap()
      .as_ref()
      .to_string();

    assert_eq!("d4", doc0, "doc0 should be d4:");
    assert_eq!("d3", doc1, "doc1 should be d3:");
    assert_eq!("d2", doc2, "doc2 should be d2:");
    assert_eq!("d1", doc3, "doc3 should be d1:");

    assert!(
      score0 > score1,
      "d4 does not have a better score then d3: {} >? {}",
      score0,
      score1
    );
    assert!(
      score1 > score2,
      "d3 does not have a better score then d2: {} >? {}",
      score1,
      score2
    );
    assert!(
      score2 > score3,
      "d3 does not have a better score then d1: {} >? {}",
      score2,
      score3
    );

    Ok(())
  }

  #[test]
  fn test_rewrite_boolean() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let sub1: Query = tq("hed", "albino").into();
    let sub2: Query = tq("hed", "elephant").into();

    let q = DisjunctionMaxQuery::new(vec![sub1.clone(), sub2.clone()], 1.0)?;

    let rewritten = s.rewrite(q.clone())?;

    let mut builder = Builder::new();
    builder.add(sub1, Occur::Should)?;
    builder.add(sub2, Occur::Should)?;
    let expected: Query = builder.build().into();

    assert_eq!(expected, rewritten);

    Ok(())
  }

  #[test]
  fn test_rewrite_empty() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let q = DisjunctionMaxQuery::new(vec![], 0.0)?;
    let rewritten = s.rewrite(q)?;

    let expected: Query = MatchNoDocsQuery::new().into();

    assert_eq!(expected, rewritten);

    Ok(())
  }

  #[test]
  fn test_disjunct_order_and_equals() -> Result<()> {
    let sub1: Query = tq("hed", "albino").into();
    let sub2: Query = tq("hed", "elephant").into();

    let q1: Query = DisjunctionMaxQuery::new(vec![sub1.clone(), sub2.clone()], 1.0)?.into();
    let q2: Query = DisjunctionMaxQuery::new(vec![sub2, sub1], 1.0)?.into();

    assert_eq!(q1, q2);

    Ok(())
  }

  #[test]
  fn test_to_string_order_matters() -> Result<()> {
    let mut random = random();

    let clause_nbr = random.random_range(4..=25);

    let mut terms = Vec::with_capacity(clause_nbr);
    for i in 0..clause_nbr {
      terms.push(((b'a' + i as u8) as char).to_string());
    }

    let expected = terms
      .iter()
      .map(|term| format!("test:{}", term))
      .collect::<Vec<_>>()
      .join(" | ");
    let expected = format!("({})~1.0", expected);

    let disjuncts: Vec<Query> = terms.iter().map(|term| tq("test", term).into()).collect();

    let source = DisjunctionMaxQuery::new(disjuncts, 1.0)?;

    assert_eq!(expected, source.as_string("")?);

    Ok(())
  }
  // TODO 测试未通过 6234308664746830463
  fn test_random_top_docs() -> Result<()> {
    let mut random = random();
    do_test_random_top_docs(&mut random, 2, &[0.05, 0.05])?;
    do_test_random_top_docs(&mut random, 2, &[1.0, 0.05])?;
    do_test_random_top_docs(&mut random, 3, &[1.0, 0.5, 0.05])?;
    do_test_random_top_docs(&mut random, 4, &[1.0, 0.5, 0.05, 0.0])?;
    do_test_random_top_docs(&mut random, 4, &[1.0, 0.5, 0.05, 0.0])?;
    Ok(())
  }
  #[test]
  fn test_explain_match() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let sub1: Query = tq("hed", "elephant").into();
    let sub2: Query = tq("dek", "elephant").into();

    let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

    let rewritten = s.rewrite(dq)?;
    let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let leaves = s.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let explanation = weight.explain(ctx, 1, &s)?;

    assert_eq!("max of:", explanation.get_description());
    assert_eq!(2, explanation.get_details().len());

    Ok(())
  }
  #[test]
  fn test_explain_no_match() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let sub1: Query = tq("abc", "elephant").into();
    let sub2: Query = tq("def", "elephant").into();

    let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

    let rewritten = s.rewrite(dq)?;
    let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let leaves = s.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let explanation = weight.explain(ctx, 1, &s)?;

    assert_eq!("No matching clause", explanation.get_description());
    assert_eq!(2, explanation.get_details().len());

    Ok(())
  }

  #[test]
  fn test_explain_match_one_non_matching_subquery_not_included_in_explanation() -> Result<()> {
    let mut random = random();
    let s = set_up(&mut random)?;

    let sub1: Query = tq("hed", "elephant").into();
    let sub2: Query = tq("def", "elephant").into();

    let dq = DisjunctionMaxQuery::new(vec![sub1, sub2], 0.0)?;

    let rewritten = s.rewrite(dq)?;
    let weight = s.create_weight(rewritten, ScoreMode::Complete, 1.0)?;

    let leaves = s.get_top_reader_context().leaves()?;
    let ctx = &leaves[0];

    let explanation = weight.explain(ctx, 1, &s)?;

    assert_eq!("max of:", explanation.get_description());
    assert_eq!(1, explanation.get_details().len());

    Ok(())
  }
  fn do_test_random_top_docs<R>(random: &mut R, num_fields: usize, freqs: &[f64]) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    assert_eq!(num_fields, freqs.len());

    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT StandardAnalyzer 未实现
    // let analyzer = StandardAnalyzer::new();
    let iwc = IndexWriterConfig::new();
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let num_docs = if is_night_mode() {
      at_least(random, 1000)
    } else {
      at_least(random, 100)
    };

    for _ in 0..num_docs {
      let mut doc = Document::new();

      for (j, freq) in freqs.iter().take(num_fields).enumerate() {
        let mut builder = String::new();

        let num_as = if random.random::<f64>() < *freq {
          0
        } else {
          1 + random.random_range(0..5)
        };

        for _ in 0..num_as {
          if !builder.is_empty() {
            builder.push(' ');
          }
          builder.push('a');
        }

        if random.random_bool(0.5) {
          doc.add(StringField::from_string("field", "c", Store::No)?);
        }

        let num_others = if random.random_bool(0.5) {
          0
        } else {
          1 + random.random_range(0..5)
        };

        for _ in 0..num_others {
          if !builder.is_empty() {
            builder.push(' ');
          }
          builder.push_str(&random.random::<i32>().to_string());
        }
        // TODO IMPORTANT StreadReader未实现
        doc.add(TextField::from_string(j.to_string(), builder, Store::No)?);
      }

      writer.add_document(doc)?;
    }

    let reader = directory_reader_util::open_from_writer(&writer)?;
    writer.close()?;

    let searcher = new_searcher_with_reader(reader)?;

    for i in 0..4 {
      let mut clauses: Vec<Query> = Vec::new();

      for j in 0..num_fields {
        if i % 2 == 1 {
          clauses.push(tq(&j.to_string(), "a").into());
        } else {
          let boost = if random.random_bool(0.5) {
            0.0
          } else {
            random.random::<f32>()
          };

          if boost > 0.0 {
            clauses.push(tq_with_boost(&j.to_string(), "a", boost)?.into());
          } else {
            clauses.push(tq(&j.to_string(), "a").into());
          }
        }
      }

      let tie_breaker = random.random::<f32>();
      let query: Query = DisjunctionMaxQuery::new(clauses.clone(), tie_breaker)?.into();

      CheckHits::check_top_scores(random, &query, &searcher)?;

      let mut builder = Builder::new();
      builder.add(DisjunctionMaxQuery::new(clauses, tie_breaker)?, Occur::Must)?;
      builder.add(tq("field", "c"), Occur::Filter)?;

      let query: Query = builder.build().into();

      CheckHits::check_top_scores(random, &query, &searcher)?;
    }
    Ok(())
  }
  fn tq(field: &str, term: &str) -> TermQuery {
    TermQuery::new(Term::from_text(field, term))
  }

  fn tq_with_boost(field: &str, term: &str, boost: f32) -> Result<BoostQuery> {
    let q = tq(field, term);
    BoostQuery::new(q, boost)
  }

  #[derive(Clone, Default)]
  pub struct TestSimilarity;

  impl TestSimilarity {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> TFIDFSimilarity {
      let v = TFIDFSubEnum::Test(TestSimilarity);
      TFIDFSimilarity::new(v)
    }
  }

  impl TFIDFSimilarityBase for TestSimilarity {
    fn tf(&self, freq: f32) -> f32 {
      if freq > 0.0 { 1.0 } else { 0.0 }
    }

    fn idf_explain(
      &self,
      collection_stats: &CollectionStatistics,
      term_stats: &TermStatistics,
    ) -> Explanation {
      idf_explain(self, collection_stats, term_stats)
    }

    fn idf(&self, _doc_freq: i64, _doc_count: i64) -> f32 {
      1f32
    }

    fn length_norm(&self, _length: i32) -> f32 {
      1f32
    }
  }
}
