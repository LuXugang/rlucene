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
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
/// A `Query` that matches documents whose terms fall within a specified range.
///
/// This query matches documents containing terms that fall within the given
/// range according to `BytesRef::compare`.
///
/// **NOTE**: `TermRangeQuery` is significantly slower than point-based ranges
/// see [`PointRangeQuery`](crate::core::search::point_range_query::PointRangeQuery) because it must visit all terms that match the range
/// and merge their matches.
///
/// This query uses the [`ConstantScoreBlendedRewrite`]
/// rewrite method.
#[derive(Clone)]
pub struct TermRangeQuery {
  lower_term: Option<BytesRef<Vec<u8>>>,
  upper_term: Option<BytesRef<Vec<u8>>>,
  include_lower: bool,
  include_upper: bool,
  base: AutomatonQuery,
  id: Identity,
}
impl TermRangeQuery {
  /// Constructs a query selecting all terms greater than or equal to `lower_term`
  /// but less than or equal to `upper_term`.
  ///
  /// If an endpoint is `None`, it is considered "open". Either or both endpoints
  /// may be open. Open endpoints may not be exclusive (it is not possible to
  /// select all but the first or last term without explicitly specifying the
  /// term to exclude).
  ///
  /// # Parameters
  ///
  /// - `field`: The field that holds both lower and upper terms.
  /// - `lower_term`: The term text at the lower end of the range.
  /// - `upper_term`: The term text at the upper end of the range.
  /// - `include_lower`: If `true`, `lower_term` is included in the range.
  /// - `include_upper`: If `true`, `upper_term` is included in the range.
  ///
  /// Uses `CONSTANT_SCORE_BLENDED_REWRITE` as the default rewrite method.
  pub fn new<T>(
    field: T,
    lower_term: Option<BytesRef<Vec<u8>>>,
    upper_term: Option<BytesRef<Vec<u8>>>,
    include_lower: bool,
    include_upper: bool,
  ) -> Result<Self>
  where
    T: Into<String>,
  {
    Self::with_rewrite(
      field,
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      ConstantScoreBlendedRewrite,
    )
  }
  /// Constructs a query selecting all terms greater than or equal to `lower_term`
  /// but less than or equal to `upper_term`.
  ///
  /// If an endpoint is `None`, it is considered "open". Either or both endpoints
  /// may be open. Open endpoints may not be exclusive (it is not possible to
  /// select all but the first or last term without explicitly specifying the
  /// term to exclude).
  ///
  /// # Parameters
  ///
  /// - `field`: The field that holds both lower and upper terms.
  /// - `lower_term`: The term text at the lower end of the range.
  /// - `upper_term`: The term text at the upper end of the range.
  /// - `include_lower`: If `true`, `lower_term` is included in the range.
  /// - `include_upper`: If `true`, `upper_term` is included in the range.
  /// - `rewrite_method`: The rewrite method used when building the final query.
  pub fn with_rewrite<T, R>(
    field: T,
    lower_term: Option<BytesRef<Vec<u8>>>,
    upper_term: Option<BytesRef<Vec<u8>>>,
    include_lower: bool,
    include_upper: bool,
    rewrite_method: R,
  ) -> Result<Self>
  where
    T: Into<String>,
    R: Into<RewriteMethodEnum>,
  {
    let automaton = to_automaton(
      lower_term.as_ref(),
      upper_term.as_ref(),
      include_lower,
      include_upper,
    )?;
    let lower = match lower_term {
      Some(ref lt) => lt.clone(),
      None => BytesRef::default(),
    };
    let base = AutomatonQuery::new(Term::new(field, lower), automaton, true, rewrite_method)?;

    Ok(Self {
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      base,
      id: Identity::default(),
    })
  }
  /// Factory that creates a new `TermRangeQuery` using `String` values
  /// for term text.
  ///
  /// Uses `ConstantScoreBlendedRewrite` as the default rewrite method.
  pub fn new_string_range<F>(
    field: F,
    lower_term: Option<impl AsRef<str>>,
    upper_term: Option<impl AsRef<str>>,
    include_lower: bool,
    include_upper: bool,
  ) -> Result<Self>
  where
    F: Into<String>,
  {
    Self::new_string_range_with_rewrite(
      field,
      lower_term,
      upper_term,
      include_lower,
      include_upper,
      ConstantScoreBlendedRewrite,
    )
  }

  /// Factory that creates a new `TermRangeQuery` using `String` values
  /// for term text.
  pub fn new_string_range_with_rewrite<F, R>(
    field: F,
    lower_term: Option<impl AsRef<str>>,
    upper_term: Option<impl AsRef<str>>,
    include_lower: bool,
    include_upper: bool,
    rewrite_method: R,
  ) -> Result<Self>
  where
    F: Into<String>,
    R: Into<RewriteMethodEnum>,
  {
    let lower = lower_term.map(|s| BytesRef::from_string(s.as_ref()));
    let upper = upper_term.map(|s| BytesRef::from_string(s.as_ref()));

    Self::with_rewrite(
      field,
      lower,
      upper,
      include_lower,
      include_upper,
      rewrite_method,
    )
  }
  /// Returns the lower value of this range query.
  pub fn lower_term(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.lower_term.as_ref()
  }

  /// Returns the upper value of this range query.
  pub fn upper_term(&self) -> Option<&BytesRef<Vec<u8>>> {
    self.upper_term.as_ref()
  }

  /// Returns `true` if the lower endpoint is inclusive.
  pub fn includes_lower(&self) -> bool {
    self.include_lower
  }

  /// Returns `true` if the upper endpoint is inclusive.
  pub fn includes_upper(&self) -> bool {
    self.include_upper
  }
}

impl QueryBase for TermRangeQuery {
  fn as_string(&self, field: &str) -> Result<String> {
    let mut buffer = String::new();

    if self.base.get_field() != field {
      buffer.push_str(self.base.get_field());
      buffer.push(':');
    }

    buffer.push(if self.include_lower { '[' } else { '{' });

    let lower_str = match self.lower_term.as_ref() {
      Some(term) => {
        let s = term.utf8_to_string()?;
        if s == "*" { "\\*".to_string() } else { s }
      },
      None => "*".to_string(),
    };
    buffer.push_str(&lower_str);

    buffer.push_str(" TO ");

    let upper_str = match self.upper_term.as_ref() {
      Some(term) => {
        let s = term.utf8_to_string()?;
        if s == "*" { "\\*".to_string() } else { s }
      },
      None => "*".to_string(),
    };
    buffer.push_str(&upper_str);

    buffer.push(if self.include_upper { ']' } else { '}' });

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
    rewrite_method.rewrite(searcher, self)
  }

  fn visit<QV>(&self, _visitor: &QV)
  where
    QV: QueryVisitor,
  {
    todo!()
  }
}

impl Debug for TermRangeQuery {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.as_string("") {
      Ok(s) => write!(f, "{}", s),
      Err(_) => Err(std::fmt::Error),
    }
  }
}

impl HasIdentity for TermRangeQuery {
  fn identity(&self) -> &Identity {
    &self.id
  }
}

impl MultiTermQuery for TermRangeQuery {
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
impl Hash for TermRangeQuery {
  fn hash<H>(&self, state: &mut H)
  where
    H: std::hash::Hasher,
  {
    self.base.hash(state);
    if self.include_lower {
      1231.hash(state);
    } else {
      1237.hash(state);
    }

    if self.include_upper {
      1231.hash(state);
    } else {
      1237.hash(state);
    }

    self.lower_term.hash(state);
    self.upper_term.hash(state);
  }
}
impl Eq for TermRangeQuery {}
impl PartialEq for TermRangeQuery {
  fn eq(&self, other: &Self) -> bool {
    self.base == other.base
      && self.include_lower == other.include_lower
      && self.include_upper == other.include_upper
      && self.lower_term == other.lower_term
      && self.upper_term == other.upper_term
  }
}
pub fn to_automaton(
  lower_term: Option<&BytesRef<Vec<u8>>>,
  upper_term: Option<&BytesRef<Vec<u8>>>,
  mut include_lower: bool,
  mut include_upper: bool,
) -> Result<Automaton> {
  if lower_term.is_none() {
    include_lower = true;
  }

  if upper_term.is_none() {
    include_upper = true;
  }

  Automata::make_binary_interval(lower_term, include_lower, upper_term, include_upper)
}
#[cfg(test)]
pub(crate) mod tests {
  use crate::core::analysis::analyzer::{
    Analyzer, AnalyzerEnum, AnalyzerStoredValue, BoxedAnalyzer, TokenStreamComponents,
  };
  use crate::core::analysis::reader::Reader;
  use crate::core::analysis::token_stream::{TokenStream, default_attribute};
  use crate::core::analysis::tokenizer::{Tokenizer, TokenizerBase};
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::field_type::FieldType;
  use crate::core::index::directory_reader;
  use crate::core::index::index_reader_context::IndexReaderContext;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::OpenMode;
  use crate::core::search::index_searcher::{
    IndexSearcher, get_max_clause_count, set_max_clause_count,
  };
  use crate::core::search::multi_term_query::TopTermsScoringBooleanQueryRewrite;
  use crate::core::search::query::Query;
  use crate::core::search::term_range_query::TermRangeQuery;
  use crate::core::store::directory::Directory;
  use crate::core::util::attribute_source::{AttributeSource, Attributes};
  use crate::core::util::error::lucene_error::{LuceneError, Result};
  use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test::core::analysis::mock_tokenizer;
  use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader,
    new_string_field, new_text_field, random,
  };
  use rand::Rng;
  use std::collections::{HashMap, HashSet};
  use std::sync::Arc;

  #[allow(dead_code)] // for quick search
  struct TestTermRangeQuery;
  #[test]
  fn test_exclusive() -> Result<()> {
    let mut random = random();
    let query = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), false, false)?;
    let mut doc_count = 0;
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();
    initialize_index(
      &mut random,
      dir.clone(),
      &["A", "B", "C", "D"],
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(1, hits.len(), "A,B,C,D, only B in range");

    initialize_index(&mut random, dir.clone(), &["A", "B", "D"], &mut field_types)?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(1, hits.len(), "A,B,D, only B in range");

    add_doc(
      &mut random,
      dir.clone(),
      "C",
      &mut doc_count,
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(1, hits.len(), "C added, still only B in range");

    Ok(())
  }
  #[test]
  fn test_inclusive() -> Result<()> {
    let mut random = random();
    let query = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), true, true)?;
    let mut doc_count = 0;
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    initialize_index(
      &mut random,
      dir.clone(),
      &["A", "B", "C", "D"],
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(3, hits.len(), "A,B,C,D - A,B,C in range");

    initialize_index(&mut random, dir.clone(), &["A", "B", "D"], &mut field_types)?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(2, hits.len(), "A,B,D - A and B in range");

    add_doc(
      &mut random,
      dir.clone(),
      "C",
      &mut doc_count,
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    let hits = searcher.search(query.clone(), 1000)?.score_docs;
    assert_eq!(3, hits.len(), "C added - A, B, C in range");

    Ok(())
  }
  #[test]
  fn test_all_docs() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    initialize_index(
      &mut random,
      dir.clone(),
      &["A", "B", "C", "D"],
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(Arc::new(reader))?;

    let query = TermRangeQuery::new("content", None, None, true, true)?;
    assert_eq!(4, searcher.search(query.clone(), 1000)?.score_docs.len());

    let query = TermRangeQuery::new_string_range("content", Some(""), None::<String>, true, true)?;
    assert_eq!(4, searcher.search(query.clone(), 1000)?.score_docs.len());

    let query = TermRangeQuery::new_string_range("content", Some(""), None::<String>, true, false)?;
    assert_eq!(4, searcher.search(query.clone(), 1000)?.score_docs.len());

    // and now another one
    let query = TermRangeQuery::new_string_range("content", Some("B"), None::<String>, true, true)?;
    assert_eq!(3, searcher.search(query.clone(), 1000)?.score_docs.len());

    Ok(())
  }
  /// This test should not be here, but it tests the fuzzy query rewrite mode
  /// (TOP_TERMS_SCORING_BOOLEAN_REWRITE) with constant score and checks, that only the lower end of
  /// terms is put into the range
  #[test]
  fn test_top_terms_rewrite() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    initialize_index(
      &mut random,
      dir.clone(),
      &["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K"],
      &mut field_types,
    )?;

    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;

    let rewrite_method = TopTermsScoringBooleanQueryRewrite::new(50);
    let query = TermRangeQuery::new_string_range_with_rewrite(
      "content",
      Some("B"),
      Some("J"),
      true,
      true,
      rewrite_method,
    )?;

    check_boolean_terms(
      &searcher,
      query.clone(),
      &["B", "C", "D", "E", "F", "G", "H", "I", "J"],
    )?;

    let saved_clause_count = get_max_clause_count();
    set_max_clause_count(3)?;
    check_boolean_terms(&searcher, query.clone(), &["B", "C", "D"])?;
    set_max_clause_count(saved_clause_count)?;
    Ok(())
  }

  fn check_boolean_terms<IRC>(
    searcher: &IndexSearcher<IRC>,
    query: TermRangeQuery,
    terms: &[&str],
  ) -> Result<()>
  where
    IRC: IndexReaderContext,
  {
    let rewritten = searcher.rewrite(query)?;

    let bq = match rewritten {
      Query::Boolean(q) => q,
      _ => {
        return Err(LuceneError::illegal_state(
          "expected rewritten query to be a BooleanQuery",
        ));
      },
    };

    let mut allowed_terms: HashSet<String> = terms.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(allowed_terms.len(), bq.clauses().len());

    for c in bq.clauses() {
      let tq = match &c.query {
        Query::Term(q) => q,
        _ => {
          return Err(LuceneError::illegal_state(
            "expected clause query to be a TermQuery",
          ));
        },
      };

      let term = tq.term.text()?.to_string();
      assert!(allowed_terms.contains(&term), "invalid term: {}", term);
      allowed_terms.remove(&term);
    }

    assert_eq!(0, allowed_terms.len());
    Ok(())
  }
  #[test]
  fn test_equals_hashcode() -> Result<()> {
    let mut query = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), true, true)?;

    let mut other = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), true, true)?;

    assert_eq!(query, query, "query equals itself is true");
    assert_eq!(query, other, "equivalent queries are equal");

    {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};

      let mut h1 = DefaultHasher::new();
      query.hash(&mut h1);
      let qh = h1.finish();

      let mut h2 = DefaultHasher::new();
      other.hash(&mut h2);
      let oh = h2.finish();

      assert_eq!(
        qh, oh,
        "hashcode must return same value when equals is true"
      );
    }

    other = TermRangeQuery::new_string_range("notcontent", Some("A"), Some("C"), true, true)?;
    assert_ne!(query, other, "Different fields are not equal");

    other = TermRangeQuery::new_string_range("content", Some("X"), Some("C"), true, true)?;
    assert_ne!(query, other, "Different lower terms are not equal");

    other = TermRangeQuery::new_string_range("content", Some("A"), Some("Z"), true, true)?;
    assert_ne!(query, other, "Different upper terms are not equal");

    query = TermRangeQuery::new_string_range("content", None::<String>, Some("C"), true, true)?;
    other = TermRangeQuery::new_string_range("content", None::<String>, Some("C"), true, true)?;
    assert_eq!(
      query, other,
      "equivalent queries with null lowerterms are equal()"
    );

    {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};

      let mut h1 = DefaultHasher::new();
      query.hash(&mut h1);
      let qh = h1.finish();

      let mut h2 = DefaultHasher::new();
      other.hash(&mut h2);
      let oh = h2.finish();

      assert_eq!(
        qh, oh,
        "hashcode must return same value when equals is true"
      );
    }

    query = TermRangeQuery::new_string_range("content", Some("C"), None::<String>, true, true)?;
    other = TermRangeQuery::new_string_range("content", Some("C"), None::<String>, true, true)?;
    assert_eq!(
      query, other,
      "equivalent queries with null upperterms are equal()"
    );

    {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};

      let mut h1 = DefaultHasher::new();
      query.hash(&mut h1);
      let qh = h1.finish();

      let mut h2 = DefaultHasher::new();
      other.hash(&mut h2);
      let oh = h2.finish();

      assert_eq!(qh, oh, "hashcode returns same value");
    }

    query = TermRangeQuery::new_string_range("content", None::<String>, Some("C"), true, true)?;
    other = TermRangeQuery::new_string_range("content", Some("C"), None::<String>, true, true)?;
    assert_ne!(
      query, other,
      "queries with different upper and lower terms are not equal"
    );

    query = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), false, false)?;
    other = TermRangeQuery::new_string_range("content", Some("A"), Some("C"), true, true)?;
    assert_ne!(
      query, other,
      "queries with different inclusive are not equal"
    );

    Ok(())
  }
  fn initialize_index<D, R>(
    random: &mut R,
    dir: Arc<D>,
    values: &[&str],
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    let a = MockAnalyzer::with_automaton(random, mock_tokenizer::WHITESPACE.clone(), false);
    initialize_index_with_analyzer(random, dir, values, a, field_to_type)
  }

  fn initialize_index_with_analyzer<D, A, R>(
    random: &mut R,
    dir: Arc<D>,
    values: &[&str],
    analyzer: A,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    D: Directory,
    A: Into<AnalyzerEnum>,
  {
    let mut config = new_index_writer_config_with_analyzer(random, analyzer);
    config.set_open_mode(OpenMode::Create);

    let mut writer = IndexWriter::new(dir, config)?;
    let mut doc_count: i32 = 0;

    for v in values {
      insert_doc(random, &mut writer, &mut doc_count, v, field_to_type)?;
    }

    writer.close()?;
    Ok(())
  }

  fn add_doc<D, R>(
    random: &mut R,
    dir: Arc<D>,
    content: &str,
    doc_count: &mut i32,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    R: Rng + ?Sized,
    D: Directory,
  {
    let a = MockAnalyzer::with_automaton(random, mock_tokenizer::WHITESPACE.clone(), false);
    let mut config = new_index_writer_config_with_analyzer(random, a);
    config.set_open_mode(OpenMode::Append);
    let mut writer = IndexWriter::new(dir, config)?;
    insert_doc(random, &mut writer, doc_count, content, field_to_type)?;
    writer.close()?;
    Ok(())
  }

  fn insert_doc<D, R>(
    random: &mut R,
    writer: &mut IndexWriter<D>,
    doc_count: &mut i32,
    content: &str,
    field_to_type: &mut HashMap<String, FieldType>,
  ) -> Result<()>
  where
    D: Directory,
    R: Rng + ?Sized,
  {
    let mut doc = Document::new();

    doc.add(new_string_field(
      random,
      "id",
      format!("id{}", *doc_count),
      Store::Yes,
      field_to_type,
    )?);
    doc.add(new_text_field(
      random,
      "content",
      content,
      Store::No,
      field_to_type,
    )?);

    writer.add_document(doc)?;
    *doc_count += 1;
    Ok(())
  }
  #[test]
  fn test_exclusive_lower_null() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();
    let mut doc_count = 0;
    let query =
      TermRangeQuery::new_string_range("content", None::<String>, Some("C"), false, false)?;

    initialize_index_with_analyzer(
      &mut random,
      dir.clone(),
      &["A", "B", "", "C", "D"],
      SingleCharAnalyzer::new(),
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      3,
      searcher.search(query.clone(), 1000)?.total_hits.value(),
      "A,B,<empty string>,C,D => A, B & <empty string> are in range"
    );

    initialize_index_with_analyzer(
      &mut random,
      dir.clone(),
      &["A", "B", "", "D"],
      SingleCharAnalyzer::new(),
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      3,
      searcher.search(query.clone(), 1000)?.total_hits.value(),
      "A,B,<empty string>,D => A, B & <empty string> are in range"
    );

    add_doc(
      &mut random,
      dir.clone(),
      "C",
      &mut doc_count,
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      3,
      searcher.search(query, 1000)?.total_hits.value(),
      "C added, still A, B & <empty string> are in range"
    );
    Ok(())
  }
  #[test]
  fn test_inclusive_lower_null() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();
    let mut doc_count = 0;
    let query = TermRangeQuery::new_string_range("content", None::<String>, Some("C"), true, true)?;

    initialize_index_with_analyzer(
      &mut random,
      dir.clone(),
      &["A", "B", "", "C", "D"],
      SingleCharAnalyzer::new(),
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      4,
      searcher.search(query.clone(), 1000)?.total_hits.value(),
      "A,B,<empty string>,C,D => A,B,<empty string>,C in range"
    );

    initialize_index_with_analyzer(
      &mut random,
      dir.clone(),
      &["A", "B", "", "D"],
      SingleCharAnalyzer::new(),
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      3,
      searcher.search(query.clone(), 1000)?.total_hits.value(),
      "A,B,<empty string>,D - A, B and <empty string> in range"
    );

    add_doc(
      &mut random,
      dir.clone(),
      "C",
      &mut doc_count,
      &mut field_types,
    )?;
    let reader = directory_reader::open(dir.clone())?;
    let searcher = new_searcher_with_reader(reader)?;
    assert_eq!(
      4,
      searcher.search(query, 1000)?.total_hits.value(),
      "C added => A,B,<empty string>,C in range"
    );
    Ok(())
  }
  #[cfg(test)]
  impl From<SingleCharAnalyzer> for AnalyzerEnum {
    fn from(_analyzer: SingleCharAnalyzer) -> Self {
      AnalyzerEnum::Custom(BoxedAnalyzer::new(|_field| {
        Ok(TokenStreamComponents::new(
          Box::new(crate::core::search::term_range_query::tests::SingleCharTokenizer::new())
            as Box<dyn TokenStream + Send + Sync>,
          None,
        ))
      }))
    }
  }
  pub struct SingleCharAnalyzer {
    stored_value: AnalyzerStoredValue,
  }

  impl SingleCharAnalyzer {
    pub fn new() -> Self {
      Self {
        stored_value: AnalyzerStoredValue::new(),
      }
    }
  }

  impl Default for SingleCharAnalyzer {
    fn default() -> Self {
      Self::new()
    }
  }

  impl Analyzer for SingleCharAnalyzer {
    fn create_components(&self, _field: &str) -> Result<TokenStreamComponents> {
      Ok(TokenStreamComponents::new(
        Box::new(SingleCharTokenizer::new())
          as Box<dyn crate::core::analysis::token_stream::TokenStream + Send + Sync>,
        None,
      ))
    }

    fn stored_value(&self) -> &AnalyzerStoredValue {
      &self.stored_value
    }

    type TokenStream<TS>
      = TS
    where
      TS: TokenStream;

    fn normalize_from_ts<TS>(&self, field_name: &str, in_: TS) -> Result<Self::TokenStream<TS>>
    where
      TS: TokenStream,
    {
      self.default_normalize_from_ts(field_name, in_)
    }

    fn get_offset_gap(&self, field_name: &str) -> i32 {
      self.default_get_offset_gap(field_name)
    }
  }

  pub struct SingleCharTokenizer {
    buffer: [char; 1],
    done: bool,
    tokenizer_base: TokenizerBase,
  }

  impl SingleCharTokenizer {
    pub fn new() -> Self {
      Self {
        buffer: ['\0'; 1],
        done: false,
        tokenizer_base: TokenizerBase::new(default_attribute()),
      }
    }
  }

  impl TokenStream for SingleCharTokenizer {
    fn increment_token(&mut self) -> Result<bool> {
      if self.done {
        return Ok(false);
      }

      let count = self.tokenizer_base.input.read_buf(&mut self.buffer)?;
      self.tokenizer_base.token_stream_base.att.clear_attributes();
      self.done = true;
      if count == 1 {
        self
          .tokenizer_base
          .token_stream_base
          .att
          .copy_buffer(&self.buffer, 0, 1)?;
      }
      Ok(true)
    }

    fn end(&mut self) -> Result<()> {
      self.tokenizer_base.end()
    }

    fn reset(&mut self) -> Result<()> {
      self.tokenizer_base.reset()?;
      self.done = false;
      Ok(())
    }

    fn close(&mut self) -> Result<()> {
      self.tokenizer_base.close()
    }

    fn get_attribute_source(&self) -> &Attributes {
      self.tokenizer_base.get_attribute_source()
    }

    fn get_attribute_source_mut(&mut self) -> &mut Attributes {
      self.tokenizer_base.get_attribute_source_mut()
    }

    fn set_reader(&mut self, input: crate::core::analysis::reader::ReaderEnum) -> Result<()> {
      self.tokenizer_base.set_reader(input)
    }
  }

  impl Tokenizer for SingleCharTokenizer {
    fn get_tokenizer_base_mut(&mut self) -> &mut TokenizerBase {
      &mut self.tokenizer_base
    }

    fn get_tokenizer_base(&self) -> &TokenizerBase {
      &self.tokenizer_base
    }
  }
}
