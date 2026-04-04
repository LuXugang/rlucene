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
use crate::core::codecs::knn_field_vectors_writer::VectorValueEnum;
use crate::core::index::docs_with_field_set::DocsWithFieldSet;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::sorter::DocMap;
use crate::core::search::doc_id_set::DocIdSet;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::accountable::Accountable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::HashMap;
use std::sync::Arc;

pub trait KnnVectorsWriter: Accountable {
  /// Adds a new field for indexing.
  fn add_field(&mut self, _field_info: Arc<FieldInfo>) -> Result<usize> {
    Err(LuceneError::unsupported_operation(""))
  }

  /// Flushes all buffered data on disk.
  fn flush<DM>(&mut self, _max_doc: i32, _sort_map: Option<&DM>) -> Result<()>
  where
    DM: DocMap,
  {
    Err(LuceneError::unsupported_operation(""))
  }

  fn finish(&mut self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
  fn add_value(
    &mut self,
    _doc_id: i32,
    _vector_value: &VectorValueEnum,
    _field_vectors_writers_idx: usize,
  ) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
}

/// Given old doc ids and an id mapping, maps old ordinal to new ordinal. Note: this method return
/// nothing and output are written to parameters
///
/// # Arguments
/// * `old_doc_ids` - the old or current document ordinals. Must not be null.
/// * `sort_map` - the document sorting map for how to make the new ordinals. Must not be null.
/// * `old2new_ord` - maps from old ord to new ord
/// * `new2old_ord` - maps from new ord to old ord
/// * `new_docs_with_field` - set of new doc ids which has the value
pub fn map_old_ord_to_new_ord<DM>(
  old_doc_ids: &DocsWithFieldSet,
  sort_map: &DM,
  mut old2new_ord: Option<&mut [usize]>,
  mut new2old_ord: Option<&mut [usize]>,
  mut new_docs_with_field: Option<&mut DocsWithFieldSet>,
) -> Result<()>
where
  DM: DocMap,
{
  debug_assert!(old2new_ord.is_some() || new2old_ord.is_some() || new_docs_with_field.is_some());

  debug_assert!({
    if let Some(ref arr) = old2new_ord {
      arr.len() == old_doc_ids.cardinality() as usize
    } else {
      true
    }
  });
  debug_assert!({
    if let Some(ref arr) = new2old_ord {
      arr.len() == old_doc_ids.cardinality() as usize
    } else {
      true
    }
  });

  let mut new_id_to_old_ord = HashMap::new();

  let mut iterator = old_doc_ids.iterator()?;
  let mut new_doc_ids = vec![0; old_doc_ids.cardinality() as usize];

  let mut old_ord = 0;

  let mut old_doc_id = iterator.next_doc()?;
  while old_doc_id != NO_MORE_DOCS {
    let new_id = sort_map.old_to_new(old_doc_id)? as usize;
    new_id_to_old_ord.insert(new_id, old_ord);
    new_doc_ids[old_ord] = new_id;
    old_ord += 1;

    old_doc_id = iterator.next_doc()?;
  }

  new_doc_ids.sort();

  for (new_ord, &new_doc_id) in new_doc_ids.iter().enumerate() {
    let curr_old_ord = *new_id_to_old_ord
      .get(&new_doc_id)
      .ok_or_else(|| LuceneError::illegal_state("missing mapping for new_doc_id"))?;

    if let Some(arr) = old2new_ord.as_mut() {
      arr[curr_old_ord] = new_ord;
    }

    if let Some(arr) = new2old_ord.as_mut() {
      arr[new_ord] = curr_old_ord;
    }

    if let Some(set) = new_docs_with_field.as_mut() {
      set.add(new_doc_id as i32)?;
    }
  }

  Ok(())
}
