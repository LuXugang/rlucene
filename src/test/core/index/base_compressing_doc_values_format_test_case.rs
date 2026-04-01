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
use crate::core::document::field::FieldBase;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::index::index_writer::IndexWriter;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::packed::PackedInts;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::base_doc_values_format_test_case::BaseDocValuesFormatTestCase;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer,
};
use crate::test::core::util::test_util::TestUtil;
use rand::Rng;
use rand::RngExt;
use rand::prelude::IndexedRandom;

pub trait BaseCompressingDocValuesFormatTestCase: BaseDocValuesFormatTestCase {
  fn dir_size<D: Directory>(&self, directory: &D) -> Result<usize> {
    let mut size = 0;
    for file in directory.list_all()? {
      size += directory.file_length(&file)?;
    }
    Ok(size)
  }

  fn test_unique_values_compression<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    // TODO ByteBuffersDirectory 未实现
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iwriter = IndexWriter::new(dir.clone(), iwc)?;

    let unique_value_count = TestUtil::next_int(random, 1, 256) as usize;
    let mut values = Vec::new();

    let mut doc = Document::new();
    let mut dvf = NumericDocValuesField::new("dv", 0);
    doc.add(dvf.clone());
    for _ in 0..300 {
      let value = if values.len() < unique_value_count {
        let value = random.random::<i64>();
        values.push(value);
        value
      } else {
        *values.choose(random).unwrap()
      };
      dvf.set_long_value(value)?;
      iwriter.add_document(doc.clone())?;
    }
    iwriter.force_merge(1)?;
    let size1 = self.dir_size(dir.as_ref())?;
    for _ in 0..20 {
      dvf.set_long_value(*values.choose(random).unwrap())?;
      iwriter.add_document(doc.clone())?;
    }
    iwriter.force_merge(1)?;
    let size2 = self.dir_size(dir.as_ref())?;
    assert!(size2 < size1 + 8 * 20);

    iwriter.close()?;
    Ok(())
  }

  fn test_date_compression<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    // TODO ByteBuffersDirectory 未实现
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iwriter = IndexWriter::new(dir.clone(), iwc)?;

    let base = 13_i64;
    let day = 1000_i64 * 60 * 60 * 24;

    let mut doc = Document::new();
    let mut dvf = NumericDocValuesField::new("dv", 0);
    doc.add(dvf.clone());
    for _ in 0..300 {
      dvf.set_long_value(base + random.random_range(0..1000) * day)?;
      iwriter.add_document(doc.clone())?;
    }
    iwriter.force_merge(1)?;
    let size1 = self.dir_size(dir.as_ref())?;
    for _ in 0..50 {
      dvf.set_long_value(base + random.random_range(0..1000) * day)?;
      iwriter.add_document(doc.clone())?;
    }
    iwriter.force_merge(1)?;
    let size2 = self.dir_size(dir.as_ref())?;
    let packed_cost = (PackedInts::bits_required(day)? as usize * 50) / 8;
    assert!(size2 < size1 + packed_cost);

    iwriter.close()?;
    Ok(())
  }

  fn test_single_big_value_compression<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
    // TODO ByteBuffersDirectory 未实现
    let dir = new_directory_shared(random)?;
    let analyzer = MockAnalyzer::new(random);
    let iwc = new_index_writer_config_with_analyzer(random, analyzer);
    let iwriter = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    let mut dvf = NumericDocValuesField::new("dv", 0);
    doc.add(dvf.clone());
    for i in 0..20000 {
      dvf.set_long_value((i & 1023) as i64)?;
      iwriter.add_document(doc.clone())?;
    }
    iwriter.force_merge(1)?;
    let size1 = self.dir_size(dir.as_ref())?;
    dvf.set_long_value(i64::MAX)?;
    iwriter.add_document(doc.clone())?;
    iwriter.force_merge(1)?;
    let size2 = self.dir_size(dir.as_ref())?;
    assert!(size2 < size1 + (20000 * (63 - 10)) / 8);

    iwriter.close()?;
    Ok(())
  }
}
