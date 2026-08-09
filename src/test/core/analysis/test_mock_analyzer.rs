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
use crate::core::analysis::analyzer::{Analyzer, AnalyzerStoredValue, TokenStreamComponents};
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::directory_reader;
use crate::core::index::fields::Fields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::term_vectors::TermVectors;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::automation::reg_exp::RegExp;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::base_token_stream_test_case::{
  assert_analyzes_to6, assert_analyzes_to8, assert_analyzes_to9, assert_token_stream_contents18,
  check_one_term,
};
use crate::test_framework::core::analysis::mock_analyzer::{ENGLISH_STOPSET, MockAnalyzer};
use crate::test_framework::core::analysis::mock_char_filter::MockCharFilter;
use crate::test_framework::core::analysis::mock_tokenizer::{
  KEYWORD, MockTokenizer, SIMPLE, WHITESPACE,
};
use crate::test_framework::core::util::automaton::automaton_test_util::AutomatonTestUtil;
use crate::test_framework::core::util::lucene_test_case::{
  at_least, get_only_leaf_reader, new_directory_shared, new_index_writer_config_with_analyzer,
  random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use parking_lot::Mutex;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[allow(dead_code)] // for quick search
struct TestMockAnalyzer;

/// Test a configuration that behaves a lot like WhitespaceAnalyzer
#[test]
fn test_whitespace() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::new(&mut random);
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "A bc defg hiJklmn opqrstuv wxy z ",
    &["a", "bc", "defg", "hijklmn", "opqrstuv", "wxy", "z"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "aba cadaba shazam",
    &["aba", "cadaba", "shazam"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "break on whitespace",
    &["break", "on", "whitespace"],
  )
}

/// Test a configuration that behaves a lot like SimpleAnalyzer
#[test]
fn test_simple() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::with_automaton(&mut random, SIMPLE.clone(), true);
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "a-bc123 defg+hijklmn567opqrstuv78wxy_z ",
    &["a", "bc", "defg", "hijklmn", "opqrstuv", "wxy", "z"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "aba4cadaba-Shazam",
    &["aba", "cadaba", "shazam"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "break+on/Letters",
    &["break", "on", "letters"],
  )
}

/// Test a configuration that behaves a lot like KeywordAnalyzer
#[test]
fn test_keyword() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::with_automaton(&mut random, KEYWORD.clone(), false);
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "a-bc123 defg+hijklmn567opqrstuv78wxy_z ",
    &["a-bc123 defg+hijklmn567opqrstuv78wxy_z "],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "aba4cadaba-Shazam",
    &["aba4cadaba-Shazam"],
  )?;
  assert_analyzes_to6(
    &mut random,
    &analyzer,
    "break+on/Nothing",
    &["break+on/Nothing"],
  )?;
  // currently though emits no tokens for empty string: maybe we can do it,
  // but we don't want to emit tokens infinitely...
  assert_analyzes_to6(&mut random, &analyzer, "", &[])
}

/// Test a configuration where each character is a term.
#[test]
fn test_single_char() -> Result<()> {
  let mut random = random();
  let single = CharacterRunAutomaton::new(RegExp::from_string(".")?.to_automaton()?)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, single, false);
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "foobar",
    &["f", "o", "o", "b", "a", "r"],
    Some(&[0, 1, 2, 3, 4, 5]),
    Some(&[1, 2, 3, 4, 5, 6]),
  )?;
  check_random_data(&mut random, &analyzer, 100)
}

/// Test a configuration where two characters make a term.
#[test]
fn test_two_chars() -> Result<()> {
  let mut random = random();
  let two_chars = CharacterRunAutomaton::new(RegExp::from_string("..")?.to_automaton()?)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, two_chars, false);
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "foobar",
    &["fo", "ob", "ar"],
    Some(&[0, 2, 4]),
    Some(&[2, 4, 6]),
  )?;

  let mut stream = analyzer.token_stream("bogus", ReaderEnum::from("fooba"))?;
  assert_token_stream_contents18(
    &mut *stream,
    &["fo", "ob"],
    Some(&[0, 2]),
    Some(&[2, 4]),
    Some(&[1, 1]),
    Some(5),
  )?;
  drop(stream);

  check_random_data(&mut random, &analyzer, 100)
}

/// Test a configuration where three characters make a term.
#[test]
fn test_three_chars() -> Result<()> {
  let mut random = random();
  let three_chars = CharacterRunAutomaton::new(RegExp::from_string("...")?.to_automaton()?)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, three_chars, false);
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "foobar",
    &["foo", "bar"],
    Some(&[0, 3]),
    Some(&[3, 6]),
  )?;

  let mut stream = analyzer.token_stream("bogus", ReaderEnum::from("fooba"))?;
  assert_token_stream_contents18(
    &mut *stream,
    &["foo"],
    Some(&[0]),
    Some(&[3]),
    Some(&[1]),
    Some(5),
  )?;
  drop(stream);

  check_random_data(&mut random, &analyzer, 100)
}

/// Test a configuration where each word starts with one uppercase character.
#[test]
fn test_uppercase() -> Result<()> {
  let mut random = random();
  let uppercase = CharacterRunAutomaton::new(RegExp::from_string("[A-Z][a-z]*")?.to_automaton()?)?;
  let analyzer = MockAnalyzer::with_automaton(&mut random, uppercase, false);
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "FooBarBAZ",
    &["Foo", "Bar", "B", "A", "Z"],
    Some(&[0, 3, 6, 7, 8]),
    Some(&[3, 6, 7, 8, 9]),
  )?;
  assert_analyzes_to9(
    &mut random,
    &analyzer,
    "aFooBar",
    &["Foo", "Bar"],
    Some(&[1, 4]),
    Some(&[4, 7]),
  )?;
  check_random_data(&mut random, &analyzer, 100)
}

/// Test a configuration that behaves a lot like StopAnalyzer
#[test]
fn test_stop() -> Result<()> {
  let mut random = random();
  let analyzer =
    MockAnalyzer::with_filter(&mut random, SIMPLE.clone(), true, ENGLISH_STOPSET.clone());
  assert_analyzes_to8(
    &mut random,
    &analyzer,
    "the quick brown a fox",
    &["quick", "brown", "fox"],
    Some(&[2, 1, 2]),
  )
}

/// Test a configuration that behaves a lot like KeepWordFilter
#[test]
fn test_keep() -> Result<()> {
  let mut random = random();
  let foo = Automata::make_string("foo")?;
  let bar = Automata::make_string("bar")?;
  let union = Operations::union_list(&[&foo, &bar])?;
  let keep_words = CharacterRunAutomaton::new(Operations::complement(
    &union,
    Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
  )?)?;
  let analyzer = MockAnalyzer::with_filter(&mut random, SIMPLE.clone(), true, keep_words);
  assert_analyzes_to8(
    &mut random,
    &analyzer,
    "quick foo brown bar bar fox foo",
    &["foo", "bar", "bar", "foo"],
    Some(&[2, 2, 1, 2]),
  )
}

/// Test a configuration that behaves a lot like LengthFilter
#[test]
fn test_length() -> Result<()> {
  let mut random = random();
  let length_5 = CharacterRunAutomaton::new(RegExp::from_string(".{5,}")?.to_automaton()?)?;
  let analyzer = MockAnalyzer::with_filter(&mut random, WHITESPACE.clone(), true, length_5);
  assert_analyzes_to8(
    &mut random,
    &analyzer,
    "ok toolong fine notfine",
    &["ok", "fine"],
    Some(&[1, 2]),
  )
}

/// Test MockTokenizer encountering a too long token
#[test]
fn test_too_long_token() -> Result<()> {
  let mut random = random();
  let whitespace = TooLongTokenAnalyzer {
    random: Mutex::new(StdRng::seed_from_u64(random.random())),
    stored_value: AnalyzerStoredValue::global(),
  };

  let mut stream = whitespace.token_stream("bogus", ReaderEnum::from("test 123 toolong ok "))?;
  assert_token_stream_contents18(
    &mut *stream,
    &["test", "123", "toolo", "ng", "ok"],
    Some(&[0, 5, 9, 14, 17]),
    Some(&[4, 8, 14, 16, 19]),
    Some(&[1, 1, 1, 1, 1]),
    Some(20),
  )?;
  drop(stream);

  let mut stream = whitespace.token_stream("bogus", ReaderEnum::from("test 123 toolo"))?;
  assert_token_stream_contents18(
    &mut *stream,
    &["test", "123", "toolo"],
    Some(&[0, 5, 9]),
    Some(&[4, 8, 14]),
    Some(&[1, 1, 1]),
    Some(14),
  )
}

#[test]
fn test_lucene_3042() -> Result<()> {
  let mut random = random();
  let test_string = "t";
  let analyzer = MockAnalyzer::new(&mut random);
  {
    let mut stream = analyzer.token_stream("dummy", ReaderEnum::from(test_string))?;
    stream.reset()?;
    while stream.increment_token()? {
      // consume
    }
    stream.end()?;
    stream.close()?;
  }
  assert_analyzes_to6(&mut random, &analyzer, test_string, &["t"])
}

/// Blast some random strings through the analyzer and verify reusable streams are deterministic.
#[test]
fn test_random_strings() -> Result<()> {
  let mut random = random();
  let analyzer = MockAnalyzer::new(&mut random);
  let iterations = at_least(&mut random, 1000);
  check_random_data(&mut random, &analyzer, iterations)
}

/// Blast random strings through differently configured tokenizers.
#[test]
fn test_random_regexps() -> Result<()> {
  let mut random = random();
  let minimum = if cfg!(feature = "nightly") { 30 } else { 1 };
  let iterations = at_least(&mut random, minimum);
  for _ in 0..iterations {
    let automaton = AutomatonTestUtil::random_automaton(&mut random)?;
    let automaton = Operations::determinize(
      automaton.as_ref(),
      Operations::DEFAULT_DETERMINIZE_WORK_LIMIT,
    )?;
    let dfa = CharacterRunAutomaton::new(automaton.into_owned())?;
    let lowercase = random.random();
    let limit = TestUtil::next_int(&mut random, 0, 500);
    let mut analyzer = RandomRegexpsAnalyzer {
      dfa,
      lowercase,
      limit,
      random: Mutex::new(StdRng::seed_from_u64(random.random())),
      stored_value: AnalyzerStoredValue::global(),
    };
    check_random_data(&mut random, &analyzer, 100)?;
    analyzer.close()?;
  }
  Ok(())
}

#[test]
fn test_forward_offsets() -> Result<()> {
  let mut random = random();
  let num = at_least(&mut random, 1000);
  for _ in 0..num {
    let string = TestUtil::random_htmlish_string(&mut random, 20);
    let char_filter = MockCharFilter::new(ReaderEnum::from(string), 2)?;
    let analyzer = MockAnalyzer::new(&mut random);
    let mut token_stream =
      analyzer.token_stream("bogus", ReaderEnum::MockCharFilter(char_filter))?;
    token_stream.reset()?;
    while token_stream.increment_token()? {}
    token_stream.end()?;
    token_stream.close()?;
  }
  Ok(())
}

#[test]
fn test_wrap_reader() -> Result<()> {
  let mut random = random();
  let analyzer = WrappingMockAnalyzer {
    delegate: MockAnalyzer::new(&mut random),
    stored_value: AnalyzerStoredValue::global(),
  };
  check_one_term(&mut random, &analyzer, "abc", "aabc")
}

#[test]
fn test_change_gaps() -> Result<()> {
  let mut random = random();
  let position_gap = random.random_range(0..1000);
  let offset_gap = random.random_range(0..1000);
  // TODO IMPORTANT AnalyzerWrapper未实现
  let mut analyzer = MockAnalyzer::new(&mut random);
  analyzer.set_position_increment_gap(position_gap);
  analyzer.set_offset_gap(offset_gap);

  let dir = new_directory_shared(&mut random)?;
  let config = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut field_type = FieldType::new();
  field_type.set_index_options(IndexOptions::Docs)?;
  field_type.set_tokenized(true)?;
  field_type.set_store_term_vectors(true)?;
  field_type.set_store_term_vector_positions(true)?;
  field_type.set_store_term_vector_offsets(true)?;

  let mut doc = Document::new();
  doc.add(Field::new("f", "a", field_type.clone()));
  doc.add(Field::new("f", "a", field_type));
  writer.add_document(doc)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let leaf = get_only_leaf_reader(&reader)?;
  let mut term_vectors = leaf.term_vectors()?;
  let fields = term_vectors.get(0)?.expect("term vectors should exist");
  let terms = fields.terms("f")?.expect("field f should have terms");
  let mut terms_enum = terms.iterator()?;
  assert_eq!(
    BytesRef::from_string("a"),
    terms_enum
      .next()?
      .expect("term a should exist")
      .into_owned()
  );
  let mut postings = terms_enum.postings_with_flags(None, ALL as i32)?;
  assert_eq!(0, postings.next_doc()?);
  assert_eq!(2, postings.freq()?);
  assert_eq!(0, postings.next_position()?);
  assert_eq!(0, postings.start_offset()?);
  let first_end_offset = postings.end_offset()?;
  assert_eq!(1 + position_gap, postings.next_position()?);
  assert_eq!(1 + first_end_offset + offset_gap, postings.end_offset()?);
  assert!(terms_enum.next()?.is_none());

  reader.close()?;
  writer.close()?;
  dir.as_ref().close()
}

#[derive(Debug, Eq, PartialEq)]
struct AnalysisSnapshot {
  tokens: Vec<(String, i32, i32, i32, i32, String)>,
  final_offset: i32,
}

fn analyze<A>(analyzer: &A, text: &str) -> Result<AnalysisSnapshot>
where
  A: Analyzer,
{
  let mut stream = analyzer.token_stream("dummy", ReaderEnum::from(text))?;
  stream.reset()?;
  let mut tokens = Vec::new();
  while stream.increment_token()? {
    let attributes = stream.get_attribute_source();
    tokens.push((
      attributes.to_string(),
      attributes.start_offset()?,
      attributes.end_offset()?,
      attributes.get_position_increment()?,
      attributes.get_position_length()?,
      attributes.type_()?.to_string(),
    ));
  }
  stream.end()?;
  let final_offset = stream.get_attribute_source().end_offset()?;
  stream.close()?;
  Ok(AnalysisSnapshot {
    tokens,
    final_offset,
  })
}

fn check_random_data<R, A>(random: &mut R, analyzer: &A, iterations: i32) -> Result<()>
where
  R: rand::Rng + ?Sized,
  A: Analyzer,
{
  for _ in 0..iterations {
    let text = TestUtil::random_analysis_string(random, 20, false);
    let first = analyze(analyzer, &text)?;
    let second = analyze(analyzer, &text)?;
    assert_eq!(first, second, "analysis was not reproducible for {text:?}");
    analyzer.normalize("dummy", &text)?;
  }
  Ok(())
}

struct WrappingMockAnalyzer {
  delegate: MockAnalyzer,
  stored_value: AnalyzerStoredValue,
}

impl Analyzer for WrappingMockAnalyzer {
  fn create_components(&self, field_name: &str) -> Result<TokenStreamComponents> {
    self.delegate.create_components(field_name)
  }

  fn init_reader(&self, _field_name: &str, reader: ReaderEnum) -> ReaderEnum {
    ReaderEnum::MockCharFilter(MockCharFilter::new(reader, 7).expect("valid remainder"))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(WrappingMockAnalyzer);

struct TooLongTokenAnalyzer {
  random: Mutex<StdRng>,
  stored_value: AnalyzerStoredValue,
}

impl Analyzer for TooLongTokenAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::with_automaton(
      StdRng::seed_from_u64(self.random.lock().random()),
      WHITESPACE.clone(),
      false,
      5,
    );
    Ok(TokenStreamComponents::new(
      Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(TooLongTokenAnalyzer);

struct RandomRegexpsAnalyzer {
  dfa: CharacterRunAutomaton,
  lowercase: bool,
  limit: i32,
  random: Mutex<StdRng>,
  stored_value: AnalyzerStoredValue,
}

impl Analyzer for RandomRegexpsAnalyzer {
  fn create_components(&self, _field_name: &str) -> Result<TokenStreamComponents> {
    let tokenizer = MockTokenizer::with_automaton(
      StdRng::seed_from_u64(self.random.lock().random()),
      self.dfa.clone(),
      self.lowercase,
      self.limit,
    );
    Ok(TokenStreamComponents::new(
      Box::new(tokenizer) as Box<dyn TokenStream + Send + Sync>,
      None,
    ))
  }

  fn stored_value(&self) -> &AnalyzerStoredValue {
    &self.stored_value
  }
}

crate::impl_analyzer_close!(RandomRegexpsAnalyzer);
