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
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config, random,
};
use rand::Rng;

#[allow(dead_code)] // for quick search
struct TestIndexOptions;
#[test]
fn test_change_index_options_via_add_document() -> Result<()> {
  for from in IndexOptions::values() {
    for to in IndexOptions::values() {
      do_test_change_index_options_via_add_document(from, to)?;
    }
  }
  Ok(())
}

fn do_test_change_index_options_via_add_document(
  from: IndexOptions,
  to: IndexOptions,
) -> Result<()> {
  let mut random = random();

  let dir = new_directory_shared(&mut random)?;
  let iwc = new_index_writer_config(&mut random);
  let w = IndexWriter::new(dir.clone(), iwc)?;

  let mut ft1 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  ft1.set_index_options(from)?;
  let mut doc1 = Document::new();
  doc1.add(Field::new("foo", "bar", ft1));
  w.add_document(doc1)?;

  let mut ft2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  ft2.set_index_options(to)?;
  let mut doc2 = Document::new();
  doc2.add(Field::new("foo", "bar", ft2));

  if from == to {
    w.add_document(doc2)?;
  } else {
    let res = w.add_document(doc2);
    let expected = format!(
      "Inconsistency of field data structures across documents for field [foo] of doc [1]. \
           index options: expected '{}', but it has '{}'.",
      from, to
    );

    match res {
      Err(LuceneError::IllegalArgument(msg)) => {
        assert_eq!(expected, msg.message);
      },
      other => {
        debug_assert!(false, "Unexpected error type: {:?}", other);
      },
    }
  }

  w.close()?;
  Ok(())
}
#[test]
fn test_change_index_options_via_add_indexes_codec_reader() -> Result<()> {
  for from in IndexOptions::values() {
    for to in IndexOptions::values() {
      do_test_change_index_options_add_indexes_codec_reader(from, to)?;
    }
  }
  Ok(())
}
fn do_test_change_index_options_add_indexes_codec_reader(
  _from: IndexOptions,
  _to: IndexOptions,
) -> Result<()> {
  // TODO IMPORTANT add_indexes_from_codec_readers未实现
  Ok(())
}
#[test]
fn test_change_index_options_via_add_indexes_directory() -> Result<()> {
  let mut random = random();
  for from in IndexOptions::values() {
    for to in IndexOptions::values() {
      do_test_change_index_options_add_indexes_directory(&mut random, from, to)?;
    }
  }
  Ok(())
}
fn do_test_change_index_options_add_indexes_directory<R>(
  random: &mut R,
  from: IndexOptions,
  to: IndexOptions,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir1 = new_directory_shared(random)?;
  let w1 = IndexWriter::new(dir1.clone(), new_index_writer_config(random))?;

  let mut ft1 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  ft1.set_index_options(from)?;
  let mut doc1 = Document::new();
  doc1.add(Field::new("foo", "bar", ft1));
  w1.add_document(doc1)?;

  let dir2 = new_directory_shared(random)?;
  let w2 = IndexWriter::new(dir2.clone(), new_index_writer_config(random))?;

  let mut ft2 = FieldType::from_ref(&*crate::core::document::text_field::TYPE_STORED)?;
  ft2.set_index_options(to)?;
  let mut doc2 = Document::new();
  doc2.add(Field::new("foo", "bar", ft2));
  w2.add_document(doc2)?;
  w2.close()?;
  drop(w2);

  if from == to {
    w1.add_indexes_from_dir(std::slice::from_ref(&dir2))?;
    w1.force_merge(1)?;
    let reader = directory_reader::open_from_writer(&w1)?;
    let leaf = get_only_leaf_reader(&reader)?;
    let expected = if from == IndexOptions::None { to } else { from };
    assert_eq!(
      expected,
      *leaf
        .get_field_infos()?
        .field_info_by_name("foo")
        .unwrap()
        .get_index_options()
    );
    reader.close()?;
  } else {
    let err = w1.add_indexes_from_dir(std::slice::from_ref(&dir2));
    assert!(matches!(err, Err(LuceneError::IllegalArgument(_))));
    assert_eq!(
      format!(
        "cannot change field \"foo\" from index options={} to inconsistent index options={}",
        from, to
      ),
      err.unwrap_err().to_string()
    );
  }

  w1.close()?;
  Ok(())
}
