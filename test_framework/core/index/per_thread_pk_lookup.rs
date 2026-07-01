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
use crate::core::index::index_reader::{CacheHelper, CacheKey};
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{LRBits, LRPosting, LRTermsEnum, LeafReader};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::core::util::TryIntoInt;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use std::collections::HashMap;

pub struct PerThreadPKLookup<LR>
where
  LR: LeafReader,
{
  id_field_name: String,
  pub(crate) terms_enums: Vec<LRTermsEnum<LR>>,
  pub(crate) postings_enums: Vec<Option<LRPosting<LR>>>,
  pub(crate) live_docs: Vec<Option<LRBits<LR>>>,
  pub(crate) doc_bases: Vec<i32>,
  pub(crate) has_deletions: bool,
  enum_indexes: HashMap<CacheKey, usize>,
}

impl<LR> PerThreadPKLookup<LR>
where
  LR: LeafReader,
{
  pub fn new<IRC>(reader_context: &IRC, id_field_name: &str) -> Result<Self>
  where
    IRC: IndexReaderContext<LeafReader = LR>,
  {
    Self::new_with_reuse(
      reader_context,
      id_field_name,
      HashMap::new(),
      Vec::new(),
      Vec::new(),
    )
  }

  fn new_with_reuse<IRC>(
    reader_context: &IRC,
    id_field_name: &str,
    prev_enum_indexes: HashMap<CacheKey, usize>,
    mut reusable_terms_enums: Vec<Option<LRTermsEnum<LR>>>,
    mut reusable_postings_enums: Vec<Option<LRPosting<LR>>>,
  ) -> Result<Self>
  where
    IRC: IndexReaderContext<LeafReader = LR>,
  {
    let mut leaves = reader_context
      .leaves()?
      .iter()
      .map(|context| Ok((context.reader().num_docs()?, context)))
      .collect::<Result<Vec<_>>>()?;
    // Larger segments are more likely to have the id, so sort largest to smallest by numDocs.
    leaves.sort_by(|(num_docs1, _), (num_docs2, _)| num_docs2.cmp(num_docs1));

    let mut terms_enums = Vec::new();
    let mut postings_enums = Vec::new();
    let mut live_docs = Vec::new();
    let mut doc_bases = Vec::new();
    let mut enum_indexes = HashMap::new();
    let mut has_deletions = false;

    for (_, context) in leaves {
      let leaf_reader = context.reader();
      let cache_key = leaf_reader
        .get_core_cache_helper()?
        .map(|cache_helper| cache_helper.get_key());

      let (terms_enum, postings_enum) = if let Some(seg) = cache_key
        .as_ref()
        .and_then(|key| prev_enum_indexes.get(key))
      {
        (
          reusable_terms_enums.get_mut(*seg).and_then(Option::take),
          reusable_postings_enums.get_mut(*seg).and_then(Option::take),
        )
      } else if let Some(terms) = leaf_reader.terms(id_field_name)? {
        (Some(terms.iterator()?), None)
      } else {
        (None, None)
      };

      if let Some(terms_enum) = terms_enum {
        if let Some(cache_key) = cache_key {
          enum_indexes.insert(cache_key, terms_enums.len());
        }

        doc_bases.push(context.doc_base.try_convert()?);
        live_docs.push(leaf_reader.get_live_docs()?);
        has_deletions |= leaf_reader.has_deletions()?;
        terms_enums.push(terms_enum);
        postings_enums.push(postings_enum);
      }
    }

    Ok(Self {
      id_field_name: id_field_name.to_string(),
      terms_enums,
      postings_enums,
      live_docs,
      doc_bases,
      has_deletions,
      enum_indexes,
    })
  }

  /** Returns docID if found, else -1. */
  pub fn lookup(&mut self, id: &BytesRef<Vec<u8>>) -> Result<i32> {
    for seg in 0..self.terms_enums.len() {
      if self.terms_enums[seg].seek_exact(id)? {
        self.postings_enums[seg] =
          Some(self.terms_enums[seg].postings_with_flags(self.postings_enums[seg].take(), 0)?);

        let postings_enum = self.postings_enums[seg].as_mut().unwrap();
        loop {
          let doc_id = postings_enum.next_doc()?;
          if doc_id == NO_MORE_DOCS {
            break;
          }
          if self.live_docs[seg]
            .as_ref()
            .map_or(Ok(true), |live_docs| live_docs.get(doc_id as usize))?
          {
            return Ok(self.doc_bases[seg] + doc_id);
          }
        }
        debug_assert!(self.has_deletions);
      }
    }

    Ok(-1)
  }

  /** Reuse previous PerThreadPKLookup's termsEnum and postingsEnum. */
  pub fn reopen<IRC>(self, reader_context: Option<&IRC>) -> Result<Option<Self>>
  where
    IRC: IndexReaderContext<LeafReader = LR>,
  {
    let Some(reader_context) = reader_context else {
      return Ok(None);
    };

    let reusable_terms_enums = self.terms_enums.into_iter().map(Some).collect();
    Ok(Some(Self::new_with_reuse(
      reader_context,
      &self.id_field_name,
      self.enum_indexes,
      reusable_terms_enums,
      self.postings_enums,
    )?))
  }
}
