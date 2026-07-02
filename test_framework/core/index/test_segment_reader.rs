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
use crate::core::index::index_options::IndexOptions;
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::multi_reader::MultiReader;
use crate::test_framework::core::index::doc_helper::{DATA, FIELDS};

#[allow(dead_code)] // for quick search
struct TestSegmentReader;

pub(crate) fn check_norms<LR>(reader: LR) -> crate::core::util::error::lucene_error::Result<()>
where
  LR: LeafReader + Clone,
{
  let multi_readers = MultiReader::with_leaf_reader(vec![reader.clone()])?;
  for f in FIELDS.iter() {
    if *f.field_type().index_options() != IndexOptions::None {
      let field_name = f.name();
      let norms_opt = reader.get_norm_values(field_name)?;
      assert_eq!(norms_opt.is_some(), !f.field_type().omit_norms());
      assert_eq!(norms_opt.is_some(), !DATA.no_norms.contains_key(field_name));
      if norms_opt.is_none() {
        let norms2 = MultiDocValues::get_norm_values(&multi_readers, field_name)?;
        assert!(norms2.is_none());
      }
    }
  }
  Ok(())
}
