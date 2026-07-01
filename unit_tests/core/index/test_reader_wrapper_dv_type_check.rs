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
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::index::BytesRef;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::util::error::lucene_error::Result;
use crate::test::support::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::support::core::index::random_index_writer::RandomIndexWriter;
use crate::test::support::core::util::lucene_test_case::{
  get_only_leaf_reader, new_directory_shared, new_index_writer_config_with_analyzer,
  new_string_field, random, random_from_seed, rarely,
};
use crate::test::support::core::util::test_util::TestUtil;
use rand::RngExt;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestReaderWrapperDVTypeCheck;

#[test]
fn test_no_dv_field_on_segment() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;
  let analyzer = MockAnalyzer::new(&mut random);
  // TODO IMPORTANT setCodec未实现
  let cfg = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  let iw = RandomIndexWriter::with_config(&mut random, dir, cfg);

  let mut sdv_exist = false;
  let mut ssdv_exist = false;

  let seed = random.random::<u64>();
  {
    let mut index_random = random_from_seed(seed);
    let docs = TestUtil::next_int(&mut index_random, 1, 4);
    let mut field_to_type = HashMap::new();

    for i in 0..docs {
      let mut d = Document::new();
      d.add(new_string_field(
        &mut index_random,
        "id",
        i.to_string(),
        Store::No,
        &mut field_to_type,
      )?);
      if rarely(&mut index_random) {
        d.add(SortedDocValuesField::new(
          "sdv",
          BytesRef::from_string(&i.to_string()),
        ));
        sdv_exist = true;
      }
      let num_sorted_set = index_random.random_range(0..5) - 3;
      for j in 0..num_sorted_set {
        d.add(SortedSetDocValuesField::new(
          "ssdv",
          BytesRef::from_string(&j.to_string()),
        ));
        ssdv_exist = true;
      }
      iw.add_document(&mut index_random, d)?;
      iw.commit(&mut index_random)?;
    }
  }
  iw.force_merge(&mut random, 1)?;
  let reader = iw.get_reader(&mut random)?;

  iw.close(&mut random)?;
  let wrapper = get_only_leaf_reader(&reader)?;

  let sdv = wrapper.get_sorted_doc_values("sdv")?;
  let ssdv = wrapper.get_sorted_set_doc_values("ssdv")?;

  assert!(
    wrapper.get_sorted_doc_values("ssdv")?.is_none(),
    "confusing DV type"
  );
  assert!(
    wrapper.get_sorted_set_doc_values("sdv")?.is_none(),
    "confusing DV type"
  );

  assert!(
    wrapper.get_sorted_doc_values("NOssdv")?.is_none(),
    "absent field"
  );
  assert!(
    wrapper.get_sorted_set_doc_values("NOsdv")?.is_none(),
    "absent field"
  );

  assert_eq!(sdv_exist, sdv.is_some(), "optional sdv field");
  assert_eq!(ssdv_exist, ssdv.is_some(), "optional ssdv field");

  reader.close()?;

  Ok(())
}
