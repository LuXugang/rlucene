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
use crate::core::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::{EMPTY, FieldNumbers, get_merged_field_infos};
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::index::vector_encoding::VectorEncoding;
use crate::core::index::vector_similarity_function::VectorSimilarityFunction;
use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer, random,
};

use crate::core::document::document::Document;
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::string_field::StringField;
use crate::core::index::directory_reader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexWriter, read_field_infos};
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestFieldInfos;
#[test]
fn test_field_infos() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut d1 = Document::new();
  for i in 0..15 {
    d1.add(StringField::from_string(
      format!("f{}", i),
      format!("v{}", i),
      Store::Yes,
    )?);
  }
  writer.add_document(d1)?;
  writer.commit()?;

  let mut d2 = Document::new();
  d2.add(StringField::from_string("f0", "v0", Store::Yes)?);
  d2.add(StringField::from_string("f15", "v15", Store::Yes)?);
  d2.add(StringField::from_string("f16", "v16", Store::Yes)?);
  writer.add_document(d2)?;
  writer.commit()?;

  let d3 = Document::new();
  writer.add_document(d3)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir.clone())?;
  assert_eq!(3, sis.size());

  let fis1 = read_field_infos(sis.info(0).unwrap())?;
  let fis2 = read_field_infos(sis.info(1).unwrap())?;
  let fis3 = read_field_infos(sis.info(2).unwrap())?;

  let iter = fis1.iter();
  for (i, fi) in iter.enumerate() {
    assert_eq!(i, fi.number as usize);
    assert_eq!(format!("f{}", i), fi.name);
    assert_eq!(
      format!("f{}", i),
      fis1.field_info_by_number(i as i32)?.unwrap().name
    ); // lookup by number
    assert_eq!(
      format!("f{}", i),
      fis1.field_info_by_name(&format!("f{}", i)).unwrap().name
    ); // lookup by name
  }

  // testing sparse FieldInfos
  assert_eq!("f0", fis2.field_info_by_number(0)?.unwrap().name);
  assert_eq!("f0", fis2.field_info_by_name("f0").unwrap().name);
  assert!(fis2.field_info_by_number(1)?.is_none());
  assert!(fis2.field_info_by_name("f1").is_none());
  assert_eq!("f15", fis2.field_info_by_number(15)?.unwrap().name);
  assert_eq!("f15", fis2.field_info_by_name("f15").unwrap().name);
  assert_eq!("f16", fis2.field_info_by_number(16)?.unwrap().name);
  assert_eq!("f16", fis2.field_info_by_name("f16").unwrap().name);

  // testing empty FieldInfos
  assert!(fis3.field_info_by_number(0)?.is_none());
  assert!(fis3.field_info_by_name("f0").is_none());
  assert_eq!(0, fis3.size());
  let mut it3 = fis3.iter();
  assert!(it3.next().is_none());

  Ok(())
}
#[test]
fn test_field_attributes() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut type1 = FieldType::new();
  type1.set_stored(true)?;
  type1.put_attribute("testKey1", "testValue1")?;

  let mut d1 = Document::new();
  d1.add(Field::new("f1", "v1", type1.clone()));
  let mut type2 = type1.clone();
  type2.put_attribute("testKey1", "testValue2")?;

  writer.add_document(d1)?;
  writer.commit()?;

  type1.put_attribute("testKey1", "testValueX")?;
  type1.put_attribute("testKey2", "testValue2")?;

  let mut d2 = Document::new();
  d2.add(Field::new("f1", "v2", type1.clone()));
  d2.add(Field::new("f2", "v2", type2.clone()));
  writer.add_document(d2)?;
  writer.commit()?;
  writer.force_merge(1)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let fis = get_merged_field_infos(reader)?;
  assert_eq!(2, fis.size());

  for fi in fis.iter() {
    match fi.name.as_str() {
      // testKey1 can point to either testValue1 or testValueX based on the order
      // of merge, but we see textValueX winning here since segment_2 is merged on segment_1.
      "f1" => {
        assert_eq!(Some("testValueX".to_string()), fi.get_attribute("testKey1"));
        assert_eq!(Some("testValue2".to_string()), fi.get_attribute("testKey2"));
      },
      "f2" => {
        assert_eq!(Some("testValue2".to_string()), fi.get_attribute("testKey1"));
      },
      _ => {
        unreachable!("Unknown field");
      },
    }
  }
  writer.close()?;
  Ok(())
}

#[test]
fn test_field_attributes_single_segment() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let mut config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  config.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut d1 = Document::new();
  let mut type1 = FieldType::new();
  type1.set_stored(true)?;
  type1.put_attribute("att1", "attdoc1")?;
  d1.add(Field::new("f1", "v1", type1.clone()));

  type1.put_attribute("att2", "attdoc1")?;
  d1.add(Field::new("f1", "v1", type1.clone()));

  writer.add_document(d1)?;

  let mut d2 = Document::new();
  type1.put_attribute("att1", "attdoc2")?;
  type1.put_attribute("att2", "attdoc2")?;
  type1.put_attribute("att3", "attdoc2")?;

  let mut type2 = FieldType::new();
  type2.set_stored(true)?;
  type2.put_attribute("att4", "attdoc2")?;

  d2.add(Field::new("f1", "v2", type1.clone()));
  d2.add(Field::new("f2", "v2", type2.clone()));
  writer.add_document(d2)?;

  writer.commit()?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let fis = get_merged_field_infos(reader)?;

  let fi1 = fis.field_info_by_name("f1").unwrap();
  assert_eq!(Some("attdoc1".to_string()), fi1.get_attribute("att1"));
  assert_eq!(Some("attdoc1".to_string()), fi1.get_attribute("att2"));
  assert_eq!(None, fi1.get_attribute("att3"));

  let fi2 = fis.field_info_by_name("f2").unwrap();
  assert_eq!(Some("attdoc2".to_string()), fi2.get_attribute("att4"));

  writer.close()?;
  Ok(())
}

#[test]
fn test_merged_field_infos_empty() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let config = new_index_writer_config(&mut random)?;
  let writer = IndexWriter::new(dir.clone(), config)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let actual = get_merged_field_infos(reader)?;

  assert!(Arc::ptr_eq(&EMPTY.clone(), &actual));
  writer.close()?;
  Ok(())
}
#[test]
fn test_merged_field_infos_single_leaf() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let mock = MockAnalyzer::new(&mut random);
  let config = new_index_writer_config_with_analyzer(&mut random, mock)?;
  let writer = IndexWriter::new(dir.clone(), config)?;

  let mut d1 = Document::new();
  d1.add(StringField::from_string("f1", "v1", Store::Yes)?);
  writer.add_document(d1)?;
  writer.commit()?;

  let mut d2 = Document::new();
  d2.add(StringField::from_string("f2", "v2", Store::Yes)?);
  writer.add_document(d2)?;
  writer.commit()?;

  writer.force_merge(1)?;

  let reader = directory_reader::open_from_writer(&writer)?;
  let actual = get_merged_field_infos(&reader)?;
  let reader = reader.get_context()?;
  let leaves = reader.leaves()?;
  let expected = leaves[0].reader().get_field_infos()?;

  assert_eq!(1, leaves.len());
  assert!(std::ptr::eq(expected.as_ref(), actual.as_ref()));

  writer.close()?;
  drop(dir);
  Ok(())
}

#[test]
fn test_field_numbers_auto_increment() -> Result<()> {
  let mut field_numbers = FieldNumbers::new(Some("softDeletes"), Some("parentDoc"))?;
  for i in 0..10 {
    let fi = FieldInfo::new(
      format!("field{}", i),
      -1,
      false,
      false,
      false,
      IndexOptions::None,
      DocValuesType::None,
      DocValuesSkipIndexType::None,
      -1,
      HashMap::new(),
      0,
      0,
      0,
      0,
      VectorEncoding::FLOAT32(4),
      VectorSimilarityFunction::Euclidean,
      false,
      false,
    )?;
    field_numbers.add_or_get(&fi)?;
  }
  let idx = field_numbers.add_or_get(&FieldInfo::new(
    "EleventhField".to_string(),
    -1,
    false,
    false,
    false,
    IndexOptions::None,
    DocValuesType::None,
    DocValuesSkipIndexType::None,
    -1,
    HashMap::new(),
    0,
    0,
    0,
    0,
    VectorEncoding::FLOAT32(4),
    VectorSimilarityFunction::Euclidean,
    false,
    false,
  )?)?;
  assert_eq!(10, idx, "Field numbers 0 through 9 were allocated");

  field_numbers.clear();
  let idx = field_numbers.add_or_get(&FieldInfo::new(
    "PostClearField".to_string(),
    -1,
    false,
    false,
    false,
    IndexOptions::None,
    DocValuesType::None,
    DocValuesSkipIndexType::None,
    -1,
    HashMap::new(),
    0,
    0,
    0,
    0,
    VectorEncoding::FLOAT32(4),
    VectorSimilarityFunction::Euclidean,
    false,
    false,
  )?)?;
  assert_eq!(0, idx, "Field numbers should reset after clear()");
  Ok(())
}
