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
use crate::codecs::stored_fields_format::StoredFieldsFormat;
use crate::codecs::stored_fields_writer::{StoredFieldsWriter, StoredFieldsWriterEnum};
use crate::codecs::Codec;
use crate::document::stored_value::{StoredValue, StoredValueType};
use crate::index::field_info::FieldInfo;
use crate::index::segment_info::SegmentInfo;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::store::directory::Directory;
use crate::store::IOContext;
use crate::util::error::lucene_error::{LuceneError, Result};
use parking_lot::Mutex;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct StoredFieldsConsumer<D1, D2, S>
where
    D1: Directory,
    D2: Directory,
    S: StoredFieldsConsumerBase,
{
    directory: Arc<Mutex<D1>>,
    pub(crate) info: Rc<SegmentInfo<D2>>,
    pub(crate) writer: Option<StoredFieldsWriterEnum<D1>>,
    last_doc: i32,
    sub: Option<S>,
}
impl<D1, D2, S> StoredFieldsConsumer<D1, D2, S>
where
    D1: Directory,
    D2: Directory,
    S: StoredFieldsConsumerBase<Directory = D2, TempDirectory = D1>,
{
    pub(crate) fn new(
        directory: Arc<Mutex<D1>>,
        info: Rc<SegmentInfo<D2>>,
        sub: Option<S>,
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
        if self.writer.is_none() {
            let mut need_init = false;
            match self.sub {
                Some(ref mut sub) => match sub.init_stored_fields_writer(self.info.clone()) {
                    Ok(writer) => {
                        self.writer = Some(writer);
                    },
                    Err(e) => match e {
                        LuceneError::NotImplemented(_) => need_init = true,
                        _ => return Err(e),
                    },
                },
                None => {
                    need_init = true;
                },
            }
            if need_init {
                let writer = codec.stored_fields_format().fields_writer(
                    self.directory.clone(),
                    self.info.clone(),
                    &IOContext::default_io_context()?,
                )?;
                self.writer = Some(writer);
            }
        }
        Ok(())
    }

    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()> {
        debug_assert!(self.last_doc < doc_id);
        self.init_stored_fields_writer(codec)?;

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
        #[cfg(test)]
        match self.sub {
            Some(ref mut sub) => {
                sub.start_document(codec, self.last_doc)?;
            },
            None => {},
        }

        Ok(())
    }

    fn write_field(&mut self, info: &FieldInfo, value: &StoredValue) -> Result<()> {
        let writer = self.writer.as_mut().expect("writer must be initialized");

        match value.get_type() {
            StoredValueType::INTEGER => writer.write_field_i32(info, value.get_int_value()?),
            StoredValueType::LONG => writer.write_field_i64(info, value.get_long_value()?),
            StoredValueType::FLOAT => writer.write_field_f32(info, value.get_float_value()?),
            StoredValueType::DOUBLE => writer.write_field_f64(info, value.get_double_value()?),
            StoredValueType::BINARY => writer.write_field_bytes(info, value.get_binary_value()?),
            StoredValueType::STRING => writer.write_field_str(info, value.get_string_value()?),
        }
    }

    fn finish_document(&mut self) -> Result<()> {
        let writer = self.writer.as_mut().expect("writer must be initialized");
        writer.finish_document()?;
        #[cfg(test)]
        match self.sub {
            Some(ref mut sub) => sub.finish_document(),
            None => Ok(()),
        }?;
        Ok(())
    }

    fn finish(&mut self, codec: &impl Codec, max_doc: i32) -> Result<()> {
        while self.last_doc < max_doc - 1 {
            self.start_document(codec, self.last_doc + 1)?;
            self.finish_document()?;
        }
        Ok(())
    }

    fn flush<DM>(
        &mut self,
        state: &SegmentWriteState<D2>,
        sort_map: Option<Rc<DM>>,
        codec: &impl Codec,
    ) -> Result<()>
    where
        DM: DocMap,
    {
        self.writer
            .as_mut()
            .expect("writer must be initialized")
            .finish(state.segment_info.max_doc()?)?;
        if let Some(sub) = &mut self.sub {
            if let Err(e) = sub.flush(state, sort_map, codec) {
                if !matches!(e, LuceneError::NotImplemented(_)) {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        match self.sub {
            Some(ref mut sub) => sub.abort(),
            None => Ok(()),
        }
    }
}

pub(crate) trait StoredFieldsConsumerBase {
    type TempDirectory: Directory;
    fn init_stored_fields_writer(
        &mut self,
        info: Rc<SegmentInfo<Self::Directory>>,
    ) -> Result<StoredFieldsWriterEnum<Self::TempDirectory>>;
    #[cfg(test)]
    fn start_document(&mut self, codec: &impl Codec, doc_id: i32) -> Result<()>;
    #[cfg(test)]
    fn finish_document(&mut self) -> Result<()>;

    type Directory: Directory;
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

#[cfg(test)]
mod tests {
    use crate::codecs::Codec;

    use crate::index::field_info::FieldInfo;
    use crate::index::segment_info::SegmentInfo;
    use crate::index::segment_write_state::SegmentWriteState;

    use crate::codecs::lucene101_codec::Lucene101Codec;
    use crate::codecs::stored_fields_writer::StoredFieldsWriterEnum;
    use crate::index::field_infos::FieldInfos;
    use crate::index::sorter::{DocMap, DummyDocMap};
    use crate::index::stored_fields_consumer::{StoredFieldsConsumer, StoredFieldsConsumerBase};

    use crate::store::directory::Directory;
    use crate::store::flush_info::FlushInfo;
    use crate::store::IOContext;
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::util::error::lucene_error::{LuceneError, Result};
    use crate::util::info_stream::{InfoStreamEnum, NoOutput};
    use crate::util::{StringHelper, LATEST};
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestStoredFieldsConsumer;

    #[test]
    fn test_finish() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(Mutex::new(new_directory(&mut random)?));
        let si = Rc::new(SegmentInfo::new(
            dir.clone(),
            Some((*LATEST).clone()),
            Some((*LATEST).clone()),
            "_0".to_string(),
            3,
            false,
            false,
            HashMap::new(),
            StringHelper::random_id(),
            HashMap::new(),
            None,
        )?);
        let sub = StoredFieldsConsumerImpl::new(dir.clone());
        let mut consumer = StoredFieldsConsumer::new(dir.clone(), si.clone(), Some(sub));
        let num_doc = 3;
        let codec = Lucene101Codec;
        consumer.finish(&codec, num_doc)?;

        let stream = Arc::new(Mutex::new(InfoStreamEnum::NoOutput(NoOutput)));
        let field_infos = Rc::new(FieldInfos::new(vec![Rc::from(FieldInfo::default()); 1])?);
        let context = Rc::new(IOContext::with_flush(FlushInfo::new(num_doc, 10))?);
        let state = SegmentWriteState::new(stream, dir.clone(), si, field_infos, None, context);
        consumer.flush(&state, Some(Rc::new(DummyDocMap)), &codec)?;
        assert_eq!(
            num_doc,
            consumer
                .sub
                .as_ref()
                .unwrap()
                .start_doc_counter
                .load(Ordering::SeqCst)
        );
        assert_eq!(
            num_doc,
            consumer
                .sub
                .as_ref()
                .unwrap()
                .finish_doc_counter
                .load(Ordering::SeqCst)
        );
        Ok(())
    }

    struct StoredFieldsConsumerImpl<D> {
        start_doc_counter: AtomicI32,
        finish_doc_counter: AtomicI32,
        dir: Arc<Mutex<D>>,
    }
    impl<D> StoredFieldsConsumerImpl<D>
    where
        D: Directory,
    {
        pub fn new(dir: Arc<Mutex<D>>) -> Self {
            Self {
                start_doc_counter: AtomicI32::new(0),
                finish_doc_counter: AtomicI32::new(0),
                dir,
            }
        }
    }
    impl<D> StoredFieldsConsumerBase for StoredFieldsConsumerImpl<D>
    where
        D: Directory,
    {
        type TempDirectory = D;

        fn init_stored_fields_writer(
            &mut self,
            _info: Rc<SegmentInfo<Self::Directory>>,
        ) -> Result<StoredFieldsWriterEnum<Self::TempDirectory>> {
            Err(LuceneError::not_implemented(""))
        }

        fn start_document(&mut self, _codec: &impl Codec, _doc_id: i32) -> Result<()> {
            self.start_doc_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn finish_document(&mut self) -> Result<()> {
            self.finish_doc_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        type Directory = D;

        fn flush<DM>(
            &mut self,
            _state: &SegmentWriteState<Self::Directory>,
            _sort_map: Option<Rc<DM>>,
            _codec: &impl Codec,
        ) -> Result<()>
        where
            DM: DocMap,
        {
            Err(LuceneError::not_implemented(""))
        }

        fn abort(&mut self) -> Result<()> {
            Err(LuceneError::not_implemented(""))
        }
    }
}
