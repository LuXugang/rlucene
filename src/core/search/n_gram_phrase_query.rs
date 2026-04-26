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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::phrase_query::{Builder as PhraseQueryBuilder, PhraseQuery};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::error::lucene_error::Result;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// This is a [`PhraseQuery`] which is optimized for n-gram phrase query. For example, when you
/// query "ABCD" on a 2-gram field, you may want to use NGramPhraseQuery rather than
/// [`PhraseQuery`], because NGramPhraseQuery will [`Query::rewrite`] the query to
/// "AB/0 CD/2", while [`PhraseQuery`] will query "AB/0 BC/1 CD/2" (where term/position).
#[derive(Debug, Clone)]
pub struct NGramPhraseQuery {
  id: Identity,
  n: usize,
  phrase_query: PhraseQuery,
}

impl NGramPhraseQuery {
  /// Creates a query with the given n-gram size.
  pub fn new(n: usize, phrase_query: PhraseQuery) -> Self {
    Self {
      id: Identity::new(),
      n,
      phrase_query,
    }
  }

  /// Return the n in n-gram.
  pub fn get_n(&self) -> usize {
    self.n
  }

  /// Return the list of terms.
  pub fn get_terms(&self) -> &[Term] {
    self.phrase_query.get_terms()
  }

  /// Return the list of relative positions that each term should appear at.
  pub fn get_positions(&self) -> &[usize] {
    self.phrase_query.get_positions()
  }
}

impl Eq for NGramPhraseQuery {}

impl PartialEq for NGramPhraseQuery {
  fn eq(&self, other: &Self) -> bool {
    self.n == other.n && self.phrase_query == other.phrase_query
  }
}

impl Hash for NGramPhraseQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    std::any::type_name::<Self>().hash(state);
    self.phrase_query.hash(state);
    self.n.hash(state);
  }
}

impl HasIdentity for NGramPhraseQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl QueryBase for NGramPhraseQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    self.phrase_query.as_string(field)
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
    self.phrase_query.create_weight(searcher, score_mode, boost)
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let terms = self.phrase_query.get_terms();
    let positions = self.phrase_query.get_positions();

    let is_optimizable = self.phrase_query.get_slop() == 0
      && self.n >= 2
      && terms.len() >= 3
      && positions
        .windows(2)
        .all(|window| window[1] == window[0] + 1);

    if !is_optimizable {
      return self.phrase_query.rewrite(searcher);
    }
    let n = self.n;
    let terms = self.phrase_query.get_term_arc();
    drop(self);
    let terms = Arc::try_unwrap(terms).unwrap_or_else(|terms| terms.as_ref().clone());

    let terms_len = terms.len();
    let mut builder = PhraseQueryBuilder::new();

    for (i, term) in terms.into_iter().enumerate() {
      if i % n == 0 || i == terms_len - 1 {
        builder.add(term, i)?;
      }
    }
    Ok(builder.build()?.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::DefaultIndexSearchCR;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_searcher_with_reader, random,
  };
  use rand::Rng;
  #[allow(dead_code)] // for quick search
  struct TestNGramPhraseQuery;
  fn set_up<R>(random: &mut R) -> Result<DefaultIndexSearchCR>
  where
    R: Rng + ?Sized,
  {
    let directory = new_directory_shared(random)?;
    let writer = RandomIndexWriter::new(random, directory.clone());
    writer.close()?;

    let reader = directory_reader_util::open(directory.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    Ok(searcher)
  }
  #[test]
  fn test_rewrite() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    // bi-gram test ABC => AB/BC => AB/BC
    let pq1 = NGramPhraseQuery::new(2, PhraseQuery::from_terms_no_slop("f", &["AB", "BC"])?);

    let q = pq1.rewrite(&searcher)?;
    assert_eq!(q.clone().rewrite(&searcher)?, q);
    let Query::Phrase(rewritten1) = q else {
      panic!("expected PhraseQuery");
    };
    assert_eq!(
      &vec![Term::from_text("f", "AB"), Term::from_text("f", "BC")],
      rewritten1.get_terms()
    );
    assert_eq!(&vec![0, 1], rewritten1.get_positions());

    // bi-gram test ABCD => AB/BC/CD => AB//CD
    let pq2 = NGramPhraseQuery::new(
      2,
      PhraseQuery::from_terms_no_slop("f", &["AB", "BC", "CD"])?,
    );

    let q = pq2.rewrite(&searcher)?;
    assert!(matches!(q, Query::Phrase(_)));
    let Query::Phrase(rewritten2) = q else {
      panic!("expected PhraseQuery");
    };
    assert_eq!(
      &vec![Term::from_text("f", "AB"), Term::from_text("f", "CD")],
      rewritten2.get_terms()
    );
    assert_eq!(&vec![0, 2], rewritten2.get_positions());

    // tri-gram test ABCDEFGH => ABC/BCD/CDE/DEF/EFG/FGH => ABC///DEF//FGH
    let pq3 = NGramPhraseQuery::new(
      3,
      PhraseQuery::from_terms_no_slop("f", &["ABC", "BCD", "CDE", "DEF", "EFG", "FGH"])?,
    );

    let q = pq3.rewrite(&searcher)?;
    assert!(matches!(q, Query::Phrase(_)));
    let Query::Phrase(rewritten3) = q else {
      panic!("expected PhraseQuery");
    };
    assert_eq!(
      &vec![
        Term::from_text("f", "ABC"),
        Term::from_text("f", "DEF"),
        Term::from_text("f", "FGH"),
      ],
      rewritten3.get_terms()
    );
    assert_eq!(&vec![0, 3, 5], rewritten3.get_positions());

    Ok(())
  }
}
