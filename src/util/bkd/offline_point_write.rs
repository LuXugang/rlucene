/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::rc::Rc;

use crate::codecs::CodecUtil;
use crate::store::directory::Directory;
use crate::store::{IOContext, IndexInput, IndexOutput};
use crate::util::bkd::bkd_config::BKDConfig;
use crate::util::bkd::offline_point_reader::OfflinePointReader;
use crate::util::bkd::point_value::{PointValue, PointValueEnum};
use crate::util::bkd::point_writer::PointWriter;
use crate::util::error::lucene_error::Result;

/// Writes points to disk in a fixed-width format.
pub struct OfflinePointWriter<O>
where
    O: IndexOutput,
{
    pub out: Option<O>,
    pub name: String,
    pub config: Rc<BKDConfig>,
    pub count: i64,
    pub closed: bool,
    pub expected_count: i64,
}

impl<O> OfflinePointWriter<O>
where
    O: IndexOutput,
{
    /// Create a new writer with an unknown number of incoming points
    pub fn new<D>(
        config: Rc<BKDConfig>,
        temp_dir: &mut D,
        temp_file_name_prefix: &str,
        desc: &str,
        expected_count: i64,
    ) -> Result<Self>
    where
        D: Directory<IndexOutputType = O>,
    {
        let out = temp_dir.create_temp_output(
            temp_file_name_prefix,
            &format!("bkd_{desc}"),
            &IOContext::default_io_context()?,
        )?;
        let name = out.get_name().to_string();
        Ok(OfflinePointWriter {
            out: Option::from(out),
            name,
            config,
            count: 0,
            closed: false,
            expected_count,
        })
    }

    pub fn get_reader_with_buffer<D: Directory>(
        &self,
        start: i64,
        length: i64,
        reusable_buffer: Vec<u8>,
        temp_dir: &mut D,
    ) -> Result<OfflinePointReader<D::IndexInputType>> {
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
            temp_dir,
            &self.name,
            start,
            length,
            reusable_buffer,
        )?;
        Ok(reader)
    }
}
impl<O> Drop for OfflinePointWriter<O>
where
    O: IndexOutput,
{
    fn drop(&mut self) {
        self.close();
    }
}

impl<O> std::fmt::Display for OfflinePointWriter<O>
where
    O: IndexOutput,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OfflinePointWriter(count={} tempFileName={})",
            self.count, self.name
        )
    }
}

impl<O> PointWriter for OfflinePointWriter<O>
where
    O: IndexOutput,
{
    fn append_bytes(&mut self, packed_value: &[u8], doc_id: i32) -> Result<()> {
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

    fn append_point_value(&mut self, point_value: &PointValueEnum) -> Result<()> {
        debug_assert!(!self.closed, "Point writer is already closed");
        let (value, offset, length) = point_value.packed_value_doc_id_bytes();
        assert_eq!(
            length,
            self.config.bytes_per_doc(),
            "[packedValue and docID] must have length [{}] but was [{}]",
            self.config.bytes_per_doc(),
            length
        );
        self.out
            .as_mut()
            .unwrap()
            .write_bytes_range(value, offset, length)?;
        self.count += 1;
        debug_assert!(
            self.expected_count == 0 || self.count <= self.expected_count,
            "expectedCount={} vs count={}",
            self.expected_count,
            self.count
        );
        Ok(())
    }

    type PointReader<I>
        = OfflinePointReader<I>
    where
        I: IndexInput;

    fn get_reader<D>(
        &mut self,
        start: i64,
        length: i64,
        temp_dir: &mut D,
    ) -> Result<Self::PointReader<D::IndexInputType>>
    where
        D: Directory,
    {
        let buffer = vec![0u8; self.config.bytes_per_doc() as usize];
        self.get_reader_with_buffer(start, length, buffer, temp_dir)
    }

    fn count(&self) -> i64 {
        self.count
    }

    fn destroy<D>(&mut self, dir: &mut D) -> Result<()>
    where
        D: Directory,
    {
        dir.delete_file(&self.name)
    }

    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            let mut out = self.out.take().unwrap();
            match CodecUtil::write_footer(&mut out) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to write footer: {e:?}");
                },
            };
        }
    }
}
