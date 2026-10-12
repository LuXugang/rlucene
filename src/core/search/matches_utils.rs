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
use crate::core::index::index_reader_context::{IRCLeafReader, IndexReaderContext};
use crate::core::search::disjunction_matches_iterator::from_sub_iterators;
use crate::core::search::matches::Matches;
use crate::core::search::query::{QueryWeightMatches, QueryWeightMatchesIterator};
use crate::core::search::query_matches::FieldMatchesSupplier;
use crate::core::util::error::lucene_error::Result;
use std::cell::{OnceCell, RefCell};
use std::sync::LazyLock;

#[allow(dead_code)] // for quick search
pub struct MatchesUtils;

pub static MATCH_WITH_NO_TERMS: LazyLock<MatchWithNoTerms> = LazyLock::new(|| MatchWithNoTerms);
pub struct MatchWithNoTerms;
impl Matches for MatchWithNoTerms {
  type Iterator<'a> = crate::core::search::dummy::dummy_matches_iterator::DummyMatchesIterator;
  type SubMatches = Self;
  fn get_matches(&self, _field: &str) -> Result<Option<Self::Iterator<'_>>> {
    Ok(None)
  }

  fn get_sub_matches(&self) -> &[Self::SubMatches] {
    &[]
  }

  fn field(&self) -> &[String] {
    &[]
  }
}

pub struct CombinedMatch<'a, IRC: IndexReaderContext + 'static> {
  sub: Vec<QueryWeightMatches<'a, IRC>>,
  fields: OnceCell<Vec<String>>,
}
impl<'a, IRC: IndexReaderContext + 'static> CombinedMatch<'a, IRC> {
  pub fn new(sub: Vec<QueryWeightMatches<'a, IRC>>) -> Self {
    CombinedMatch {
      sub,
      fields: OnceCell::new(),
    }
  }
}

impl<'a, IRC: IndexReaderContext + 'static> Matches for CombinedMatch<'a, IRC> {
  type Iterator<'b>
    = QueryWeightMatchesIterator<'b, IRCLeafReader<IRC>>
  where
    Self: 'b;
  type SubMatches = QueryWeightMatches<'a, IRC>;
  fn get_matches(
    &self,
    field: &str,
  ) -> Result<Option<QueryWeightMatchesIterator<'_, IRCLeafReader<IRC>>>> {
    let mut sub_iterators = Vec::with_capacity(self.sub.len());
    for matches in &self.sub {
      if let Some(iterator) = matches.get_matches(field)? {
        sub_iterators.push(iterator);
      }
    }
    from_sub_iterators(sub_iterators)
  }

  fn get_sub_matches(&self) -> &[QueryWeightMatches<'a, IRC>] {
    &self.sub
  }

  fn field(&self) -> &[String] {
    self.fields.get_or_init(|| {
      let mut fields = Vec::new();
      for matches in &self.sub {
        for field in matches.field() {
          if !fields.contains(field) {
            fields.push(field.clone());
          }
        }
      }
      fields
    })
  }
}

pub fn from_sub_matches<'a, IRC: IndexReaderContext + 'static>(
  mut sub_matches: Vec<QueryWeightMatches<'a, IRC>>,
) -> Option<QueryWeightMatches<'a, IRC>> {
  if sub_matches.is_empty() {
    return None;
  }

  let mut match_index = 0;
  let mut match_count = 0;
  for (index, matches) in sub_matches.iter().enumerate() {
    if !matches!(matches, QueryWeightMatches::MatchWithNoTerms(_)) {
      match_index = index;
      match_count += 1;
    }
  }

  if match_count == 0 {
    return Some(QueryWeightMatches::MatchWithNoTerms(MatchWithNoTerms));
  }
  if match_count == 1 {
    return Some(sub_matches.swap_remove(match_index));
  }
  Some(QueryWeightMatches::Combined(Box::new(CombinedMatch::new(
    sub_matches,
  ))))
}

pub struct FieldMatches<'a, IRC: IndexReaderContext + 'static> {
  field: FieldName<'a>,
  supplier: FieldMatchesSupplier<'a, IRC>,
  cached: RefCell<Option<QueryWeightMatchesIterator<'a, IRCLeafReader<IRC>>>>,
}

impl<'a, IRC: IndexReaderContext + 'static> Matches for FieldMatches<'a, IRC> {
  type Iterator<'b>
    = QueryWeightMatchesIterator<'b, IRCLeafReader<IRC>>
  where
    Self: 'b;
  type SubMatches = QueryWeightMatches<'a, IRC>;
  fn get_matches(
    &self,
    field: &str,
  ) -> Result<Option<QueryWeightMatchesIterator<'_, IRCLeafReader<IRC>>>> {
    if field != self.field.as_ref().as_str() {
      return Ok(None);
    }
    if let Some(iterator) = self.cached.borrow_mut().take() {
      Ok(Some(iterator))
    } else {
      self.supplier.get()
    }
  }

  fn get_sub_matches(&self) -> &[QueryWeightMatches<'a, IRC>] {
    &[]
  }

  fn field(&self) -> &[String] {
    std::slice::from_ref(self.field.as_ref())
  }
}

pub enum FieldName<'a> {
  Borrowed(&'a String),
  Owned(String),
}

impl AsRef<String> for FieldName<'_> {
  fn as_ref(&self) -> &String {
    match self {
      Self::Borrowed(field) => field,
      Self::Owned(field) => field,
    }
  }
}

impl<'a> From<&'a String> for FieldName<'a> {
  fn from(field: &'a String) -> Self {
    Self::Borrowed(field)
  }
}

impl From<String> for FieldName<'_> {
  fn from(field: String) -> Self {
    Self::Owned(field)
  }
}

pub fn for_field<'a, IRC: IndexReaderContext + 'static, FName>(
  field: FName,
  supplier: FieldMatchesSupplier<'a, IRC>,
) -> Result<Option<QueryWeightMatches<'a, IRC>>>
where
  FName: Into<FieldName<'a>>,
{
  let field = field.into();
  let first = supplier.get()?;
  let Some(first) = first else {
    return Ok(None);
  };
  Ok(Some(QueryWeightMatches::Field(Box::new(FieldMatches {
    field,
    supplier,
    cached: RefCell::new(Some(first)),
  }))))
}
