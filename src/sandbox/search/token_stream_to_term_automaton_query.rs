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
use crate::core::analysis::token_stream::TokenStream;
use crate::core::util::attribute_source::AttributeSource;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::sandbox::search::term_automaton_query::TermAutomatonQuery;

/// Consumes a `TokenStream` and creates a `TermAutomatonQuery` where the transition labels are
/// tokens from the `TermToBytesRefAttribute`.
///
/// This code is very new and likely has exciting bugs!
///
/// Experimental: this API follows the original Lucene experimental status.
pub struct TokenStreamToTermAutomatonQuery {
  preserve_position_increments: bool,
}

impl Default for TokenStreamToTermAutomatonQuery {
  fn default() -> Self {
    Self::new()
  }
}

impl TokenStreamToTermAutomatonQuery {
  /// Sole constructor.
  pub fn new() -> Self {
    Self {
      preserve_position_increments: true,
    }
  }

  /// Whether to generate holes in the automaton for missing positions, `true` by default.
  pub fn set_preserve_position_increments(&mut self, enable_position_increments: bool) {
    self.preserve_position_increments = enable_position_increments;
  }

  /// Pulls the graph (including `PositionLengthAttribute`) from the provided `TokenStream`, and
  /// creates the corresponding automaton where arcs are bytes from each term.
  pub fn to_query<TS>(&self, field: &str, input: &mut TS) -> Result<TermAutomatonQuery>
  where
    TS: TokenStream,
  {
    input.reset()?;

    let mut query = TermAutomatonQuery::new(field);

    let mut pos = -1;
    let mut max_offset = 0;
    let mut max_pos = -1;
    let mut state = -1;
    while input.increment_token()? {
      let attributes = input.get_attribute_source_mut();
      let mut pos_inc = attributes.get_position_increment()?;
      if !self.preserve_position_increments && pos_inc > 1 {
        pos_inc = 1;
      }
      debug_assert!(pos > -1 || pos_inc > 0);

      if pos_inc > 1 {
        return Err(LuceneError::illegal_argument(
          "cannot handle holes; to accept any term, use '*' term",
        ));
      }

      if pos_inc > 0 {
        // New node:
        pos += pos_inc;
      }

      let end_pos = pos + attributes.get_position_length()?;
      while state < end_pos {
        state = query.create_state();
      }

      let end_offset = attributes.end_offset()?;
      let term = attributes
        .get_bytes_ref()?
        .ok_or_else(|| LuceneError::illegal_state("TermToBytesRefAttribute is missing"))?;
      // println!("{pos}-{end_pos}: {}: pos_inc={pos_inc}", term.utf8_to_string()?);
      if term.length == 1 && term.bytes[term.offset] == b'*' {
        query.add_any_transition(pos, end_pos)?;
      } else {
        query.add_transition_bytes(pos, end_pos, &term)?;
      }

      max_offset = max_offset.max(end_offset);
      max_pos = max_pos.max(end_pos);
    }

    input.end()?;

    let _ = max_offset;
    let _ = max_pos;

    query.set_accept(state, true);
    query.finish()?;

    Ok(query)
  }
}
