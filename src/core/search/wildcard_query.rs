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
use crate::core::index::terms::Terms;
use crate::core::search::automaton_query::AutomatonQuery;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::multi_term_query::{
  ConstantScoreBlendedRewrite, MultiTermQuery, RewriteMethod, RewriteMethodEnum,
};
use crate::core::search::query::{Query, QueryBase, QueryWeight};
use crate::core::search::query_visitor::QueryVisitor;
use crate::core::search::score_mode::ScoreMode;
use crate::core::util::HasIdentity;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
/// Implements the wildcard search query. Supported wildcards are `*`, which matches any
/// character sequence (including the empty one), and `?`, which matches any single
/// character. '\' is the escape character.
///
/// Note this query can be slow, as it needs to iterate over many terms. In order to prevent
/// extremely slow WildcardQueries, a Wildcard term should not start with the wildcard `*`
///
/// This query uses the [`ConstantScoreBlendedRewrite`] rewrite method.
///
/// See [`AutomatonQuery`].
#[derive(Clone)]
pub struct WildcardQuery {
  determinize_work_limit: i32,
  base: AutomatonQuery,
  id: Identity,
}

impl WildcardQuery {
  /// String equality with support for wildcards
  pub const WILDCARD_STRING: char = '*';

  /// Char equality with support for wildcards
  pub const WILDCARD_CHAR: char = '?';

  /// Escape character
  pub const WILDCARD_ESCAPE: char = '\\';

  /// Constructs a query for terms matching `term`.
  pub fn new(term: Term) -> Result<Self> {
    Self::with_determinize_work_limit(term, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32)
  }

  /// Constructs a query for terms matching `term`.
  pub fn with_determinize_work_limit(term: Term, determinize_work_limit: i32) -> Result<Self> {
    Self::with_rewrite(term, determinize_work_limit, ConstantScoreBlendedRewrite)
  }

  /// Constructs a query for terms matching `term`.
  pub fn with_rewrite<R>(term: Term, determinize_work_limit: i32, rewrite_method: R) -> Result<Self>
  where
    R: Into<RewriteMethodEnum>,
  {
    let automaton = to_automaton(&term, determinize_work_limit)?;
    let base = AutomatonQuery::new(term, automaton, false, rewrite_method)?;
    Ok(Self {
      determinize_work_limit,
      base,
      id: Identity::default(),
    })
  }
}

impl QueryBase for WildcardQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();

    if self.base.get_field() != field {
      buffer.push_str(self.base.get_field());
      buffer.push(':');
    }

    buffer.push_str(&self.base.term.text()?);
    Ok(buffer)
  }

  fn create_weight<IRC>(
    self,
    _searcher: &IndexSearcher<IRC>,
    _score_mode: &ScoreMode,
    _boost: f32,
  ) -> Result<QueryWeight<IRC>>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn rewrite<IRC>(self, searcher: &IndexSearcher<IRC>) -> Result<Query>
  where
    IRC: IndexReaderContext,
    Self: Sized,
  {
    let rewrite_method = self.base.rewrite_method.clone();
    rewrite_method.rewrite(searcher, self.into())
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

impl Debug for WildcardQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.as_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for WildcardQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for WildcardQuery {
  fn get_field(&self) -> &str {
    self.base.get_field()
  }

  type TermsEnum<T>
    = <AutomatonQuery as MultiTermQuery>::TermsEnum<T>
  where
    T: Terms;

  fn get_terms_enum<T>(&self, terms: T) -> Result<Self::TermsEnum<T>>
  where
    T: Terms + Clone,
  {
    self.base.compiled.get_terms_enum(terms)
  }

  fn as_query(&self) -> Query {
    self.clone().into()
  }
}

impl Hash for WildcardQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.base.hash(state);
  }
}

impl Eq for WildcardQuery {}

impl PartialEq for WildcardQuery {
  fn eq(&self, other: &Self) -> bool {
    self.base == other.base
  }
}

pub fn to_automaton(wildcard_query: &Term, determinize_work_limit: i32) -> Result<Automaton> {
  let mut automata = Vec::new();
  let wildcard_text = wildcard_query.text()?;
  let chars: Vec<char> = wildcard_text.chars().collect();

  let mut i: usize = 0;
  while i < chars.len() {
    let c = chars[i];

    match c {
      WildcardQuery::WILDCARD_STRING => {
        automata.push(Automata::make_any_string()?);
      },
      WildcardQuery::WILDCARD_CHAR => {
        automata.push(Automata::make_any_char()?);
      },
      WildcardQuery::WILDCARD_ESCAPE if i + 1 < chars.len() => {
        let next = chars[i + 1] as i32;
        automata.push(Automata::make_char(next)?);
        i += 1;
      },
      _ => {
        let cp = c as i32;
        automata.push(Automata::make_char(cp)?);
      },
    }

    i += 1;
  }
  let automata = automata.iter().collect::<Vec<_>>();
  let a = Operations::concatenate_with_list(automata.as_ref())?;
  let v = Operations::determinize(&a, determinize_work_limit as usize)?;
  match v {
    Cow::Borrowed(_) => Ok(a),
    Cow::Owned(v) => Ok(v),
  }
}
#[cfg(test)]
mod tests {
  use crate::analysis::common::analysis_impl::core::whitespace_analyzer::WhitespaceAnalyzer;
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::index::directory_reader::directory_reader_util;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::multi_terms::get_terms;
  use crate::core::index::term::Term;
  use crate::core::search::boolean_clause::Occur;
  use crate::core::search::boolean_query::Builder;
  use crate::core::search::index_searcher::IndexSearcher;
  use crate::core::search::multi_term_query::{
    ConstantScoreBlendedRewrite, ConstantScoreRewrite, MultiTermQuery,
  };
  use crate::core::search::prefix_query::PrefixQuery;
  use crate::core::search::query::{Query, QueryBase};
  use crate::core::search::score_mode::ScoreMode;
  use crate::core::search::term_query::TermQuery;
  use crate::core::search::wildcard_query::WildcardQuery;
  use crate::core::store::directory::DirEnum;
  use crate::core::util::automation::compiled_automaton::CompiledAutomatonTE;
  use crate::core::util::automation::operations::Operations;
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::index::random_index_writer::RandomIndexWriter;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_searcher_with_reader, new_string_field, new_text_field, random,
  };
  use rand::Rng;
  use std::collections::HashMap;
  use std::rc::Rc;
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestWildcardQuery;

  #[test]
  fn test_equals() -> Result<()> {
    let wq1 = WildcardQuery::new(Term::from_text("field", "b*a"))?;
    let wq2 = WildcardQuery::new(Term::from_text("field", "b*a"))?;
    let wq3 = WildcardQuery::new(Term::from_text("field", "b*a"))?;

    // reflexive?
    assert_eq!(wq1, wq2);
    assert_eq!(wq2, wq1);

    // transitive?
    assert_eq!(wq2, wq3);
    assert_eq!(wq1, wq3);

    // TODO IMPORTANT FuzzyQuery未实现
    // let fq = FuzzyQuery::new(Term::from_text("field", "b*a"))?;
    // assert_ne!(wq1.clone().into(), fq.clone().into());
    // assert_ne!(fq.into(), wq1.into());

    Ok(())
  }
  #[test]
  fn test_term_without_wildcard() -> Result<()> {
    let mut random = random();

    let index_store = get_index_store(&mut random, "field", &["nowildcard", "nowildcardx"])?;
    let reader = directory_reader_util::open(index_store)?;
    let searcher = new_searcher_with_reader(reader)?;

    let wq = WildcardQuery::new(Term::from_text("field", "nowildcard"))?;
    assert_matches(&searcher, wq, 1)?;

    // TODO IMPORTANT ScoringBooleanRewrite未实现
    // let q = searcher.rewrite(
    //     WildcardQuery::with_rewrite(
    //         Term::from_text("field", "nowildcard"),
    //         Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    //         ScoringBooleanRewrite,
    //     )?
    // )?;
    // assert!(matches!(q, Query::Term(_)));

    let q = searcher.rewrite(WildcardQuery::with_rewrite(
      Term::from_text("field", "nowildcard"),
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      ConstantScoreRewrite,
    )?)?;
    assert!(matches!(q, Query::MultiTermQueryConstantScoreWrapper(_)));

    let q = searcher.rewrite(WildcardQuery::with_rewrite(
      Term::from_text("field", "nowildcard"),
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
      ConstantScoreBlendedRewrite,
    )?)?;
    assert!(matches!(
      q,
      Query::MultiTermQueryConstantScoreBlendedWrapper(_)
    ));

    // TODO IMPORTANT ScoringBooleanRewrite未实现
    // let q = searcher.rewrite(
    //     WildcardQuery::with_rewrite(
    //         Term::from_text("field", "nowildcard"),
    //         Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    //         ConstantScoreBooleanRewrite,
    //     )?
    // )?;
    // assert!(matches!(q, Query::ConstantScore(_)));

    Ok(())
  }
  #[test]
  fn test_empty_term() -> Result<()> {
    // TODO IMPORTANT ScoringBooleanRewrite未实现
    // let mut random = random();
    //
    // let index_store = get_index_store(&mut random, "field", &["nowildcard", "nowildcardx"])?;
    // let reader = directory_reader_util::open(index_store.clone())?;
    // let searcher = new_searcher_with_reader(reader)?;
    //
    // let wq = WildcardQuery::with_rewrite(
    //     Term::from_text("field", ""),
    //     Operations::DEFAULT_DETERMINIZE_WORK_LIMIT as i32,
    //     ScoringBooleanRewrite,
    // )?;
    // assert_matches(&searcher, wq.clone().into(), 0)?;
    //
    // let q = searcher.rewrite(wq.into())?;
    // assert!(matches!(q, Query::MatchNoDoc(_)));

    Ok(())
  }
  #[test]
  fn test_prefix_term() -> Result<()> {
    let mut random = random();

    let index_store = get_index_store(&mut random, "field", &["prefix", "prefixx"])?;
    let reader = directory_reader_util::open(index_store.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    let mut wq: Query = WildcardQuery::new(Term::from_text("field", "prefix*"))?.into();
    assert_matches(&searcher, wq.clone(), 2)?;

    wq = WildcardQuery::new(Term::from_text("field", "*"))?.into();
    assert_matches(&searcher, wq.clone(), 2)?;

    let terms = get_terms(searcher.get_index_reader(), "field")?.unwrap();
    let te = match wq {
      Query::Wildcard(q) => q.get_terms_enum(Rc::new(terms))?,
      _ => return Err(LuceneError::illegal_state("expected WildcardQuery")),
    };
    assert!(matches!(te, CompiledAutomatonTE::TE(_)));

    Ok(())
  }
  #[test]
  fn test_asterisk() -> Result<()> {
    let mut random = random();

    let index_store = get_index_store(&mut random, "body", &["metal", "metals"])?;
    let reader = directory_reader_util::open(index_store.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    let query1 = TermQuery::new(Term::from_text("body", "metal"));
    let query2 = WildcardQuery::new(Term::from_text("body", "metal*"))?;
    let query3 = WildcardQuery::new(Term::from_text("body", "m*tal"))?;
    let query4 = WildcardQuery::new(Term::from_text("body", "m*tal*"))?;
    let query5 = WildcardQuery::new(Term::from_text("body", "m*tals"))?;

    let mut builder6 = Builder::new();
    builder6.add(query5.clone(), Occur::Should)?;
    let query6: Query = builder6.build().into();

    let mut builder7 = Builder::new();
    builder7.add(query3.clone(), Occur::Should)?;
    builder7.add(query5.clone(), Occur::Should)?;
    let query7: Query = builder7.build().into();

    let query8: Query = WildcardQuery::new(Term::from_text("body", "M*tal*"))?.into();

    assert_matches(&searcher, query1, 1)?;
    assert_matches(&searcher, query2, 2)?;
    assert_matches(&searcher, query3, 1)?;
    assert_matches(&searcher, query4, 2)?;
    assert_matches(&searcher, query5, 1)?;
    assert_matches(&searcher, query6, 1)?;
    assert_matches(&searcher, query7, 2)?;
    assert_matches(&searcher, query8, 0)?;
    assert_matches(
      &searcher,
      WildcardQuery::new(Term::from_text("body", "*tall"))?,
      0,
    )?;
    assert_matches(
      &searcher,
      WildcardQuery::new(Term::from_text("body", "*tal"))?,
      1,
    )?;
    assert_matches(
      &searcher,
      WildcardQuery::new(Term::from_text("body", "*tal*"))?,
      2,
    )?;

    Ok(())
  }
  #[test]
  fn test_questionmark() -> Result<()> {
    let mut random = random();

    let index_store = get_index_store(
      &mut random,
      "body",
      &["metal", "metals", "mXtals", "mXtXls"],
    )?;
    let reader = directory_reader_util::open(index_store.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    let query1 = WildcardQuery::new(Term::from_text("body", "m?tal"))?;
    let query2 = WildcardQuery::new(Term::from_text("body", "metal?"))?;
    let query3 = WildcardQuery::new(Term::from_text("body", "metals?"))?;
    let query4 = WildcardQuery::new(Term::from_text("body", "m?t?ls"))?;
    let query5 = WildcardQuery::new(Term::from_text("body", "M?t?ls"))?;
    let query6 = WildcardQuery::new(Term::from_text("body", "meta??"))?;

    assert_matches(&searcher, query1, 1)?;
    assert_matches(&searcher, query2, 1)?;
    assert_matches(&searcher, query3, 0)?;
    assert_matches(&searcher, query4, 3)?;
    assert_matches(&searcher, query5, 0)?;
    assert_matches(&searcher, query6, 1)?;

    Ok(())
  }
  #[test]
  fn test_escapes() -> Result<()> {
    let mut random = random();

    let index_store = get_index_store(
      &mut random,
      "field",
      &[
        "foo*bar",
        "foo??bar",
        "fooCDbar",
        "fooSOMETHINGbar",
        "foo\\",
      ],
    )?;
    let reader = directory_reader_util::open(index_store.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    // without escape: matches foo??bar, fooCDbar, foo*bar, and fooSOMETHINGbar
    let unescaped = WildcardQuery::new(Term::from_text("field", "foo*bar"))?;
    assert_matches(&searcher, unescaped, 4)?;

    // with escape: only matches foo*bar
    let escaped = WildcardQuery::new(Term::from_text("field", "foo\\*bar"))?;
    assert_matches(&searcher, escaped, 1)?;

    // without escape: matches foo??bar and fooCDbar
    let unescaped = WildcardQuery::new(Term::from_text("field", "foo??bar"))?;
    assert_matches(&searcher, unescaped, 2)?;

    // with escape: matches foo??bar only
    let escaped = WildcardQuery::new(Term::from_text("field", "foo\\?\\?bar"))?;
    assert_matches(&searcher, escaped, 1)?;

    // check escaping at end: lenient parse yields "foo\"
    let at_end = WildcardQuery::new(Term::from_text("field", "foo\\"))?;
    assert_matches(&searcher, at_end, 1)?;

    Ok(())
  }
  fn get_index_store<R>(random: &mut R, field: &str, contents: &[&str]) -> Result<Arc<DirEnum>>
  where
    R: Rng + ?Sized,
  {
    let dir = new_directory_shared(random)?;
    // TODO IMPORTANT MockTokenizer.WHITESPACE未实现 暂时使用WhitespaceAnalyzer
    let writer = RandomIndexWriter::with_analyzer(random, dir.clone(), WhitespaceAnalyzer::new());
    let mut field_to_type = HashMap::new();

    for content in contents {
      let mut doc = Document::new();
      doc.add(new_text_field(
        random,
        field,
        *content,
        Store::Yes,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;
    }

    writer.close()?;
    Ok(dir)
  }
  fn assert_matches<IRC, Q>(
    searcher: &IndexSearcher<IRC>,
    q: Q,
    expected_matches: usize,
  ) -> Result<()>
  where
    IRC: IndexReaderContext,
    Q: Into<Query>,
  {
    let result = searcher.search(q, 1000)?.score_docs;
    assert_eq!(expected_matches, result.len());
    Ok(())
  }
  #[test]
  fn test_parsing_and_searching() -> Result<()> {
    let mut random = random();
    let field = "content";
    let docs = ["\\ abcdefg1", "\\79 hijklmn1", "\\\\ opqrstu1"];

    // queries that should find all docs
    let match_all = vec![
      WildcardQuery::new(Term::from_text(field, "*"))?,
      WildcardQuery::new(Term::from_text(field, "*1"))?,
      WildcardQuery::new(Term::from_text(field, "**1"))?,
      WildcardQuery::new(Term::from_text(field, "*?"))?,
      WildcardQuery::new(Term::from_text(field, "*?1"))?,
      WildcardQuery::new(Term::from_text(field, "?*1"))?,
      WildcardQuery::new(Term::from_text(field, "**"))?,
      WildcardQuery::new(Term::from_text(field, "***"))?,
      WildcardQuery::new(Term::from_text(field, "\\\\*"))?,
    ];

    // queries that should find no docs
    let match_none = vec![
      WildcardQuery::new(Term::from_text(field, "a*h"))?,
      WildcardQuery::new(Term::from_text(field, "a?h"))?,
      WildcardQuery::new(Term::from_text(field, "*a*h"))?,
      WildcardQuery::new(Term::from_text(field, "?a"))?,
      WildcardQuery::new(Term::from_text(field, "a?"))?,
    ];

    let match_one_doc_prefix = [
      vec![
        PrefixQuery::new(Term::from_text(field, "a"))?,
        PrefixQuery::new(Term::from_text(field, "ab"))?,
        PrefixQuery::new(Term::from_text(field, "abc"))?,
      ], // these should find only doc 0
      vec![
        PrefixQuery::new(Term::from_text(field, "h"))?,
        PrefixQuery::new(Term::from_text(field, "hi"))?,
        PrefixQuery::new(Term::from_text(field, "hij"))?,
        PrefixQuery::new(Term::from_text(field, "\\7"))?,
      ], // these should find only doc 1
      vec![
        PrefixQuery::new(Term::from_text(field, "o"))?,
        PrefixQuery::new(Term::from_text(field, "op"))?,
        PrefixQuery::new(Term::from_text(field, "opq"))?,
        PrefixQuery::new(Term::from_text(field, "\\\\"))?,
      ], // these should find only doc 2
    ];

    let match_one_doc_wild = [
      vec![
        WildcardQuery::new(Term::from_text(field, "*a*"))?,
        WildcardQuery::new(Term::from_text(field, "*ab*"))?,
        WildcardQuery::new(Term::from_text(field, "*abc**"))?,
        WildcardQuery::new(Term::from_text(field, "ab*e*"))?,
        WildcardQuery::new(Term::from_text(field, "*g?"))?,
        WildcardQuery::new(Term::from_text(field, "*f?1"))?,
      ],
      vec![
        WildcardQuery::new(Term::from_text(field, "*h*"))?,
        WildcardQuery::new(Term::from_text(field, "*hi*"))?,
        WildcardQuery::new(Term::from_text(field, "*hij**"))?,
        WildcardQuery::new(Term::from_text(field, "hi*k*"))?,
        WildcardQuery::new(Term::from_text(field, "*n?"))?,
        WildcardQuery::new(Term::from_text(field, "*m?1"))?,
        WildcardQuery::new(Term::from_text(field, "hij**"))?,
      ],
      vec![
        WildcardQuery::new(Term::from_text(field, "*o*"))?,
        WildcardQuery::new(Term::from_text(field, "*op*"))?,
        WildcardQuery::new(Term::from_text(field, "*opq**"))?,
        WildcardQuery::new(Term::from_text(field, "op*q*"))?,
        WildcardQuery::new(Term::from_text(field, "*u?"))?,
        WildcardQuery::new(Term::from_text(field, "*t?1"))?,
        WildcardQuery::new(Term::from_text(field, "opq**"))?,
      ],
    ];

    // prepare the index
    let dir = new_directory_shared(&mut random)?;
    // TODO IMPORTANT MockTokenizer.WHITESPACE未实现 暂时使用WhitespaceAnalyzer
    let iw = RandomIndexWriter::with_analyzer(&mut random, dir.clone(), WhitespaceAnalyzer::new());
    let mut field_to_type = HashMap::new();

    for d in docs {
      let mut doc = Document::new();
      doc.add(new_text_field(
        &mut random,
        field,
        d,
        Store::No,
        &mut field_to_type,
      )?);
      iw.add_document(doc)?;
    }
    iw.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    let searcher = new_searcher_with_reader(Arc::new(reader))?;

    // test queries that must find all
    for q in match_all {
      let hits = searcher.search(q, 1000)?.score_docs;
      assert_eq!(docs.len(), hits.len());
    }

    // test queries that must find none
    for q in match_none {
      let hits = searcher.search(q, 1000)?.score_docs;
      assert_eq!(0, hits.len());
    }

    // test the prefix queries find only one doc
    for (i, qs) in match_one_doc_prefix.iter().enumerate() {
      for q in qs {
        let hits = searcher.search(q.clone(), 1000)?.score_docs;
        assert_eq!(1, hits.len());
        assert_eq!(i as i32, hits[0].doc);
      }
    }

    // test the wildcard queries find only one doc
    for (i, qs) in match_one_doc_wild.iter().enumerate() {
      for q in qs {
        let hits = searcher.search(q.clone(), 1000)?.score_docs;
        assert_eq!(1, hits.len());
        assert_eq!(i as i32, hits[0].doc);
      }
    }

    Ok(())
  }
  #[test]
  fn test_large() -> Result<()> {
    let mut random = random();

    // big string from a user
    let big = "{group-bm-http-server-02083.node.dm.reg,group-bm-http-server-02082.node.dm.reg,group-bm-http-server-02081.node.dm.reg,group-bm-http-server-02080.node.dm.reg,group-bm-http-server-02079.node.dm.reg,group-bm-http-server-02078.node.dm.reg,group-bm-http-server-02077.node.dm.reg,group-bm-http-server-02076.node.dm.reg,group-bm-http-server-02073.node.dm.reg,group-bm-http-server-02070.node.dm.reg,group-bm-http-server-02067.node.dm.reg,group-bm-http-server-02064.node.dm.reg,group-bm-http-server-02029.node.dm.reg,group-bm-http-server-02028.node.dm.reg,group-bm-http-server-02027.node.dm.reg,group-bm-http-server-02026.node.dm.reg,group-bm-http-server-02025.node.dm.reg,group-bm-http-server-02023.node.dm.reg,group-bm-http-server-02022.node.dm.reg,group-bm-http-server-02021.node.dm.reg,group-bm-http-server-02020.node.dm.reg,group-bm-http-server-02019.node.dm.reg,group-bm-http-server-02018.node.dm.reg,group-bm-http-server-02016.node.dm.reg,group-bm-http-server-02015.node.dm.reg,group-bm-http-server-02014.node.dm.reg,group-bm-http-server-02009.node.dm.reg,group-bm-http-server-02007.node.dm.reg,group-bm-http-server-02004.node.dm.reg,group-bm-http-server-02003.node.dm.reg,group-bm-http-server-02002.node.dm.reg,group-bm-http-server-01311.node.dm.reg,group-bm-http-server-01309.node.dm.reg,group-bm-http-server-01307.node.dm.reg}";

    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();

    let mut doc = Document::new();
    doc.add(new_string_field(
      &mut random,
      "body",
      big,
      Store::Yes,
      &mut field_to_type,
    )?);
    writer.add_document(doc)?;
    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    let query: Query = WildcardQuery::new(Term::from_text("body", format!("{}*", big)))?.into();
    assert_matches(&searcher, query, 1)?;

    Ok(())
  }
  #[test]
  fn test_cost_estimate() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let writer = RandomIndexWriter::new(&mut random, dir.clone());
    let mut field_to_type = HashMap::new();

    for i in 0..1000 {
      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "body",
        "foo bar",
        Store::No,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "body",
        "foo wuzzle",
        Store::No,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;

      let mut doc = Document::new();
      doc.add(new_string_field(
        &mut random,
        "body",
        format!("bar {}", i),
        Store::No,
        &mut field_to_type,
      )?);
      writer.add_document(doc)?;
    }

    writer.flush()?;
    writer.force_merge(1)?;
    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(searcher.get_leaf_contexts()?.len(), 1);
    let lrc = &searcher.get_leaf_contexts()?[0];

    let query: Query = WildcardQuery::new(Term::from_text("body", "foo*"))?.into();
    let rewritten = searcher.rewrite(query)?;
    let weight = rewritten.create_weight(&searcher, &ScoreMode::CompleteNoScores, 1.0)?;
    let mut supplier = weight.scorer_supplier(lrc, &searcher)?.unwrap();
    assert_eq!(2000, supplier.cost(lrc, &searcher)? as i64);

    let query: Query = WildcardQuery::new(Term::from_text("body", "bar*"))?.into();
    let rewritten = searcher.rewrite(query)?;
    let weight = rewritten.create_weight(&searcher, &ScoreMode::CompleteNoScores, 1.0)?;
    let mut supplier = weight.scorer_supplier(lrc, &searcher)?.unwrap();
    assert_eq!(3000, supplier.cost(lrc, &searcher)? as i64);

    Ok(())
  }
}
