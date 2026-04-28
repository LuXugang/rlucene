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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::leaf_reader_context::LeafReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_states::TermStates;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::multi_term_query::{MultiTermQuery, RewriteMethod};
use crate::core::search::query::Query;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;

pub trait TermCollectingRewrite: RewriteMethod {
  type B;
  /// Return a suitable builder for the top-level [`Query`] for holding all expanded terms.
  fn get_top_level_builder(&self) -> Result<Self::B>;

  /// Finalize the creation of the query from the builder.
  fn build(&self, builder: Self::B) -> Result<Query>;

  /// Add a [`MultiTermQuery`] term to the top-level query builder.
  fn add_clause(
    &self,
    top_level: &mut Self::B,
    term: Term,
    doc_count: i32,
    boost: f32,
  ) -> Result<()> {
    self.add_clause_with_states(top_level, term, doc_count, boost, None)
  }

  fn add_clause_with_states(
    &self,
    top_level: &mut Self::B,
    term: Term,
    doc_count: i32,
    boost: f32,
    states: Option<TermStates>,
  ) -> Result<()>;
  fn collect_terms<IRC, Q, C>(
    &self,
    top_reader_context: &IRC,
    query: &Q,
    collector: &mut C,
  ) -> Result<()>
  where
    IRC: IndexReaderContext,
    Q: MultiTermQuery,
    C: TermCollector,
  {
    for context in top_reader_context.leaves()? {
      let terms = context.reader().terms(query.get_field())?;
      let Some(terms) = terms else {
        continue;
      };

      let mut terms_enum = self.get_terms_enum(query, Rc::new(terms))?;

      // TODO IMPORTANT 这里要判断是否为 EMPTY
      collector.set_reader_context::<IRC>(context)?;
      collector.set_next_enum(&mut terms_enum)?;
      collect_terms(collector, &mut terms_enum, top_reader_context)?;
    }

    Ok(())
  }
}
pub trait TermCollector {
  fn set_reader_context<IRC>(
    &mut self,
    context: &LeafReaderContext<IRCLeafReader<IRC>>,
  ) -> Result<()>
  where
    IRC: IndexReaderContext;

  /// Return false to stop collecting.
  fn collect<TE, IRC>(
    &mut self,
    bytes: BytesRef<Vec<u8>>,
    terms_enum: &mut TE,
    top_reader_context: &IRC,
  ) -> Result<bool>
  where
    TE: TermsEnum,
    IRC: IndexReaderContext;

  /// The next segment's [`TermsEnum`] that is used to collect terms.
  fn set_next_enum<TE>(&mut self, terms_enum: &mut TE) -> Result<()>
  where
    TE: TermsEnum;
}
fn collect_terms<C, TE, IRC>(
  collector: &mut C,
  terms_enum: &mut TE,
  top_reader_context: &IRC,
) -> Result<()>
where
  C: TermCollector,
  TE: TermsEnum,
  IRC: IndexReaderContext,
{
  while let Some(bytes) = terms_enum.next()? {
    let bytes = bytes.into_owned();

    if !collector.collect(bytes, terms_enum, top_reader_context)? {
      return Ok(());
    }
  }

  Ok(())
}
