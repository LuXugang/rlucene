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
use crate::core::analysis::char_array_set::CharArraySet;
use crate::core::analysis::stop_filter::StopFilter;
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::analysis::base_token_stream_test_case::{
  assert_token_stream_contents4, assert_token_stream_contents12,
};
use crate::test_framework::core::analysis::mock_tokenizer::{MockTokenizer, WHITESPACE};
use crate::test_framework::core::util::english::English;
use crate::test_framework::core::util::lucene_test_case::{random, random_from_seed};
use rand::prelude::SliceRandom;
use rand::{Rng, RngExt};
use std::collections::HashSet;
use std::sync::Arc;

const MAX_NUMBER_OF_TOKENS: usize = 50;

#[allow(dead_code)] // for quick search
struct TestStopFilter;

#[test]
fn test_exact_case() -> Result<()> {
  let mut random = random();
  let mut stop_words = CharArraySet::new(false);
  stop_words.add_all(["is", "the", "Time"]);
  let mut input = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    WHITESPACE.clone(),
    false,
  );
  input.set_reader("Now is The Time".into())?;
  let mut stream = StopFilter::new(input, Arc::new(stop_words));
  assert_token_stream_contents12(&mut stream, &["Now", "The"])
}

#[test]
fn test_stop_filter() -> Result<()> {
  let mut random = random();
  let stop_set = StopFilter::make_stop_set(["is", "the", "Time"]);
  let mut input = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    WHITESPACE.clone(),
    false,
  );
  input.set_reader("Now is The Time".into())?;
  let mut stream = StopFilter::new(input, stop_set);
  assert_token_stream_contents12(&mut stream, &["Now", "The"])
}

#[test]
fn test_token_position_with_stopword_filter() -> Result<()> {
  let mut random = random();
  // at least 1 token
  let number_of_tokens = random.random_range(1..MAX_NUMBER_OF_TOKENS);
  let mut text = String::new();
  let mut stop_words = Vec::with_capacity(number_of_tokens);
  let mut stopword_positions = Vec::with_capacity(number_of_tokens);
  generate_test_set_with_stopwords_and_stopword_positions(
    &mut random,
    number_of_tokens,
    &mut text,
    &mut stop_words,
    &mut stopword_positions,
  );

  let stop_set = StopFilter::make_stop_set(&stop_words);
  let mut input = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    WHITESPACE.clone(),
    false,
  );
  input.set_reader(text.into())?;
  let mut stop_filter = StopFilter::new(input, stop_set);
  do_test_stopwords_positions(&mut stop_filter, &stopword_positions, number_of_tokens)
}

#[test]
fn test_token_positions_with_concatenated_stopword_filters() -> Result<()> {
  let mut random = random();
  // at least 1 token
  let number_of_tokens = random.random_range(1..MAX_NUMBER_OF_TOKENS);
  let mut text = String::new();
  let mut stop_words = Vec::with_capacity(number_of_tokens);
  let mut stopword_positions = Vec::new();
  generate_test_set_with_stopwords_and_stopword_positions(
    &mut random,
    number_of_tokens,
    &mut text,
    &mut stop_words,
    &mut stopword_positions,
  );

  // we want to make sure that concatenating two list of stopwords
  // produce the same results of using one unique list of stopwords.
  // So we first generate a list of stopwords:
  // e.g.: [a, b, c, d, e]
  // and then we split the list in two disjoint partitions
  // e.g. [a, c, e] [b, d]
  let partition = random.random_range(0..stop_words.len());
  stop_words.shuffle(&mut random);
  let stop_words_random_partition = stop_words[..partition].to_vec();
  let mut stop_words_remaining: HashSet<String> = stop_words.iter().cloned().collect();
  for stop_word in &stop_words_random_partition {
    stop_words_remaining.remove(stop_word);
  }

  let first_stop_set = StopFilter::make_stop_set(&stop_words_random_partition);
  let second_stop_set = StopFilter::make_stop_set_with_ignore_case(&stop_words_remaining, false);
  let mut input = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    WHITESPACE.clone(),
    false,
  );
  input.set_reader(text.into())?;

  // Here we create a stopFilter with the stopwords in the first partition and then we
  // concatenate it with the stopFilter created with the stopwords in the second partition
  let stop_filter = StopFilter::new(input, first_stop_set);
  let mut concatenated_stop_filter = StopFilter::new(stop_filter, second_stop_set);

  // ... and finally we check that the positions of the filtered tokens matched using the
  // concatenated stopFilters match the positions of the filtered tokens using the unique
  // original list of stopwords
  do_test_stopwords_positions(
    &mut concatenated_stop_filter,
    &stopword_positions,
    number_of_tokens,
  )
}

// LUCENE-3849: make sure after .end() we see the "ending" posInc
#[test]
fn test_end_stopword() -> Result<()> {
  let mut random = random();
  let stop_set = StopFilter::make_stop_set(["of"]);
  let mut input = MockTokenizer::with_default_max_token_length(
    random_from_seed(random.random()),
    WHITESPACE.clone(),
    false,
  );
  input.set_reader("test of".into())?;
  let mut stop_filter = StopFilter::new(input, stop_set);
  assert_token_stream_contents4(
    &mut stop_filter,
    &["test"],
    Some(&[0]),
    Some(&[4]),
    None,
    Some(&[1]),
    None,
    Some(7),
    Some(1),
    None,
    true,
    None,
  )
}

/// Randomly generate a document and a list of stopwords to apply
///
/// - `number_of_tokens`: max number of tokens in the document
/// - `text`: will contain the text at the end of the method
/// - `stop_words`: will contain the list of the stopwords at the end of the method
/// - `stopword_positions`: will contain the position of the stopwords at the end of the method
fn generate_test_set_with_stopwords_and_stopword_positions<R>(
  random: &mut R,
  number_of_tokens: usize,
  text: &mut String,
  stop_words: &mut Vec<String>,
  stopword_positions: &mut Vec<usize>,
) where
  R: Rng + ?Sized,
{
  for i in 0..number_of_tokens {
    let token = English::int_to_english(i as i32).trim().to_string();
    text.push_str(&token);
    text.push(' ');
    if i == 0 || random.random_bool(0.5) {
      // with probability 0.5 will tell if this is a stopword or
      // no - adding always the first token to make sure that the
      // list of stopwords is not empty;
      stop_words.push(token);
      stopword_positions.push(i);
    }
  }
}

fn do_test_stopwords_positions<T>(
  stop_filter: &mut T,
  stopword_positions: &[usize],
  number_of_tokens: usize,
) -> Result<()>
where
  T: TokenStream,
{
  stop_filter.reset()?;
  for i in 0..number_of_tokens {
    if stopword_positions.contains(&i) {
      // if i is in stopwordPosition it is a stopword and we skip this position
      continue;
    }
    assert!(stop_filter.increment_token()?);
    let token = English::int_to_english(i as i32).trim().to_string();
    assert_eq!(token, stop_filter.get_attribute_source().to_string());
  }
  assert!(!stop_filter.increment_token()?);
  stop_filter.end()?;
  stop_filter.close()
}
