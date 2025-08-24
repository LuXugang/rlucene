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
use crate::codecs::compound_directory::CompoundDirectory;
use crate::codecs::doc_values_format::DocValuesFormat;
use crate::codecs::lucene90_compound_reader::Lucene90CompoundReader;
use crate::codecs::lucene90_doc_values_producer::Lucene90DocValuesProducer;
use crate::codecs::{Codec, get_default_code};
use crate::index::field_infos::FieldInfos;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_core_readers::CfsOrBaseInput;
use crate::index::segment_read_state::SegmentReadState;
use crate::store::IOContext;
use crate::store::directory::{Directory, Either2Directory};
use crate::util::error::lucene_error::Result;
use crate::util::ref_count::RefCount;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::rc::Rc;
/// Manages the [`DocValuesProducer`](crate::codecs::doc_values_producer::DocValuesProducer) held by [`SegmentReader`](crate::index::segment_reader::SegmentReader) and keeps track of their reference counting.
pub(crate) struct SegmentDocValues<D>
where
    D: Directory,
{
    inner: Mutex<Inner<D>>,
}
pub(crate) struct Inner<D>
where
    D: Directory,
{
    gen_dv_producers: HashMap<i64, RefCount<Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>>>,
}

impl<D> SegmentDocValues<D>
where
    D: Directory,
{
    pub(crate) fn new() -> Self {
        SegmentDocValues {
            inner: Mutex::new(Inner {
                gen_dv_producers: HashMap::new(),
            }),
        }
    }
    pub(crate) fn new_doc_values_producer(
        &self,
        si: &SegmentCommitInfo<D>,
        dir: &mut CompoundDirectory<Lucene90CompoundReader<D>>,
        r#gen: i64,
        infos: Rc<FieldInfos>,
    ) -> Result<RefCount<Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>>>
    where
        D: Directory,
    {
        let mut dv_dir = Either2Directory::A(dir);
        let mut segment_suffix = "".to_string();

        let base_dir = &mut *si.info.dir.lock();
        if r#gen != -1 {
            // gen'd files are written outside CFS, so use SegInfo directory
            dv_dir = Either2Directory::B(base_dir);
            segment_suffix = format!("{:x}", r#gen);
        }

        let io_context = IOContext::default_io_context()?;
        // set SegmentReadState to list only the fields that are relevant to that gen
        let srs = SegmentReadState::with_suffix(&mut dv_dir, infos, &io_context, &segment_suffix);

        let dv_format = get_default_code().doc_values_format();

        Ok(RefCount::new(Rc::new(
            dv_format.fields_producer(&srs, &si.info)?,
        )))
    }
    /// Returns the [`DocValuesProducer`](crate::codecs::doc_values_producer::DocValuesProducer) for the given generation.
    pub(crate) fn get_doc_values_producer(
        &self,
        r#gen: i64,
        si: &SegmentCommitInfo<D>,
        dir: &mut CompoundDirectory<Lucene90CompoundReader<D>>,
        infos: Rc<FieldInfos>,
    ) -> Result<Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>> {
        let mut inner = self.inner.lock();

        if let Some(dvp) = inner.gen_dv_producers.get_mut(&r#gen) {
            dvp.inc_ref();
            Ok(dvp.get().clone())
        } else {
            let dvp = self.new_doc_values_producer(si, dir, r#gen, infos)?;
            let v = dvp.get().clone();
            inner.gen_dv_producers.insert(r#gen, dvp);
            Ok(v)
        }
    }
    ///  Decrement the reference count of the given [`DocValuesProducer`](crate::codecs::doc_values_producer::DocValuesProducer) generations.
    pub(crate) fn dec_ref(&self, gens: &[i64]) -> Result<()> {
        let mut inner = self.inner.lock();

        for &r#gen in gens {
            if let Some(dvp) = inner.gen_dv_producers.get_mut(&r#gen) {
                if dvp.dec_ref()? {
                    inner.gen_dv_producers.remove(&r#gen);
                }
            } else {
                debug_assert!(false, "gen={} not found in gen_dv_producers", r#gen);
            }
        }
        Ok(())
    }
}
