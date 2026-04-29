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
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::boolean_clause::Occur;
use crate::core::search::boost_attribute::DEFAULT_BOOST;
use crate::core::search::boost_query::BoostQuery;
use crate::core::search::constant_score_query::ConstantScoreQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{MultiTermQuery, MultiTermQueryEnum, RewriteMethod};
use crate::core::search::query::Query;
use crate::core::search::term_collecting_rewrite::{TermCollectingRewrite, TermCollector};
use crate::core::search::term_query::TermQuery;
use crate::core::search::{boolean_query, index_searcher};
use crate::core::util::allocator_byte::DirectAllocatorByte;
use crate::core::util::array_util::ArrayUtil;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::bytes_ref_hash::{BytesRefHash, BytesStartArray, DirectBytesStartArray};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::{ByteBlockPool, SharedCounter};
/// Base rewrite method that translates each term into a query, and keeps the scores as computed by
/// the query.
pub trait ScoringRewrite: TermCollectingRewrite {
  fn default_rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let reader = index_searcher.get_index_reader();
    let mut builder = self.get_top_level_builder()?;

    let mut col = ParallelArraysTermCollector::new(|size| self.check_max_clause_count(size));

    self.collect_terms(index_searcher.get_top_reader_context(), &query, &mut col)?;

    let size = col.terms.size();
    let mut br = BytesRef::new();
    #[allow(clippy::needless_range_loop)]
    if size > 0 {
      col.terms.sort(&col.block_pool)?;
      let sort = col.terms.ids.as_slice();
      for i in 0..(size as usize) {
        let pos = sort[i];
        col.terms.get(pos, &mut br, &col.block_pool);
        let term = Term::new(query.get_field(), std::mem::take(&mut br));
        let term_state = std::mem::take(&mut col.terms.bytes_start_array.term_state[pos as usize]);

        debug_assert_eq!(reader.doc_freq(&term)?, term_state.doc_freq()?);
        let doc_freq = term_state.doc_freq()?;
        self.add_clause_with_states(
          &mut builder,
          term,
          doc_freq,
          col.terms.bytes_start_array.boost[pos as usize],
          Some(term_state),
        )?;
      }
    }

    self.build(builder)
  }
  /// This method is called after every new term to check if the number of max clauses (e.g. in
  /// [`BooleanQuery`]) is not exceeded. Returns the corresponding error.
  fn check_max_clause_count(&self, count: usize) -> Result<()>;
}
/// A rewrite method that first translates each term into [`Occur::Should`] clause
/// in a [`BooleanQuery`], and keeps the scores as computed by the query. Note that typically such
/// scores are meaningless to the user, and require non-trivial CPU to compute, so it's almost
/// always better to use [`MultiTermQuery::CONSTANT_SCORE_BLENDED_REWRITE`] or
/// [`MultiTermQuery::CONSTANT_SCORE_REWRITE`] instead.
///
/// **NOTE**: This rewrite method will hit [`IndexSearcherError::TooManyClauses`] if the number
/// of terms exceeds [`IndexSearcher::get_max_clause_count`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScoringBooleanRewrite;
impl TermCollectingRewrite for ScoringBooleanRewrite {
  type B = boolean_query::Builder;

  fn get_top_level_builder(&self) -> Result<Self::B> {
    Ok(boolean_query::Builder::new())
  }

  fn build(&self, builder: Self::B) -> Result<Query> {
    Ok(builder.build().into())
  }

  fn add_clause_with_states(
    &self,
    top_level: &mut Self::B,
    term: Term,
    _doc_count: i32,
    boost: f32,
    states: Option<TermStates>,
  ) -> Result<()> {
    let tq = TermQuery::with_term_state(term, states);
    top_level.add(BoostQuery::new(tq, boost)?, Occur::Should)?;
    Ok(())
  }
}

impl RewriteMethod for ScoringBooleanRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    self.default_rewrite(index_searcher, query)
  }
}

impl ScoringRewrite for ScoringBooleanRewrite {
  fn check_max_clause_count(&self, count: usize) -> Result<()> {
    if count > index_searcher::get_max_clause_count() {
      return Err(index_searcher::new());
    }
    Ok(())
  }
}
/// Like [`Self::SCORING_BOOLEAN_REWRITE`] except scores are not computed. Instead, each matching
/// document receives a constant score equal to the query's boost.
///
/// **NOTE**: This rewrite method will hit [`IndexSearcherError::TooManyClauses`] if the number
/// of terms exceeds [`IndexSearcher::get_max_clause_count`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConstantScoreBooleanRewrite;
impl RewriteMethod for ConstantScoreBooleanRewrite {
  fn rewrite<IRC, Q>(self, index_searcher: &IndexSearcher<IRC>, query: Q) -> Result<Query>
  where
    Q: MultiTermQuery + Into<MultiTermQueryEnum>,
    IRC: IndexReaderContext,
  {
    let bq = ScoringBooleanRewrite.rewrite(index_searcher, query)?;
    Ok(ConstantScoreQuery::new(bq).into())
  }
}

struct ParallelArraysTermCollector<F>
where
  F: FnMut(usize) -> Result<()>,
{
  terms: BytesRefHash<TermFreqBoostByteStart>,
  block_pool: ByteBlockPool,
  ord: usize,
  check_max_clause_count: F,
}
impl<F> ParallelArraysTermCollector<F>
where
  F: FnMut(usize) -> Result<()>,
{
  fn new(check_max_clause_count: F) -> Self {
    let allocator = DirectAllocatorByte::new();
    let block_pool = ByteBlockPool::new(allocator);
    let array = TermFreqBoostByteStart::new(16);
    let terms = BytesRefHash::from_bytes_start_array(16, array);
    Self {
      terms,
      block_pool,
      ord: 0,
      check_max_clause_count,
    }
  }
}
impl<F> TermCollector for ParallelArraysTermCollector<F>
where
  F: FnMut(usize) -> Result<()>,
{
  fn set_reader_context<IRC>(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
  ) -> Result<()>
  where
    IRC: IndexReaderContext,
  {
    self.ord = context.ord;
    Ok(())
  }

  fn collect<TE, IRC>(
    &mut self,
    bytes: BytesRef<Vec<u8>>,
    terms_enum: &mut TE,
    top_reader_context: &IRC,
  ) -> Result<bool>
  where
    TE: TermsEnum,
    IRC: IndexReaderContext,
  {
    let e = self.terms.add(&bytes, &mut self.block_pool)?;
    let state = terms_enum.term_state()?;

    if e < 0 {
      let pos = (-e - 1) as usize;

      let array = &mut self.terms.bytes_start_array;

      array.term_state[pos].register_with_stats(
        state,
        self.ord,
        terms_enum.doc_freq()?,
        terms_enum.total_term_freq()?,
      );
      debug_assert_eq!(
        array.boost[pos],
        match (|| -> Result<f32> {
          let attr = terms_enum.attributes()?;
          attr.get_boost()
        })() {
          Ok(boost) => boost,
          Err(LuceneError::UnsupportedOperation(_)) => DEFAULT_BOOST,
          Err(e) => return Err(e),
        },
        "boost should be equal in all segment TermsEnums"
      );
    } else {
      let pos = e as usize;
      let array = &mut self.terms.bytes_start_array;
      let boost = match (|| -> Result<f32> {
        let attr = terms_enum.attributes()?;
        attr.get_boost()
      })() {
        Ok(boost) => boost,
        Err(LuceneError::UnsupportedOperation(_)) => DEFAULT_BOOST,
        Err(e) => return Err(e),
      };
      array.boost[pos] = boost;
      array.term_state[pos] = TermStates::with_state_and_stats(
        top_reader_context,
        state,
        self.ord,
        terms_enum.doc_freq()?,
        terms_enum.total_term_freq()?,
      )?;
      (self.check_max_clause_count)(self.terms.size() as usize)?;
    }

    Ok(true)
  }

  fn set_next_enum<TE>(&mut self, _terms_enum: &mut TE) -> Result<()>
  where
    TE: TermsEnum,
  {
    Ok(())
  }
}
/// Special implementation of BytesStartArray that keeps parallel arrays for boost and docFreq
struct TermFreqBoostByteStart {
  boost: Vec<f32>,
  term_state: Vec<TermStates>,
  base: DirectBytesStartArray,
}
impl TermFreqBoostByteStart {
  fn new(init_size: usize) -> Self {
    let base = DirectBytesStartArray::new(init_size);
    Self {
      boost: Vec::new(),
      term_state: Vec::new(),
      base,
    }
  }
}
impl BytesStartArray for TermFreqBoostByteStart {
  fn init(&mut self) {
    self.base.init();
    let len = self.base.bytes_start.as_slice().len();
    self.boost = vec![0.0; len];
    self.term_state = vec![std::default::Default::default(); len];

    debug_assert!(self.term_state.len() >= len);
    debug_assert!(self.boost.len() >= len);
  }

  fn grow(&mut self) -> Result<()> {
    self.base.grow()?;
    let ord = self.base.bytes_start.as_slice();
    let len = ord.len();
    ArrayUtil::grow_with_len(&mut self.boost, len);
    if self.term_state.len() < len {
      self
        .term_state
        .resize(len, std::default::Default::default());
    }
    Ok(())
  }

  fn clear(&mut self) {
    self.boost.clear();
    self.term_state.clear();
    self.base.clear();
  }

  fn bytes_used(&mut self) -> SharedCounter {
    self.base.bytes_used()
  }

  fn get_value(&self, index: usize) -> i32 {
    self.base.get_value(index)
  }

  fn set_value(&mut self, index: usize, value: i32) {
    self.base.set_value(index, value);
  }

  fn len(&self) -> usize {
    self.base.len()
  }

  fn need_init(&self) -> bool {
    self.base.need_init()
  }
}
