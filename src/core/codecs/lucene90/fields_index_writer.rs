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
use std::fs;

use crate::core::codecs::CodecUtil;
use crate::core::index::IndexFileNames;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, IOContext, IndexOutput};
use crate::core::util::StringHelper;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::direct_monotonic_writer::DirectMonotonicWriter;

pub struct FieldsIndexWriter<O>
where
    O: IndexOutput,
{
    name: String,
    suffix: String,
    extension: String,
    codec_name: String,
    id: [u8; StringHelper::ID_LENGTH],
    block_shift: i32,
    io_context: IOContext,
    // Using Option to wrap the IndexOutput makes it easier to release the
    // resource, which avoids the need to implement the IndexOutput's
    // Default trait.
    docs_out: Option<O>,
    file_pointers_out: Option<O>,
    total_docs: i32,
    total_chunks: i32,
    previous_fp: i64,
}
pub(crate) mod fields_index_writer_const {
    pub(crate) const VERSION_START: i32 = 0;
    pub(crate) const VERSION_CURRENT: i32 = 0;
}

impl<O> FieldsIndexWriter<O>
where
    O: IndexOutput,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new<D>(
        dir: &D,
        name: &str,
        suffix: &str,
        extension: &str,
        codec_name: &str,
        id: [u8; StringHelper::ID_LENGTH],
        block_shift: i32,
        io_context: IOContext, // TODO:avoid copy? could wrap with Rc?
    ) -> Result<Self>
    where
        D: Directory<IndexOutput = O>,
    {
        let mut docs_out =
            dir.create_temp_output(name, &format!("{codec_name}-doc_ids"), &io_context)?;
        CodecUtil::write_header(
            &mut docs_out,
            &format!("{codec_name}Docs"),
            fields_index_writer_const::VERSION_CURRENT,
        )?;

        let mut file_pointers_out =
            dir.create_temp_output(name, &format!("{codec_name}file_pointers"), &io_context)?;
        CodecUtil::write_header(
            &mut file_pointers_out,
            &format!("{codec_name}FilePointers"),
            fields_index_writer_const::VERSION_CURRENT,
        )?;

        Ok(FieldsIndexWriter {
            name: name.to_string(),
            suffix: suffix.to_string(),
            extension: extension.to_string(),
            codec_name: codec_name.to_string(),
            id,
            block_shift,
            io_context,
            docs_out: Option::from(docs_out),
            file_pointers_out: Option::from(file_pointers_out),
            total_docs: 0,
            total_chunks: 0,
            previous_fp: 0,
        })
    }
    pub(crate) fn write_index(&mut self, num_docs: i32, start_pointer: i64) -> Result<()> {
        debug_assert!(start_pointer >= self.previous_fp);
        debug_assert!(self.docs_out.is_some());
        debug_assert!(self.file_pointers_out.is_some());
        self.docs_out.as_mut().unwrap().write_vint(num_docs)?;
        self.file_pointers_out
            .as_mut()
            .unwrap()
            .write_vlong(start_pointer - self.previous_fp)?;
        self.previous_fp = start_pointer;
        self.total_docs += num_docs;
        self.total_chunks += 1;
        Ok(())
    }

    pub(crate) fn finish<D>(
        &mut self,
        num_docs: i32,
        max_pointer: usize,
        meta_out: &mut O,
        dir: &D,
    ) -> Result<()>
    where
        D: Directory,
    {
        if num_docs != self.total_docs {
            return Err(LuceneError::illegal_state(format!(
                "Expected {} docs, but got {}",
                num_docs, self.total_docs
            )));
        }

        CodecUtil::write_footer(self.docs_out.as_mut().unwrap())?;
        CodecUtil::write_footer(self.file_pointers_out.as_mut().unwrap())?;
        let docs_out_file_name = self.docs_out.as_ref().unwrap().get_name().to_string();
        let file_pointers_out_file_name = self
            .file_pointers_out
            .as_ref()
            .unwrap()
            .get_name()
            .to_string();
        {
            let _ = std::mem::take(&mut self.docs_out);
            let _ = std::mem::take(&mut self.file_pointers_out);
        }

        let mut data_out = dir.create_output(
            &IndexFileNames::segment_file_name(&self.name, &self.suffix, &self.extension),
            &self.io_context,
        )?;
        CodecUtil::write_index_header(
            &mut data_out,
            &format!("{}Idx", self.codec_name),
            fields_index_writer_const::VERSION_CURRENT,
            &self.id,
            &self.suffix,
        )?;

        meta_out.write_int(num_docs)?;
        meta_out.write_int(self.block_shift)?;
        meta_out.write_int(self.total_chunks + 1)?;
        meta_out.write_long(data_out.get_file_pointer() as i64)?;

        {
            let mut docs_in = dir.open_checksum_input(&docs_out_file_name)?;
            let mut prior_e = None;
            let result: Result<()> = (|| {
                CodecUtil::check_header(
                    &mut docs_in,
                    &format!("{}Docs", self.codec_name),
                    fields_index_writer_const::VERSION_CURRENT,
                    fields_index_writer_const::VERSION_CURRENT,
                )?;

                let mut docs = DirectMonotonicWriter::get_instance(
                    meta_out,
                    &mut data_out,
                    (self.total_chunks + 1) as i64,
                    self.block_shift,
                )?;
                let mut doc = 0;
                docs.add(doc)?;
                for _ in 0..self.total_chunks {
                    doc += docs_in.read_vint()? as i64;
                    docs.add(doc)?;
                }
                docs.finish()?;

                if doc != self.total_docs as i64 {
                    return Err(LuceneError::corrupt_index("Docs don't add up".to_string()));
                }
                Ok(())
            })();
            if let Err(e) = result {
                prior_e = Some(e);
            }

            if let Some(e) = prior_e {
                return Err(CodecUtil::check_footer_with_error(&mut docs_in, e));
            } else {
                CodecUtil::check_footer(&mut docs_in)?;
            }
        }
        dir.delete_file(&docs_out_file_name)?;
        meta_out.write_long(data_out.get_file_pointer() as i64)?;
        {
            let mut file_pointers_in = dir.open_checksum_input(&file_pointers_out_file_name)?;
            let mut prior_e = None;
            let result = (|| {
                CodecUtil::check_header(
                    &mut file_pointers_in,
                    &format!("{}FilePointers", self.codec_name),
                    fields_index_writer_const::VERSION_CURRENT,
                    fields_index_writer_const::VERSION_CURRENT,
                )?;

                let mut file_pointers = DirectMonotonicWriter::get_instance(
                    meta_out,
                    &mut data_out,
                    (self.total_chunks + 1) as i64,
                    self.block_shift,
                )?;
                let mut fp = 0;
                for _ in 0..self.total_chunks {
                    fp += file_pointers_in.read_vlong()?;
                    file_pointers.add(fp)?;
                }
                if max_pointer < fp as usize {
                    return Err(LuceneError::corrupt_index(
                        "File pointers don't add up".to_string(),
                    ));
                }
                file_pointers.add(max_pointer as i64)?;
                file_pointers.finish()?;
                Ok(())
            })();
            if let Err(e) = result {
                prior_e = Some(e);
            }
            if let Some(e) = prior_e {
                return Err(CodecUtil::check_footer_with_error(&mut file_pointers_in, e));
            } else {
                CodecUtil::check_footer(&mut file_pointers_in)?;
            }
        }
        dir.delete_file(&file_pointers_out_file_name)?;
        meta_out.write_long(data_out.get_file_pointer() as i64)?;
        meta_out.write_long(max_pointer as i64)?;
        CodecUtil::write_footer(&mut data_out)?;
        Ok(())
    }
}
impl<O> Drop for FieldsIndexWriter<O>
where
    O: IndexOutput,
{
    fn drop(&mut self) {
        if self.docs_out.is_some() {
            match fs::remove_file(self.docs_out.as_ref().unwrap().get_name()) {
                Ok(_) => {},
                Err(_e) => {
                    // TODO IMPORTANT
                    // debug_assert!(false, "Failed to delete docs file: {:?}", e);
                },
            }
        }
        if self.file_pointers_out.is_some() {
            match fs::remove_file(self.file_pointers_out.as_ref().unwrap().get_name()) {
                Ok(_) => {},
                Err(_e) => {
                    // TODO IMPORTANT
                    // debug_assert!(false, "Failed to delete file pointers file: {:?}", e);
                },
            }
        }
        let _ = self.docs_out.take();
        let _ = self.file_pointers_out.take();
    }
}
