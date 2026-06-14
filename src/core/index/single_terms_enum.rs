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
/// `FilteredTermsEnum` implementation for enumerating a single term.
///
/// For example, this can be used by [`MultiTermQuery`](crate::core::search::multi_term_query::MultiTermQuery)s that need only visit one term, but
/// want to preserve `MultiTermQuery` semantics such as `MultiTermQuery::get_rewrite_method`.
pub struct SingleTermsEnum {
  single_ref: BytesRef<Vec<u8>>,
}
impl SingleTermsEnum {
  pub fn new<T>(te: T, term_text: BytesRef<Vec<u8>>) -> FilteredTermsEnum<T, SingleTermsEnum>
  where
    T: TermsEnum,
  {
    let sub = SingleTermsEnum {
      single_ref: term_text.clone(),
    };
    let mut v = FilteredTermsEnum::new(te, sub);
    v.set_initial_seek_term(term_text);
    v
  }
}
impl FilteredTermsEnumBase for SingleTermsEnum {
  fn accept(
    &mut self,
    term: &BytesRef<Vec<u8>>,
    _ord: i64,
  ) -> crate::core::util::error::lucene_error::Result<AcceptStatus> {
    if term == &self.single_ref {
      Ok(AcceptStatus::Yes)
    } else {
      Ok(AcceptStatus::No)
    }
  }
}
