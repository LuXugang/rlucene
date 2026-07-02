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
use crate::core::util::automation::automaton::Automaton;
use crate::core::util::automation::finite_strings_iterator::{
  FiniteStringsIterator, FiniteStringsIteratorBase,
};
use crate::core::util::automation::limited_finite_strings_iterator::LimitedFiniteStringsIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::ints_ref::IntsRef;
use std::collections::HashSet;

pub(crate) struct TestOperations;

impl TestOperations {
  /// Returns the set of all accepted strings.
  ///
  /// This method exists primarily to ease testing.
  /// For production code, directly use [`FiniteStringsIterator`] instead.
  ///
  /// See also:
  /// - [`FiniteStringsIterator`]
  pub fn get_finite_strings(a: &Automaton) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let iter = FiniteStringsIterator::new(a)?;
    Self::get_finite_strings_impl(iter)
  }

  /// Returns the set of accepted strings, up to at most `limit` strings.
  ///
  /// This method exists primarily to ease testing.
  /// For production code, directly use [`LimitedFiniteStringsIterator`]
  /// instead.
  ///
  /// See also:
  /// - [`LimitedFiniteStringsIterator`]
  pub fn get_finite_strings_with_limit(
    a: &Automaton,
    limit: i32,
  ) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let iter = LimitedFiniteStringsIterator::new(a, limit)?;
    Self::get_finite_strings_impl(iter)
  }

  /// Get all finite strings of an iterator.
  pub fn get_finite_strings_impl(
    mut iterator: impl FiniteStringsIteratorBase,
  ) -> Result<HashSet<IntsRef<Vec<i32>>>> {
    let mut result = HashSet::new();
    while let Some(finite_string) = iterator.next()? {
      result.insert(IntsRef::deep_copy_of(&finite_string));
    }
    Ok(result)
  }
}
