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
use crate::core::index::field_infos::get_indexed_fields;
use crate::core::index::multi_terms::get_terms;
use crate::core::index::postings_enum::NONE;
use crate::core::index::terms::Terms;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::NO_MORE_DOCS;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::lucene_test_case::{
  at_least, new_directory_shared, new_string_field, random,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;

/// Tests the Terms.docCount statistic
#[allow(dead_code)] // for quick search
pub struct TestDocCount;
#[test]
fn test_simple() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let iw = RandomIndexWriter::new(&mut random, dir.clone());

  let num_docs = at_least(&mut random, 100);
  let mut field_to_type: HashMap<String, FieldType> = HashMap::new();
  for _ in 0..num_docs {
    iw.add_document(doc(&mut random, &mut field_to_type)?)?;
  }

  {
    let ir = iw.get_reader()?;
    verify_count(&ir, &mut random)?;
  }
  iw.force_merge(1)?;

  {
    let ir = iw.get_reader()?;
    verify_count(&ir, &mut random)?;
  }

  iw.close()?;
  Ok(())
}
fn doc<R>(random: &mut R, field_to_type: &mut HashMap<String, FieldType>) -> Result<Document>
where
  R: Rng + ?Sized,
{
  let mut doc = Document::new();
  let num_fields = TestUtil::next_int(random, 1, 10);
  for _ in 0..num_fields {
    let field_name = char::from_u32(TestUtil::next_int(random, 'a' as i32, 'z' as i32) as u32)
      .unwrap()
      .to_string();
    let field_value = char::from_u32(TestUtil::next_int(random, 'a' as i32, 'z' as i32) as u32)
      .unwrap()
      .to_string();

    doc.add(new_string_field(
      random,
      &field_name,
      &field_value,
      No,
      field_to_type,
    )?);
  }
  Ok(doc)
}

fn verify_count<CR, R>(reader: &CR, random: &mut R) -> Result<()>
where
  R: Rng + ?Sized,
  CR: CompositeReader,
{
  let max_doc = reader.max_doc()?;
  let fields = get_indexed_fields(reader)?;

  for field in fields {
    let Some(terms) = get_terms(reader, &field)? else {
      continue;
    };

    let doc_count = terms.get_doc_count()?;
    let mut visited = FixedBitSet::new(max_doc as usize);

    let mut te = terms.iterator()?;
    while te.next()?.is_some() {
      let mut de = TestUtil::docs(random, &mut te, None, NONE as i32)?;
      while de.next_doc()? != NO_MORE_DOCS {
        visited.set(de.doc_id() as usize);
      }
    }

    assert_eq!(visited.cardinality(), doc_count as usize);
  }

  Ok(())
}
