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
use crate::core::document::field_type::FieldType;
use crate::core::index::composite_reader::CompositeReader;
use crate::core::index::directory_reader;
use crate::core::index::field_infos::get_indexed_fields;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::term::Term;
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_string_field, new_text_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;
use std::vec;

#[allow(dead_code)] // for quick search
pub struct TestSumDocFreq;
#[test]
fn test_sum_doc_freq() -> Result<()> {
  let mut random = random();
  let num_docs = at_least(&mut random, 500);

  let dir = new_directory_shared(&mut random)?;
  let writer = RandomIndexWriter::new(&mut random, dir.clone());
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  let mut doc = Document::new();
  let mut id = new_string_field(&mut random, "id", "", No, &mut field_to_type)?;
  let mut field1 = new_text_field(&mut random, "foo", "", No, &mut field_to_type)?;
  let mut field2 = new_text_field(&mut random, "bar", "", No, &mut field_to_type)?;

  doc.add(id.clone());
  doc.add(field1.clone());
  doc.add(field2.clone());

  for i in 0..num_docs {
    id.set_string_value(i.to_string())?;

    let ch1 =
      char::from_u32(TestUtil::next_int(&mut random, 'a' as i32, 'z' as i32) as u32).unwrap();
    let ch2 =
      char::from_u32(TestUtil::next_int(&mut random, 'a' as i32, 'z' as i32) as u32).unwrap();
    field1.set_string_value(format!("{} {}", ch1, ch2))?;

    let ch1 =
      char::from_u32(TestUtil::next_int(&mut random, 'a' as i32, 'z' as i32) as u32).unwrap();
    let ch2 =
      char::from_u32(TestUtil::next_int(&mut random, 'a' as i32, 'z' as i32) as u32).unwrap();
    field2.set_string_value(format!("{} {}", ch1, ch2))?;

    writer.add_document(doc.clone())?;
  }

  {
    let ir = writer.get_reader()?;
    assert_sum_doc_freq(ir)?;
  }

  let num_deletions = at_least(&mut random, 20);
  for _ in 0..num_deletions {
    let id_val = random.random_range(0..num_docs);
    writer.delete_documents_with_terms(vec![Term::from_text("id", id_val.to_string())])?;
  }
  writer.force_merge(1)?;
  writer.close()?;

  {
    let ir = directory_reader::open(dir.clone())?;
    assert_sum_doc_freq(ir)?;
  }
  Ok(())
}

fn assert_sum_doc_freq<CR>(reader: CR) -> Result<()>
where
  CR: CompositeReader,
{
  let fields = get_indexed_fields(&reader)?;

  for field in fields {
    let Some(terms) = get_terms(&reader, &field)? else {
      continue;
    };

    let sum_doc_freq = terms.get_sum_doc_freq()?;
    if sum_doc_freq == -1 {
      continue;
    }

    let mut computed_sum_doc_freq: i64 = 0;
    let mut terms_enum = terms.iterator()?;
    while terms_enum.next()?.is_some() {
      computed_sum_doc_freq += terms_enum.doc_freq()? as i64;
    }

    assert_eq!(computed_sum_doc_freq, sum_doc_freq);
  }

  Ok(())
}
