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
use crate::core::document::document::Document;
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::index::directory_reader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config_with_analyzer,
  new_log_merge_policy_with_merge_factor, random,
};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestOmitNorms;

#[test]
fn test_mixed_merge_throws_error() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(3);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 2)?);
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut d = Document::new();

  let mut field_type1 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  field_type1.set_omit_norms(false)?;
  field_type1.set_store_term_vectors(false)?;
  let f1 = Field::new("f1", "This field has norms", field_type1.clone());
  d.add(f1);

  let mut field_type2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  field_type2.set_omit_norms(true)?;
  field_type2.set_store_term_vectors(false)?;
  let f2 = Field::new(
    "f2",
    "This field has NO norms in all docs",
    field_type2.clone(),
  );
  d.add(f2);

  for _ in 0..30 {
    writer.add_document(d.clone())?;
  }

  let mut d2 = Document::new();
  d2.add(Field::new(
    "f1",
    "This field has NO norms",
    field_type2.clone(),
  ));
  d2.add(Field::new(
    "f2",
    "This field has norms",
    field_type1.clone(),
  ));

  let err = writer.add_document(d2).unwrap_err();
  match err {
    LuceneError::IllegalArgument(msg) => {
      assert_eq!(
        "cannot change field \"f1\" from omitNorms=false to inconsistent omitNorms=true",
        msg.to_string()
      );
    },
    _ => unreachable!("expected IllegalArgument error"),
  }

  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(ram.clone())?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;

  assert!(
    !fi
      .field_info_by_name("f1")
      .ok_or_else(|| LuceneError::illegal_state("field f1 not found"))?
      .omits_norms()
  );
  assert!(
    fi.field_info_by_name("f2")
      .ok_or_else(|| LuceneError::illegal_state("field f2 not found"))?
      .omits_norms()
  );

  Ok(())
}
#[test]
fn test_mixed_ram() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(10);
  iwc.set_merge_policy(new_log_merge_policy_with_merge_factor(&mut random, 2)?);
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut d = Document::new();

  let f1 = Field::new(
    "f1",
    "This field has norms",
    crate::core::document::text_field::TYPE_NOT_STORED.clone(),
  );
  d.add(f1);

  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_omit_norms(true)?;
  let f2 = Field::new("f2", "This field has NO norms in all docs", custom_type);
  d.add(f2);

  for _ in 0..5 {
    writer.add_document(d.clone())?;
  }

  for _ in 0..20 {
    writer.add_document(d.clone())?;
  }

  writer.force_merge(1)?;
  writer.close()?;

  let reader = directory_reader::open(ram.clone())?;
  let leaf = get_only_leaf_reader(&reader)?;
  let fi = leaf.get_field_infos()?;

  assert!(
    !fi
      .field_info_by_name("f1")
      .ok_or_else(|| LuceneError::illegal_state("field f1 not found"))?
      .omits_norms()
  );
  assert!(
    fi.field_info_by_name("f2")
      .ok_or_else(|| LuceneError::illegal_state("field f2 not found"))?
      .omits_norms()
  );

  Ok(())
}
fn assert_no_nrm<D>(dir: Arc<D>) -> Result<()>
where
  D: Directory,
{
  let files = dir.list_all()?;
  for file in files {
    assert!(!file.ends_with(".nrm") && !file.ends_with(".len"));
  }
  Ok(())
}

#[test]
fn test_no_nrm_file() -> Result<()> {
  let mut random = random();
  let ram = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
  iwc.set_max_buffered_docs(3);
  let mut mp = new_log_merge_policy_with_merge_factor(&mut random, 2)?;
  mp.get_base_mut().set_no_cfs_ratio(0.0)?;
  iwc.set_merge_policy(mp);
  let writer = IndexWriter::new(ram.clone(), iwc)?;

  let mut d = Document::new();

  let mut custom_type = FieldType::from_ref(&*crate::core::document::text_field::TYPE_NOT_STORED)?;
  custom_type.set_omit_norms(true)?;
  let f1 = Field::new("f1", "This field has no norms", custom_type);
  d.add(f1);

  for _ in 0..30 {
    writer.add_document(d.clone())?;
  }

  writer.commit()?;

  assert_no_nrm(ram.clone())?;

  writer.force_merge(1)?;
  writer.close()?;

  assert_no_nrm(ram.clone())?;

  Ok(())
}
