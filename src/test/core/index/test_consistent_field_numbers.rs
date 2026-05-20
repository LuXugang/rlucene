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
use crate::core::document::field::Store::No;
use crate::core::document::field::{Field, Store};
use crate::core::document::field_type::FieldType;
use crate::core::document::stored_field::StoredField;
use crate::core::document::text_field::{TextField, text_field_type};
use crate::core::index::index_writer::{IndexWriter, read_field_infos};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::indexable_field_type::IndexableFieldType;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::log_merge_policy::LogMergePolicy;
use crate::core::index::no_merge_policy::NoMergePolicy;
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, new_directory_shared, new_index_writer_config, new_index_writer_config_with_analyzer,
  random,
};
use rand::RngExt;

#[allow(dead_code)] // for quick search
struct TestConsistentFieldNumbers;
#[test]
fn test_same_field_numbers_across_segments() -> Result<()> {
  let mut random = random();
  for i in 0..2 {
    let dir = new_directory_shared(&mut random)?;

    {
      let writer_opt = {
        let mock = MockAnalyzer::new(&mut random);
        let mut conf = new_index_writer_config_with_analyzer(&mut random, mock);
        conf.set_merge_policy(NoMergePolicy::default());
        let writer = IndexWriter::new(dir.clone(), conf)?;

        let mut d1 = Document::new();
        d1.add(TextField::from_string("f1", "first field", Store::Yes)?);
        d1.add(TextField::from_string("f2", "second field", Store::Yes)?);
        writer.add_document(d1)?;

        if i == 1 {
          writer.close()?;
          None
        } else {
          writer.commit()?;
          Some(writer)
        }
      };
      let writer = match writer_opt {
        Some(writer) => writer,
        None => {
          let mut conf = new_index_writer_config(&mut random);
          conf.set_merge_policy(NoMergePolicy::default());
          IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?
        },
      };

      let mut d2 = Document::new();
      d2.add(TextField::from_string("f2", "second field", Store::No)?);
      d2.add(TextField::from_string("f1", "first field", Store::Yes)?);
      d2.add(TextField::from_string("f3", "third field", Store::No)?);
      d2.add(TextField::from_string("f4", "fourth field", Store::No)?);
      writer.add_document(d2)?;

      writer.close()?;

      let sis = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(2, sis.size());

      let fis1 = read_field_infos(sis.info(0).unwrap())?;
      let fis2 = read_field_infos(sis.info(1).unwrap())?;

      assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);
      assert_eq!("f1", fis2.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis2.field_info_by_number(1)?.unwrap().name);
      assert_eq!("f3", fis2.field_info_by_number(2)?.unwrap().name);
      assert_eq!("f4", fis2.field_info_by_number(3)?.unwrap().name);
    }

    let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;
    writer.force_merge(1)?;
    writer.close()?;

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    assert_eq!(1, sis.size());

    let fis3 = read_field_infos(sis.info(0).unwrap())?;

    assert_eq!("f1", fis3.field_info_by_number(0)?.unwrap().name);
    assert_eq!("f2", fis3.field_info_by_number(1)?.unwrap().name);
    assert_eq!("f3", fis3.field_info_by_number(2)?.unwrap().name);
    assert_eq!("f4", fis3.field_info_by_number(3)?.unwrap().name);
  }

  Ok(())
}
#[test]
fn test_add_indexes() -> Result<()> {
  let mut random = random();

  let dir1 = new_directory_shared(&mut random)?;
  let dir2 = new_directory_shared(&mut random)?;

  let a = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, a);
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir1.clone(), iwc)?;

  let mut d1 = Document::new();
  d1.add(TextField::from_string("f1", "first field", Store::Yes)?);
  d1.add(TextField::from_string("f2", "second field", Store::Yes)?);
  writer.add_document(d1)?;
  writer.close()?;
  drop(writer);

  let a = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, a);
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir2.clone(), iwc)?;

  let mut d2 = Document::new();
  d2.add(TextField::from_string("f2", "second field", Store::Yes)?);
  d2.add(TextField::from_string("f1", "first field", Store::Yes)?);
  d2.add(TextField::from_string("f3", "third field", Store::Yes)?);
  d2.add(TextField::from_string("f4", "fourth field", Store::Yes)?);
  writer.add_document(d2)?;
  writer.close()?;
  drop(writer);

  let a = MockAnalyzer::new(&mut random);
  let mut iwc = new_index_writer_config_with_analyzer(&mut random, a);
  iwc.set_merge_policy(NoMergePolicy::default());
  let writer = IndexWriter::new(dir1.clone(), iwc)?;
  writer.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
  writer.close()?;
  drop(writer);

  let sis = SegmentInfos::read_latest_commit(dir1.clone())?;
  assert_eq!(2, sis.size());

  let fis1 = read_field_infos(sis.info(0).as_ref().unwrap())?;
  let fis2 = read_field_infos(sis.info(1).as_ref().unwrap())?;

  assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
  assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);

  assert_eq!("f2", fis2.field_info_by_number(0)?.unwrap().name);
  assert_eq!("f1", fis2.field_info_by_number(1)?.unwrap().name);
  assert_eq!("f3", fis2.field_info_by_number(2)?.unwrap().name);
  assert_eq!("f4", fis2.field_info_by_number(3)?.unwrap().name);

  Ok(())
}
#[test]
fn test_field_number_gaps() -> Result<()> {
  let mut random = random();
  let num_iters = at_least(&mut random, 13);
  for _ in 0..num_iters {
    let dir = new_directory_shared(&mut random)?;
    {
      let a = MockAnalyzer::new(&mut random);
      let mut config = new_index_writer_config_with_analyzer(&mut random, a);
      config.set_merge_policy(NoMergePolicy::default());
      let writer = IndexWriter::new(dir.clone(), config)?;

      let mut d = Document::new();
      d.add(TextField::from_string("f1", "d1 first field", Store::Yes)?);
      d.add(TextField::from_string("f2", "d1 second field", Store::Yes)?);
      writer.add_document(d)?;
      writer.close()?;

      let sis = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(1, sis.size());
      let fis1 = read_field_infos(sis.info(0).unwrap())?;
      assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);
    }

    {
      let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

      let mut d = Document::new();
      d.add(TextField::from_string("f1", "d2 first field", Store::Yes)?);
      d.add(StoredField::from_binary("f3", vec![1, 2, 3])?);
      writer.add_document(d)?;
      writer.close()?;

      let sis = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(2, sis.size());
      let fis1 = read_field_infos(sis.info(0).unwrap())?;
      let fis2 = read_field_infos(sis.info(1).unwrap())?;
      assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);
      assert_eq!("f1", fis2.field_info_by_number(0)?.unwrap().name);
      assert!(fis2.field_info_by_number(1)?.is_none());
      assert_eq!("f3", fis2.field_info_by_number(2)?.unwrap().name);
    }

    {
      let writer = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

      let mut d = Document::new();
      d.add(TextField::from_string("f1", "d3 first field", Store::Yes)?);
      d.add(TextField::from_string("f2", "d3 second field", Store::Yes)?);
      d.add(StoredField::from_binary("f3", vec![1, 2, 3, 4, 5])?);
      writer.add_document(d)?;
      writer.close()?;

      let sis = SegmentInfos::read_latest_commit(dir.clone())?;
      assert_eq!(3, sis.size());
      let fis1 = read_field_infos(sis.info(0).unwrap())?;
      let fis2 = read_field_infos(sis.info(1).unwrap())?;
      let fis3 = read_field_infos(sis.info(2).unwrap())?;
      assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);
      assert_eq!("f1", fis2.field_info_by_number(0)?.unwrap().name);
      assert!(fis2.field_info_by_number(1)?.is_none());
      assert_eq!("f3", fis2.field_info_by_number(2)?.unwrap().name);
      assert_eq!("f1", fis3.field_info_by_number(0)?.unwrap().name);
      assert_eq!("f2", fis3.field_info_by_number(1)?.unwrap().name);
      assert_eq!("f3", fis3.field_info_by_number(2)?.unwrap().name);
    }

    {
      let a = MockAnalyzer::new(&mut random);
      let mut config = new_index_writer_config_with_analyzer(&mut random, a);
      config.set_merge_policy(NoMergePolicy::default());
      let writer = IndexWriter::new(dir.clone(), config)?;

      writer.delete_documents_with_terms(vec![Term::from_text("f1", "d1")])?;
      // nuke the first segment entirely so that the segment with gaps is
      // loaded first!
      writer.force_merge_deletes()?;
      writer.close()?;
    }

    {
      let a = MockAnalyzer::new(&mut random);
      let mut config = new_index_writer_config_with_analyzer(&mut random, a);
      config.set_merge_policy(LogMergePolicy::log_bytes_size());
      let writer = IndexWriter::new(dir.clone(), config)?;

      writer.force_merge(1)?;
      writer.close()?;
    }

    let sis = SegmentInfos::read_latest_commit(dir.clone())?;
    assert_eq!(1, sis.size());
    let fis1 = read_field_infos(sis.info(0).unwrap())?;
    assert_eq!("f1", fis1.field_info_by_number(0)?.unwrap().name);
    assert_eq!("f2", fis1.field_info_by_number(1)?.unwrap().name);
    assert_eq!("f3", fis1.field_info_by_number(2)?.unwrap().name);
  }

  Ok(())
}
#[test]
fn test_many_fields() -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, 200);
  let max_fields = at_least(&mut random, 50);

  let mut docs = Vec::with_capacity(num_docs as usize);
  for _ in 0..num_docs {
    let mut doc_fields = Vec::with_capacity(4);
    for _ in 0..4 {
      doc_fields.push(random.random_range(0..max_fields));
    }
    docs.push(doc_fields);
  }

  let dir = new_directory_shared(&mut random)?;
  let a = MockAnalyzer::new(&mut random);
  let writer = IndexWriter::new(
    dir.clone(),
    new_index_writer_config_with_analyzer(&mut random, a),
  )?;

  for doc_fields in &docs {
    let mut d = Document::new();
    for &field_num in doc_fields {
      if field_num % 16 == 1 {
        d.add(get_text_field(field_num)?);
      } else {
        d.add(get_field(field_num)?);
      }
    }
    writer.add_document(d)?;
  }

  writer.force_merge(1)?;
  writer.close()?;

  let sis = SegmentInfos::read_latest_commit(dir.clone())?;
  for si in sis.iter() {
    let fis = read_field_infos(si)?;

    for fi in fis.iter() {
      let field_num = fi.name.parse::<i32>()?;
      if field_num % 16 == 1 {
        let expected = get_text_field(field_num)?;
        assert_eq!(
          expected.field_type().index_options(),
          fi.get_index_options()
        );
        assert_eq!(
          expected.field_type().store_term_vectors(),
          fi.has_term_vectors()
        );
      } else {
        let expected = get_field(field_num)?;
        assert_eq!(
          expected.field_type().index_options(),
          fi.get_index_options()
        );
        assert_eq!(
          expected.field_type().store_term_vectors(),
          fi.has_term_vectors()
        );
      }
    }
  }
  Ok(())
}

fn get_text_field(number: i32) -> Result<TextField> {
  let mode = number % 16;
  assert_eq!(mode, 1);
  let field_name = number.to_string();
  let text = "some text".to_string();
  TextField::from_string(field_name, text, No)
}

fn get_field(number: i32) -> Result<Field> {
  let mode = number % 16;
  let field_name = number.to_string();
  let text = "some text".to_string();

  let custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;

  let mut custom_type2 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type2.set_tokenized(false)?;

  let mut custom_type3 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type3.set_tokenized(false)?;

  let mut custom_type4 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type4.set_tokenized(false)?;
  custom_type4.set_store_term_vectors(true)?;
  custom_type4.set_store_term_vector_offsets(true)?;

  let mut custom_type5 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type5.set_store_term_vectors(true)?;
  custom_type5.set_store_term_vector_offsets(true)?;

  let mut custom_type6 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type6.set_tokenized(false)?;
  custom_type6.set_store_term_vectors(true)?;
  custom_type6.set_store_term_vector_offsets(true)?;

  let mut custom_type7 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type7.set_tokenized(false)?;
  custom_type7.set_store_term_vectors(true)?;
  custom_type7.set_store_term_vector_offsets(true)?;

  let mut custom_type8 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type8.set_tokenized(false)?;
  custom_type8.set_store_term_vectors(true)?;
  custom_type8.set_store_term_vector_positions(true)?;

  let mut custom_type9 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type9.set_store_term_vectors(true)?;
  custom_type9.set_store_term_vector_positions(true)?;

  let mut custom_type10 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type10.set_tokenized(false)?;
  custom_type10.set_store_term_vectors(true)?;
  custom_type10.set_store_term_vector_positions(true)?;

  let mut custom_type11 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type11.set_tokenized(false)?;
  custom_type11.set_store_term_vectors(true)?;
  custom_type11.set_store_term_vector_positions(true)?;

  let mut custom_type12 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type12.set_store_term_vectors(true)?;
  custom_type12.set_store_term_vector_offsets(true)?;
  custom_type12.set_store_term_vector_positions(true)?;

  let mut custom_type13 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type13.set_store_term_vectors(true)?;
  custom_type13.set_store_term_vector_offsets(true)?;
  custom_type13.set_store_term_vector_positions(true)?;

  let mut custom_type14 = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
  custom_type14.set_tokenized(false)?;
  custom_type14.set_store_term_vectors(true)?;
  custom_type14.set_store_term_vector_offsets(true)?;
  custom_type14.set_store_term_vector_positions(true)?;

  let mut custom_type15 = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
  custom_type15.set_tokenized(false)?;
  custom_type15.set_store_term_vectors(true)?;
  custom_type15.set_store_term_vector_offsets(true)?;
  custom_type15.set_store_term_vector_positions(true)?;

  let field = match mode {
    0 => Field::new(field_name, text, custom_type),
    1 => unreachable!(""),
    2 => Field::new(field_name, text, custom_type2),
    3 => Field::new(field_name, text, custom_type3),
    4 => Field::new(field_name, text, custom_type4),
    5 => Field::new(field_name, text, custom_type5),
    6 => Field::new(field_name, text, custom_type6),
    7 => Field::new(field_name, text, custom_type7),
    8 => Field::new(field_name, text, custom_type8),
    9 => Field::new(field_name, text, custom_type9),
    10 => Field::new(field_name, text, custom_type10),
    11 => Field::new(field_name, text, custom_type11),
    12 => Field::new(field_name, text, custom_type12),
    13 => Field::new(field_name, text, custom_type13),
    14 => Field::new(field_name, text, custom_type14),
    15 => Field::new(field_name, text, custom_type15),
    _ => unreachable!(),
  };

  Ok(field)
}
