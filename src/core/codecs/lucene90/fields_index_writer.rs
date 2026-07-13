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
use crate::core::codecs::CodecUtil;
use crate::core::index::IndexFileNames;
use crate::core::store::directory::Directory;
use crate::core::store::{DataInput, DataOutput, IOContext, IndexOutput};
use crate::core::util::StringHelper;
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::core::util::packed::direct_monotonic_writer::DirectMonotonicWriter;

pub struct FieldsIndexWriter<D>
where
  D: Directory,
{
  dir: D,
  name: String,
  suffix: String,
  extension: String,
  codec_name: String,
  id: [u8; StringHelper::ID_LENGTH],
  block_shift: i32,
  io_context: IOContext,
  docs_out: D::IndexOutput,
  file_pointers_out: D::IndexOutput,
  docs_out_pending_delete: bool,
  file_pointers_out_pending_delete: bool,
  temp_outputs_closed: bool,
  total_docs: i32,
  total_chunks: i32,
  previous_fp: usize,
}
pub(crate) mod fields_index_writer_const {
  pub(crate) const VERSION_START: i32 = 0;
  pub(crate) const VERSION_CURRENT: i32 = 0;
}

impl<D> FieldsIndexWriter<D>
where
  D: Directory,
{
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    dir: D,
    name: &str,
    suffix: &str,
    extension: &str,
    codec_name: &str,
    id: [u8; StringHelper::ID_LENGTH],
    block_shift: i32,
    io_context: IOContext, // TODO:avoid copy? could wrap with Rc?
  ) -> Result<Self> {
    let mut docs_out =
      dir.create_temp_output(name, &format!("{codec_name}-doc_ids"), &io_context)?;

    let mut file_pointers_out = None;
    let init_result: Result<()> = (|| {
      CodecUtil::write_header(
        &mut docs_out,
        &format!("{codec_name}Docs"),
        fields_index_writer_const::VERSION_CURRENT,
      )?;

      let mut out =
        dir.create_temp_output(name, &format!("{codec_name}file_pointers"), &io_context)?;
      CodecUtil::write_header(
        &mut out,
        &format!("{codec_name}FilePointers"),
        fields_index_writer_const::VERSION_CURRENT,
      )?;
      file_pointers_out = Some(out);
      Ok(())
    })();

    if let Err(init_error) = init_result {
      let has_file_pointers_out = file_pointers_out.is_some();
      Self::close_temp_outputs(
        &dir,
        &mut docs_out,
        file_pointers_out.as_mut(),
        false,
        true,
        has_file_pointers_out,
      )?;
      return Err(init_error);
    }

    let file_pointers_out = file_pointers_out
      .ok_or_else(|| LuceneError::illegal_state("file_pointers_out must be initialized"))?;

    Ok(FieldsIndexWriter {
      dir,
      name: name.to_string(),
      suffix: suffix.to_string(),
      extension: extension.to_string(),
      codec_name: codec_name.to_string(),
      id,
      block_shift,
      io_context,
      docs_out,
      file_pointers_out,
      docs_out_pending_delete: true,
      file_pointers_out_pending_delete: true,
      temp_outputs_closed: false,
      total_docs: 0,
      total_chunks: 0,
      previous_fp: 0,
    })
  }
  pub(crate) fn write_index(&mut self, num_docs: i32, start_pointer: usize) -> Result<()> {
    debug_assert!(start_pointer >= self.previous_fp);
    debug_assert!(self.docs_out_pending_delete);
    debug_assert!(self.file_pointers_out_pending_delete);
    debug_assert!(!self.temp_outputs_closed);
    self.docs_out.write_vint(num_docs)?;
    self
      .file_pointers_out
      .write_vlong((start_pointer - self.previous_fp) as i64)?;
    self.previous_fp = start_pointer;
    self.total_docs += num_docs;
    self.total_chunks += 1;
    Ok(())
  }

  pub(crate) fn finish(
    &mut self,
    num_docs: i32,
    max_pointer: usize,
    meta_out: &mut D::IndexOutput,
  ) -> Result<()> {
    if num_docs != self.total_docs {
      return Err(LuceneError::illegal_state(format!(
        "Expected {} docs, but got {}",
        num_docs, self.total_docs
      )));
    }

    CodecUtil::write_footer(&mut self.docs_out)?;
    CodecUtil::write_footer(&mut self.file_pointers_out)?;
    let docs_out_file_name = self.docs_out.get_name().to_string();
    let file_pointers_out_file_name = self.file_pointers_out.get_name().to_string();
    let close_result = IOUtils::close(
      [&mut self.docs_out, &mut self.file_pointers_out],
      Closeable::close,
    );
    if close_result.is_ok() {
      self.temp_outputs_closed = true;
    }
    close_result?;

    let mut data_out = self.dir.create_output(
      &IndexFileNames::segment_file_name(&self.name, &self.suffix, &self.extension),
      &self.io_context,
    )?;
    let result = (|| -> Result<()> {
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
      meta_out.write_long(data_out.get_file_pointer()? as i64)?;

      {
        let mut docs_in = self.dir.open_checksum_input(&docs_out_file_name)?;
        let result: Result<()> = (|| {
          CodecUtil::check_header(
            &mut docs_in,
            &format!("{}Docs", self.codec_name),
            fields_index_writer_const::VERSION_CURRENT,
            fields_index_writer_const::VERSION_CURRENT,
          )?;

          let body_result: Result<()> = (|| {
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
          match body_result {
            Ok(()) => CodecUtil::check_footer(&mut docs_in).map(|_| ()),
            Err(e) => Err(CodecUtil::check_footer_with_error(&mut docs_in, e)),
          }
        })();
        IOUtils::use_or_suppress_result(result, docs_in.close())?;
      }
      self.dir.delete_file(&docs_out_file_name)?;
      self.docs_out_pending_delete = false;
      meta_out.write_long(data_out.get_file_pointer()? as i64)?;
      {
        let mut file_pointers_in = self.dir.open_checksum_input(&file_pointers_out_file_name)?;
        let result: Result<()> = (|| {
          CodecUtil::check_header(
            &mut file_pointers_in,
            &format!("{}FilePointers", self.codec_name),
            fields_index_writer_const::VERSION_CURRENT,
            fields_index_writer_const::VERSION_CURRENT,
          )?;

          let body_result: Result<()> = (|| {
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
          match body_result {
            Ok(()) => CodecUtil::check_footer(&mut file_pointers_in).map(|_| ()),
            Err(e) => Err(CodecUtil::check_footer_with_error(&mut file_pointers_in, e)),
          }
        })();
        IOUtils::use_or_suppress_result(result, file_pointers_in.close())?;
      }
      self.dir.delete_file(&file_pointers_out_file_name)?;
      self.file_pointers_out_pending_delete = false;
      meta_out.write_long(data_out.get_file_pointer()? as i64)?;
      meta_out.write_long(max_pointer as i64)?;
      CodecUtil::write_footer(&mut data_out)
    })();
    IOUtils::use_or_suppress_result(result, data_out.close())
  }

  fn close_temp_outputs(
    dir: &D,
    docs_out: &mut D::IndexOutput,
    file_pointers_out: Option<&mut D::IndexOutput>,
    temp_outputs_closed: bool,
    docs_out_pending_delete: bool,
    file_pointers_out_pending_delete: bool,
  ) -> Result<()> {
    let mut file_names = Vec::new();
    if docs_out_pending_delete {
      file_names.push(docs_out.get_name().to_string());
    }
    if file_pointers_out_pending_delete && let Some(out) = file_pointers_out.as_ref() {
      file_names.push(out.get_name().to_string());
    }

    let close_result = if temp_outputs_closed {
      Ok(())
    } else {
      match file_pointers_out {
        Some(out) => IOUtils::close([docs_out, out], Closeable::close),
        None => IOUtils::close(std::iter::once(docs_out), Closeable::close),
      }
    };

    let delete_result = IOUtils::delete_files(dir, &file_names);
    delete_result?;
    close_result
  }
}
impl<D> Closeable for FieldsIndexWriter<D>
where
  D: Directory,
{
  fn close(&mut self) -> Result<()> {
    if !self.docs_out_pending_delete && !self.file_pointers_out_pending_delete {
      return Ok(());
    }

    let cleanup_result = Self::close_temp_outputs(
      &self.dir,
      &mut self.docs_out,
      Some(&mut self.file_pointers_out),
      self.temp_outputs_closed,
      self.docs_out_pending_delete,
      self.file_pointers_out_pending_delete,
    );

    self.docs_out_pending_delete = false;
    self.file_pointers_out_pending_delete = false;
    self.temp_outputs_closed = true;

    cleanup_result
  }
}

impl<D> Drop for FieldsIndexWriter<D>
where
  D: Directory,
{
  fn drop(&mut self) {
    let _ = self.close();
  }
}
