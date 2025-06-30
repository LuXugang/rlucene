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
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_writer::{StoredFieldsWriter, StoredFieldsWriterEnum};
use crate::codecs::Codec;
use crate::document::stored_value::{StoredValue, StoredValueType};
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::sorting_stored_fields_consumer::SortingStoredFieldsConsumer;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::Result;
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct StoredFieldsConsumer<D>
where
    D: Directory,
{
    directory: Arc<Mutex<D>>,
    pub(crate) info: Rc<SegmentInfo<D>>,
    pub(crate) writer: Option<StoredFieldsWriterEnum<D>>,
    last_doc: i32,
    sub: Option<SortingStoredFieldsConsumer<D>>,
}
impl<D> StoredFieldsConsumer<D>
where
    D: Directory,
{
    pub(crate) fn new(
        directory: Arc<Mutex<D>>,
        info: Rc<SegmentInfo<D>>,
        sub: Option<SortingStoredFieldsConsumer<D>>,
    ) -> Self {
        Self {
            directory,
            info,
            writer: None,
            last_doc: -1,
            sub,
        }
    }
    fn init_stored_fields_writer(&mut self, codec: &impl Codec) -> Result<()> {
        match self.sub {
            Some(ref mut sub) => {
                if sub.writer.is_none() {
                    sub.init_stored_fields_writer(self.info.clone())?;
                }
            },
            None => {
                if self.writer.is_none() {
                    let writer = codec.stored_fields_format().fields_writer(
                        self.directory.clone(),
                        self.info.clone(),
                        &IOContext::default_io_context()?,
                    )?;
                    self.writer = Some(writer);
                }
            },
        }
        Ok(())
    }

    pub(crate) fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        debug_assert!(self.last_doc < doc_id);
        self.init_stored_fields_writer(codec)?;

        match self.sub {
            Some(ref mut sub) => {
                while self.last_doc + 1 < doc_id {
                    self.last_doc += 1;
                    if let Some(writer) = &mut sub.writer {
                        writer.start_document()?;
                        writer.finish_document()?;
                    }
                }
                self.last_doc += 1;
                if let Some(writer) = &mut sub.writer {
                    writer.start_document()?;
                }
            },
            None => {
                while self.last_doc + 1 < doc_id {
                    self.last_doc += 1;
                    if let Some(writer) = &mut self.writer {
                        writer.start_document()?;
                        writer.finish_document()?;
                    }
                }
                self.last_doc += 1;
                if let Some(writer) = &mut self.writer {
                    writer.start_document()?;
                }
            },
        }
        Ok(())
    }

    pub(crate) fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        let writer = self.writer.as_mut().expect("writer must be initialized");

        match value.get_type() {
            StoredValueType::Integer => writer.write_field_i32(info, value.get_int_value()?),
            StoredValueType::Long => writer.write_field_i64(info, value.get_long_value()?),
            StoredValueType::Float => writer.write_field_f32(info, value.get_float_value()?),
            StoredValueType::Double => writer.write_field_f64(info, value.get_double_value()?),
            StoredValueType::Binary => writer.write_field_bytes(info, value.get_binary_value()?),
            StoredValueType::String => writer.write_field_str(info, value.get_string_value()?),
        }
    }

    pub(crate) fn finish_document(&mut self) -> Result<()> {
        match self.sub {
            Some(ref mut sub) => {
                let writer = sub.writer.as_mut().expect("sub writer must be initialized");
                writer.finish_document()?;
            },
            None => {
                let writer = self.writer.as_mut().expect("writer must be initialized");
                writer.finish_document()?;
            },
        }
        Ok(())
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        while self.last_doc < max_doc - 1 {
            self.start_document(codec, self.last_doc + 1)?;
            self.finish_document()?;
        }
        Ok(())
    }

    fn flush<DM>(&mut self, state: &SegmentWriteState<D>, _sort_map: Option<Rc<DM>>) -> Result<()>
    where
        DM: DocMap,
    {
        match self.sub {
            Some(ref mut sub) => {
                sub.writer
                    .as_mut()
                    .unwrap()
                    .finish(state.segment_info.max_doc()?)?;
                let _ = sub.writer.take();
            },
            None => {
                self.writer
                    .as_mut()
                    .unwrap()
                    .finish(state.segment_info.max_doc()?)?;
                let _ = self.writer.take();
            },
        }
        Ok(())
    }

    pub(crate) fn abort(&mut self) -> Result<()> {
        match self.sub {
            Some(ref mut sub) => sub.abort(),
            None => Ok(()),
        }
    }
}

pub(crate) trait StoredFieldsConsumerBase {
    type Directory: Directory;
    fn init_stored_fields_writer(&mut self, info: Rc<SegmentInfo<Self::Directory>>) -> Result<()>;
    fn flush<DM>(
        &mut self,
        state: &SegmentWriteState<Self::Directory>,
        sort_map: Option<Rc<DM>>,
        codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap;
    fn abort(&mut self) -> Result<()>;
}
