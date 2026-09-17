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
use crate::core::index::filtered_terms_enum::{
  AcceptStatus, FilteredTermsEnum, FilteredTermsEnumBase,
};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;
/// [`FilteredTermsEnum`] implementation for enumerating a single term.
///
/// For example, this can be used by [`MultiTermQuery`](crate::core::search::multi_term_query::MultiTermQuery)s that need only visit one term, but
/// want to preserve [`MultiTermQuery`](crate::core::search::multi_term_query::MultiTermQuery) semantics such as [`MultiTermQuery`](crate::core::search::multi_term_query::MultiTermQuery).
pub struct SingleTermsEnum {
  single_ref: BytesRef<Vec<u8>>,
}
impl SingleTermsEnum {
  pub fn new<T>(te: T, term_text: BytesRef<Vec<u8>>) -> FilteredTermsEnum<T, SingleTermsEnum>
  where
    T: TermsEnum,
  {
    let sub = SingleTermsEnum {
      single_ref: term_text,
    };
    FilteredTermsEnum::new(te, sub)
  }
}
impl FilteredTermsEnumBase for SingleTermsEnum {
  fn next_seek_term(
    &mut self,
    current: Option<&BytesRef<&[u8]>>,
  ) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
    if current.is_none() {
      Ok(Some(Cow::Borrowed(&self.single_ref)))
    } else {
      Ok(None)
    }
  }

  fn accept(
    &mut self,
    term: &BytesRef<&[u8]>,
    _ord: i64,
  ) -> crate::core::util::error::lucene_error::Result<AcceptStatus> {
    if term.compare_to(&self.single_ref).is_eq() {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::End)
    }
  }
}
