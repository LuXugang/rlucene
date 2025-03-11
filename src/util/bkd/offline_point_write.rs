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
use crate::codecs::CodecUtil;
use crate::store::directory::Directory;
use crate::store::{DataOutput, IOContext, IndexOutput};
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::offline_point_reader::OfflinePointReader;
use crate::util::bkd::point_reader::PointReaderEnum;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::bkd::point_writer::PointWriter;
use crate::util::error::lucene_error::LuceneError;
use std::cell::RefCell;
use std::rc::Rc;

/// Writes points to disk in a fixed-width format.
pub struct OfflinePointWriter<D>
where
    D: Directory,
{
    pub temp_dir: Rc<RefCell<D>>,
    pub out: Option<D::IndexOutputType>,
    pub name: String,
    pub config: Rc<BKDConfig>,
    pub count: i64,
    pub closed: bool,
    pub expected_count: i64,
}

impl<D> OfflinePointWriter<D>
where
    D: Directory,
{
    /// Create a new writer with an unknown number of incoming points
    pub fn new(
        config: Rc<BKDConfig>,
        temp_dir: Rc<RefCell<D>>,
        temp_file_name_prefix: &str,
        desc: &str,
        expected_count: i64,
    ) -> Result<Self, LuceneError> {
        let out = temp_dir.borrow_mut().create_temp_output(
            temp_file_name_prefix,
            &format!("bkd_{}", desc),
            &IOContext::default_io_context()?,
        )?;
        let name = out.get_name().to_string();
        Ok(OfflinePointWriter {
            temp_dir,
            out: Option::from(out),
            name,
            config,
            count: 0,
            closed: false,
            expected_count,
        })
    }

    pub fn get_reader_with_buffer(
        &self,
        start: i64,
        length: i64,
        reusable_buffer: Rc<RefCell<Vec<u8>>>,
    ) -> Result<PointReaderEnum<D>, LuceneError> {
        debug_assert!(
            self.closed && self.out.is_none(),
            "point writer is still open and trying to get a reader"
        );
        debug_assert!(
            start + length <= self.count,
            "start={} length={} count={}",
            start,
            length,
            self.count
        );
        debug_assert!(
            self.expected_count == 0 || self.count == self.expected_count,
            "expectedCount={} vs count={}",
            self.expected_count,
            self.count
        );
        let reader = OfflinePointReader::new(
            self.config.clone(),
            self.temp_dir.clone(),
            &self.name,
            start,
            length,
            reusable_buffer,
        )?;
        Ok(PointReaderEnum::Offline(reader))
    }
}
impl<D> Drop for OfflinePointWriter<D>
where
    D: Directory,
{
    fn drop(&mut self) {
        self.close();
    }
}

impl<D> std::fmt::Display for OfflinePointWriter<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OfflinePointWriter(count={} tempFileName={})",
            self.count, self.name
        )
    }
}

impl<D> PointWriter for OfflinePointWriter<D>
where
    D: Directory,
{
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<(), LuceneError> {
        debug_assert!(!self.closed, "Point writer is already closed");
        assert_eq!(
            packed_value.len(),
            self.config.packed_bytes_length() as usize,
            "[packedValue] must have length [{}] but was [{}]",
            self.config.packed_bytes_length(),
            packed_value.len()
        );
        debug_assert!(packed_value.len() <= i32::MAX as usize);
        let out = self.out.as_mut().unwrap();
        out.write_bytes_range(packed_value, 0, packed_value.len() as i32)?;
        out.write_int(i32::to_be(doc_id))?;
        self.count += 1;
        debug_assert!(
            self.expected_count == 0 || self.count <= self.expected_count,
            "expectedCount={} vs count={}",
            self.expected_count,
            self.count
        );
        Ok(())
    }

    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<(), LuceneError> {
        debug_assert!(!self.closed, "Point writer is already closed");
        let (offset, length) = point_value.packed_value_doc_id_bytes();
        assert_eq!(
            length,
            self.config.bytes_per_doc(),
            "[packedValue and docID] must have length [{}] but was [{}]",
            self.config.bytes_per_doc(),
            length
        );
        self.out.as_mut().unwrap().write_bytes_range(
            point_value.get_value().borrow_mut().as_slice(),
            offset,
            length,
        )?;
        self.count += 1;
        debug_assert!(
            self.expected_count == 0 || self.count <= self.expected_count,
            "expectedCount={} vs count={}",
            self.expected_count,
            self.count
        );
        Ok(())
    }

    type Dir = D;

    fn get_reader(
        &self,
        start: i64,
        length: i64,
    ) -> Result<PointReaderEnum<Self::Dir>, LuceneError> {
        let buffer = Rc::new(RefCell::new(vec![
            0u8;
            self.config.bytes_per_doc() as usize
        ]));
        self.get_reader_with_buffer(start, length, buffer)
    }
    fn count(&self) -> i64 {
        self.count
    }

    fn destroy(&mut self) -> Result<(), LuceneError> {
        self.temp_dir.borrow_mut().delete_file(&self.name)
    }

    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            let mut out = self.out.take().unwrap();
            match CodecUtil::write_footer(&mut out) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Failed to write footer: {:?}", e);
                }
            };
        }
    }
}
