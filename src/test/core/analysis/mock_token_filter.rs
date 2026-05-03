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
use crate::core::analysis::token_filter::{TokenFilter, TokenFilterBase};
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::{AttributeSource, Attributes};
use crate::core::util::automation::automata::Automata;
use crate::core::util::automation::character_run_automaton::CharacterRunAutomaton;
use crate::core::util::automation::operations::Operations;
use crate::core::util::error::lucene_error::Result;
use std::sync::LazyLock;

/// Empty set of stopwords.
pub static EMPTY_STOPSET: LazyLock<CharacterRunAutomaton> =
  LazyLock::new(|| CharacterRunAutomaton::new(Automata::make_empty().expect("")).expect(""));

/// Set of common english stopwords.
pub static ENGLISH_STOPSET: LazyLock<CharacterRunAutomaton> = LazyLock::new(|| {
  let automata = [
    Automata::make_string("a").expect(""),
    Automata::make_string("an").expect(""),
    Automata::make_string("and").expect(""),
    Automata::make_string("are").expect(""),
    Automata::make_string("as").expect(""),
    Automata::make_string("at").expect(""),
    Automata::make_string("be").expect(""),
    Automata::make_string("but").expect(""),
    Automata::make_string("by").expect(""),
    Automata::make_string("for").expect(""),
    Automata::make_string("if").expect(""),
    Automata::make_string("in").expect(""),
    Automata::make_string("into").expect(""),
    Automata::make_string("is").expect(""),
    Automata::make_string("it").expect(""),
    Automata::make_string("no").expect(""),
    Automata::make_string("not").expect(""),
    Automata::make_string("of").expect(""),
    Automata::make_string("on").expect(""),
    Automata::make_string("or").expect(""),
    Automata::make_string("such").expect(""),
    Automata::make_string("that").expect(""),
    Automata::make_string("the").expect(""),
    Automata::make_string("their").expect(""),
    Automata::make_string("then").expect(""),
    Automata::make_string("there").expect(""),
    Automata::make_string("these").expect(""),
    Automata::make_string("they").expect(""),
    Automata::make_string("this").expect(""),
    Automata::make_string("to").expect(""),
    Automata::make_string("was").expect(""),
    Automata::make_string("will").expect(""),
    Automata::make_string("with").expect(""),
  ];
  let refs = automata.iter().collect::<Vec<_>>();
  let union = Operations::union_list(&refs).expect("union should not fail");
  let deterministic = Operations::determinize(&union, Operations::DEFAULT_DETERMINIZE_WORK_LIMIT)
    .expect("determinize should not fail")
    .into_owned();
  CharacterRunAutomaton::new(deterministic).expect("english stopset automaton should be valid")
});

/// A tokenfilter for testing that removes terms accepted by a DFA.
///
/// - Union a list of singletons to act like a stopfilter.
/// - Use the complement to act like a keepwordfilter
/// - Use a regex like `.{12,}` to act like a lengthfilter
pub struct MockTokenFilter<TS>
where
  TS: TokenStream,
{
  filter: CharacterRunAutomaton,
  skipped_positions: i32,
  token_filter_base: TokenFilterBase<TS>,
}

impl<TS> MockTokenFilter<TS>
where
  TS: TokenStream,
{
  /// Create a new `MockTokenFilter`.
  ///
  /// `filter` is a DFA representing the terms that should be removed.
  pub fn new(input: TS, filter: CharacterRunAutomaton) -> Self {
    Self {
      filter,
      skipped_positions: 0,
      token_filter_base: TokenFilterBase::new(input),
    }
  }

  fn is_filtered(&self, attr: &Attributes) -> Result<bool> {
    let len = attr.length()?;
    let term = attr.buffer()?[..len].iter().collect::<String>();
    self.filter.run_str(&term)
  }
}

impl<TS> TokenStream for MockTokenFilter<TS>
where
  TS: TokenStream,
{
  fn increment_token(&mut self) -> Result<bool> {
    self.skipped_positions = 0;
    while self.token_filter_base.input.increment_token()? {
      if !self.is_filtered(self.token_filter_base.input.get_attribute_source())? {
        let attr = self.token_filter_base.input.get_attribute_source_mut();
        attr.set_position_increment(attr.get_position_increment()? + self.skipped_positions)?;
        return Ok(true);
      }

      let attr = self.token_filter_base.input.get_attribute_source();
      self.skipped_positions += attr.get_position_increment()?;
    }
    Ok(false)
  }

  fn end(&mut self) -> Result<()> {
    self.token_filter_base.end()?;
    let attr = self.token_filter_base.input.get_attribute_source_mut();
    attr.set_position_increment(attr.get_position_increment()? + self.skipped_positions)?;
    Ok(())
  }

  fn reset(&mut self) -> Result<()> {
    self.token_filter_base.reset()?;
    self.skipped_positions = 0;
    Ok(())
  }

  fn close(&mut self) -> Result<()> {
    self.token_filter_base.close()
  }

  fn get_attribute_source(&self) -> &Attributes {
    self.token_filter_base.input.get_attribute_source()
  }

  fn get_attribute_source_mut(&mut self) -> &mut Attributes {
    self.token_filter_base.input.get_attribute_source_mut()
  }
}

impl<TS> TokenFilter for MockTokenFilter<TS> where TS: TokenStream {}
