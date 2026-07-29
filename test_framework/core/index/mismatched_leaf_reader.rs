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
use crate::core::codecs::stored_fields_writer::StoredFieldsWriter;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::stored_field_visitor::{Status, StoredFieldVisitor};
use crate::core::util::error::lucene_error::LuceneError;
use crate::core::util::error::lucene_error::Result;
use rand::Rng;
use rand::prelude::SliceRandom;
use std::sync::Arc;

pub struct MismatchedLeafReader;

pub struct MismatchedVisitor<'a, V> {
  visitor: &'a mut V,
  shuffled: Arc<FieldInfos>,
}

impl<'a, V> MismatchedVisitor<'a, V> {
  pub fn new(visitor: &'a mut V, shuffled: Arc<FieldInfos>) -> Self {
    Self { visitor, shuffled }
  }

  fn renumber(&self, field_info: Arc<FieldInfo>) -> Result<Arc<FieldInfo>> {
    self
      .shuffled
      .field_info_by_name(&field_info.name)?
      .ok_or_else(|| {
        LuceneError::illegal_state(format!(
          "missing shuffled field info for {}",
          field_info.name
        ))
      })
  }
}

impl<V> StoredFieldVisitor for MismatchedVisitor<'_, V>
where
  V: StoredFieldVisitor,
{
  fn binary_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: Vec<u8>,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .binary_field(self.renumber(field_info)?, value, writer)
  }

  fn string_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: String,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .string_field(self.renumber(field_info)?, value, writer)
  }

  fn int_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: i32,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .int_field(self.renumber(field_info)?, value, writer)
  }

  fn long_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: i64,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .long_field(self.renumber(field_info)?, value, writer)
  }

  fn float_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: f32,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .float_field(self.renumber(field_info)?, value, writer)
  }

  fn double_field<S>(
    &mut self,
    field_info: Arc<FieldInfo>,
    value: f64,
    writer: Option<&mut S>,
  ) -> Result<()>
  where
    S: StoredFieldsWriter,
  {
    self
      .visitor
      .double_field(self.renumber(field_info)?, value, writer)
  }

  fn needs_field<S>(&mut self, field_info: Arc<FieldInfo>, writer: Option<&mut S>) -> Result<Status>
  where
    S: StoredFieldsWriter,
  {
    self.visitor.needs_field(self.renumber(field_info)?, writer)
  }
}

pub fn shuffle_infos<R>(infos: &FieldInfos, random: &mut R) -> Result<FieldInfos>
where
  R: Rng + ?Sized,
{
  let mut shuffled: Vec<Arc<FieldInfo>> = infos.iter().cloned().collect();
  shuffled.shuffle(random);

  let mut new_infos = Vec::with_capacity(shuffled.len());
  for (i, old_info) in shuffled.into_iter().enumerate() {
    let new_info = Arc::new(clone_field_info(old_info.as_ref(), i as i32)?);
    new_infos.push(new_info);
  }

  FieldInfos::new(new_infos)
}

fn clone_field_info(fi: &FieldInfo, field_number: i32) -> Result<FieldInfo> {
  FieldInfo::new(
    fi.name.clone(),
    field_number,
    fi.has_term_vectors(),
    fi.omits_norms(),
    fi.has_payloads(),
    *fi.get_index_options(),
    *fi.get_doc_values_type(),
    *fi.doc_values_skip_index_type(),
    fi.get_doc_values_gen(),
    fi.attributes().lock().attributes.clone(),
    fi.get_point_dimension_count(),
    fi.get_point_index_dimension_count(),
    fi.get_point_num_bytes(),
    fi.get_vector_dimension(),
    *fi.get_vector_encoding(),
    *fi.get_vector_similarity_function(),
    fi.is_soft_deletes_field(),
    fi.is_parent_field(),
  )
}
