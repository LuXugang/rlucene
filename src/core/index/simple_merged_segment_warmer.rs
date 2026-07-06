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
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexReaderWarmer;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::segment_reader::DefaultLeafReader;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term_vectors::TermVectors;
use crate::core::store::directory::Directory;
use crate::core::util::info_stream::{InfoStream, InfoStreamMT};
use std::time::Instant;

/// A very simple merged segment warmer that just ensures data structures are initialized.
pub struct SimpleMergedSegmentWarmer {
  info_stream: InfoStreamMT,
}

impl SimpleMergedSegmentWarmer {
  /// Creates a new `SimpleMergedSegmentWarmer`.
  ///
  /// * `info_stream` - `InfoStream` to log statistics about warming.
  pub fn new(info_stream: InfoStreamMT) -> Self {
    Self { info_stream }
  }
}

impl<D> IndexReaderWarmer<D> for SimpleMergedSegmentWarmer
where
  D: Directory,
{
  fn warm(
    &self,
    reader: &DefaultLeafReader<D>,
  ) -> crate::core::util::error::lucene_error::Result<()> {
    let start_time = Instant::now();
    let mut indexed_count = 0;
    let mut doc_values_count = 0;
    let mut norms_count = 0;

    for info in reader.get_field_infos()?.iter() {
      if info.get_index_options() != &IndexOptions::None {
        LeafReader::terms(reader, info.get_name())?;
        indexed_count += 1;

        if info.has_norms() {
          LeafReader::get_norm_values(reader, info.get_name())?;
          norms_count += 1;
        }
      }

      if info.get_doc_values_type() != &DocValuesType::None {
        match info.get_doc_values_type() {
          DocValuesType::Numeric => {
            LeafReader::get_numeric_doc_values(reader, info.get_name())?;
          },
          DocValuesType::Binary => {
            LeafReader::get_binary_doc_values(reader, info.get_name())?;
          },
          DocValuesType::Sorted => {
            LeafReader::get_sorted_doc_values(reader, info.get_name())?;
          },
          DocValuesType::SortedNumeric => {
            LeafReader::get_sorted_numeric_doc_values(reader, info.get_name())?;
          },
          DocValuesType::SortedSet => {
            LeafReader::get_sorted_set_doc_values(reader, info.get_name())?;
          },
          DocValuesType::None => {
            debug_assert!(false, "unknown dv type");
          },
        }
        doc_values_count += 1;
      }
    }

    IndexReader::stored_fields(reader)?.document(0)?;
    IndexReader::term_vectors(reader)?.get(0)?;

    if self.info_stream.is_enabled("SMSW") {
      self.info_stream.message(
        "SMSW",
        &format!(
          "Finished warming segment: {}, indexed={}, docValues={}, norms={}, time={}",
          reader,
          indexed_count,
          doc_values_count,
          norms_count,
          start_time.elapsed().as_millis()
        ),
      )?;
    }
    Ok(())
  }
}
