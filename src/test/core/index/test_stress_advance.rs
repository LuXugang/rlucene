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
use crate::core::document::field::Store;
use crate::core::document::field_type::FieldType;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::postings_enum::NONE;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  at_least, get_only_leaf_reader, is_night_mode, new_directory_shared, new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)] // for quick search
pub struct TestStressAdvance;

#[test]
fn test_stress_advance() -> Result<()> {
  let num_iters = if is_night_mode() { 3 } else { 1 };
  for iter in 0..num_iters {
    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: iter={}", iter);
    }

    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let w = RandomIndexWriter::new(&mut random, dir.clone());

    let mut a_docs: HashSet<i32> = HashSet::new();
    let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
    let mut doc = Document::new();
    let mut f = new_string_field(&mut random, "field", "", Store::No, &mut field_to_type)?;
    let mut id_field = new_string_field(&mut random, "id", "", Store::Yes, &mut field_to_type)?;
    doc.add(f.clone());
    doc.add(id_field.clone());

    let num_docs = at_least(&mut random, 4097);
    if cfg!(feature = "test_log_verbose") {
      println!("\nTEST: numDocs={}", num_docs);
    }

    for id in 0..num_docs {
      if random.random_range(0..4) == 3 {
        f.set_string_value("a")?;
        a_docs.insert(id);
      } else {
        f.set_string_value("b")?;
      }
      id_field.set_string_value(id.to_string())?;
      let mut doc = Document::new();
      doc.add(f.clone());
      doc.add(id_field.clone());
      w.add_document(doc)?;
    }

    w.force_merge(1)?;

    let mut a_doc_ids: Vec<i32> = Vec::new();
    let mut b_doc_ids: Vec<i32> = Vec::new();

    let r = w.get_reader()?;
    let mut stored_fields = r.stored_fields()?;
    let max_doc = r.max_doc()?;
    for doc_id in 0..max_doc {
      let id = stored_fields
        .document(doc_id)?
        .get("id")?
        .unwrap()
        .parse::<i32>()?;
      if a_docs.contains(&id) {
        a_doc_ids.push(doc_id);
      } else {
        b_doc_ids.push(doc_id);
      }
    }

    let leaf = get_only_leaf_reader(&r)?;
    let terms = leaf.terms("field")?.expect("field terms should exist");
    let mut te = terms.iterator()?;

    let mut de = None;
    for iter2 in 0..10 {
      if cfg!(feature = "test_log_verbose") {
        println!("\nTEST: iter={} iter2={}", iter, iter2);
      }

      assert_eq!(
        SeekStatus::Found,
        te.seek_ceil(&BytesRef::from_string("a"))?
      );
      de = Some(TestUtil::docs(&mut random, &mut te, de, NONE as i32)?);
      test_one(&mut random, de.as_mut().unwrap(), &a_doc_ids)?;

      assert_eq!(
        SeekStatus::Found,
        te.seek_ceil(&BytesRef::from_string("b"))?
      );
      de = Some(TestUtil::docs(&mut random, &mut te, de, NONE as i32)?);
      test_one(&mut random, de.as_mut().unwrap(), &b_doc_ids)?;
    }

    r.close()?;
    w.close()?;
  }

  Ok(())
}

fn test_one<R, D>(random: &mut R, docs: &mut D, expected: &[i32]) -> Result<()>
where
  R: Rng + ?Sized,
  D: DocIdSetIterator,
{
  let mut upto: i32 = -1;
  while upto < expected.len() as i32 {
    let doc_id = if random.random_range(0..4) == 1 || upto == expected.len() as i32 - 1 {
      upto += 1;
      docs.next_doc()?
    } else {
      let inc = TestUtil::next_int(random, 1, expected.len() as i32 - 1 - upto);
      upto += inc;
      docs.advance(expected[upto as usize])?
    };

    if upto == expected.len() as i32 {
      assert_eq!(NO_MORE_DOCS, doc_id);
    } else {
      assert_ne!(NO_MORE_DOCS, doc_id);
      assert_eq!(expected[upto as usize], doc_id);
    }
  }
  Ok(())
}
