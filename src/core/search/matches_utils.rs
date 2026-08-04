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
use crate::core::search::disjunction_matches_iterator::from_sub_iterators;
use crate::core::search::matches::Matches;
use crate::core::search::query::{QueryWeightMatches, QueryWeightMatchesIterator};
use crate::core::util::error::lucene_error::Result;
use std::cell::RefCell;
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub struct MatchesUtils;

pub static MATCH_WITH_NO_TERMS: LazyLock<MatchWithNoTerms> = LazyLock::new(|| MatchWithNoTerms);
pub struct MatchWithNoTerms;
impl Matches for MatchWithNoTerms {
  fn get_matches(&self, _field: &str) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    Ok(None)
  }

  fn get_sub_matches(&self) -> Vec<&QueryWeightMatches<'_>> {
    Vec::new()
  }

  fn field(&self) -> &[String] {
    &[]
  }
}

pub struct CombinedMatch<'a> {
  sub: Vec<QueryWeightMatches<'a>>,
  fields: Vec<String>,
}
impl<'a> CombinedMatch<'a> {
  pub fn new(sub: Vec<QueryWeightMatches<'a>>) -> Self {
    let mut fields = Vec::new();
    for matches in &sub {
      for field in matches.field() {
        if !fields.contains(field) {
          fields.push(field.clone());
        }
      }
    }
    CombinedMatch { sub, fields }
  }
}

impl Matches for CombinedMatch<'_> {
  fn get_matches(&self, field: &str) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    let mut sub_iterators = Vec::new();
    for matches in &self.sub {
      if let Some(iterator) = matches.get_matches(field)? {
        sub_iterators.push(iterator);
      }
    }
    from_sub_iterators(sub_iterators)
  }

  fn get_sub_matches(&self) -> Vec<&QueryWeightMatches<'_>> {
    self.sub.iter().collect()
  }

  fn field(&self) -> &[String] {
    &self.fields
  }
}

pub fn from_sub_matches<'a>(
  mut sub_matches: Vec<QueryWeightMatches<'a>>,
) -> Option<QueryWeightMatches<'a>> {
  if sub_matches.is_empty() {
    return None;
  }

  let mut match_index = None;
  let mut match_count = 0;
  for (index, matches) in sub_matches.iter().enumerate() {
    if !matches!(matches, QueryWeightMatches::MatchWithNoTerms(_)) {
      match_index = Some(index);
      match_count += 1;
    }
  }

  if match_count == 0 {
    return Some(QueryWeightMatches::MatchWithNoTerms(MatchWithNoTerms));
  }
  if match_count == 1 {
    return Some(sub_matches.swap_remove(match_index.unwrap()));
  }
  Some(QueryWeightMatches::Matches(Box::new(CombinedMatch::new(
    sub_matches,
  ))))
}

pub struct FieldMatches<'a, F>
where
  F: Fn() -> Result<Option<QueryWeightMatchesIterator<'a>>>,
{
  field: String,
  fields: Vec<String>,
  supplier: F,
  cached: RefCell<Option<QueryWeightMatchesIterator<'a>>>,
}

impl<'a, F> Matches for FieldMatches<'a, F>
where
  F: Fn() -> Result<Option<QueryWeightMatchesIterator<'a>>>,
{
  fn get_matches(&self, field: &str) -> Result<Option<QueryWeightMatchesIterator<'_>>> {
    if field != self.field {
      return Ok(None);
    }
    if let Some(iterator) = self.cached.borrow_mut().take() {
      Ok(Some(iterator))
    } else {
      (self.supplier)()
    }
  }

  fn get_sub_matches(&self) -> Vec<&QueryWeightMatches<'_>> {
    Vec::new()
  }

  fn field(&self) -> &[String] {
    &self.fields
  }
}

pub fn for_field<'a, F>(field: String, supplier: F) -> Result<Option<QueryWeightMatches<'a>>>
where
  F: Fn() -> Result<Option<QueryWeightMatchesIterator<'a>>> + 'a,
{
  let first = supplier()?;
  let Some(first) = first else {
    return Ok(None);
  };
  Ok(Some(QueryWeightMatches::Matches(Box::new(FieldMatches {
    fields: vec![field.clone()],
    field,
    supplier,
    cached: RefCell::new(Some(first)),
  }))))
}
