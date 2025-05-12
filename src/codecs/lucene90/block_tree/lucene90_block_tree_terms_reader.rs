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
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::codecs::lucene90::block_tree::field_reader::FieldReader;
use crate::codecs::postings_reader_base::PostingsReaderBase;
use crate::index::field_infos::FieldInfos;
use crate::store::IndexInput;

pub struct Lucene90BlockTreeTermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    terms_reader: Rc<RefCell<TermsReader<I, P>>>,
    field_reader: FieldMapWrapper<I, P>,
}
pub struct FieldMapWrapper<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    field_map: HashMap<i32, FieldReader<I, P>>,
}
pub struct TermsReader<I, P>
where
    I: IndexInput,
    P: PostingsReaderBase,
{
    pub(crate) terms_in: I,
    index_in: I,
    pub(crate) postings_reader: P,
    field_infos: Rc<FieldInfos>,
    field_list: Vec<String>,
    pub(crate) segment: String,
    pub(crate) version: i32,
}
pub mod lucene90_bttr_util {
    use std::rc::Rc;

    use crate::index::BytesRef;
    use crate::util::fst_impl::byte_sequence_outputs::ByteSequenceOutputs;
    use crate::util::fst_impl::outputs::Outputs;

    pub(crate) const OUTPUT_FLAGS_NUM_BITS: i32 = 2;
    pub(crate) const OUTPUT_FLAGS_MASK: i32 = 0x3;
    pub(crate) const OUTPUT_FLAG_IS_FLOOR: i32 = 0x1;
    pub(crate) const OUTPUT_FLAG_HAS_TERMS: i32 = 0x2;

    /// Extension of terms file
    pub(crate) const TERMS_EXTENSION: &str = "tim";
    pub(crate) const TERMS_CODEC_NAME: &str = "BlockTreeTermsDict";
    /// Initial terms format
    pub const VERSION_START: i32 = 0;
    /// Version that encodes output as MSB VLong for better FST sharing
    /// (GITHUB#12620)
    pub const VERSION_MSB_VLONG_OUTPUT: i32 = 1;
    /// Version that specializes arc store for continuous label in FST
    pub const VERSION_FST_CONTINUOUS_ARCS: i32 = 2;
    /// Current terms format version
    pub const VERSION_CURRENT: i32 = VERSION_FST_CONTINUOUS_ARCS;
    /// Extension of terms index file
    pub(crate) const TERMS_INDEX_EXTENSION: &str = "tip";
    pub(crate) const TERMS_INDEX_CODEC_NAME: &str = "BlockTreeTermsIndex";
    /// Extension of terms meta file
    pub(crate) const TERMS_META_EXTENSION: &str = "tmd";
    pub(crate) const TERMS_META_CODEC_NAME: &str = "BlockTreeTermsMeta";
    thread_local! {
        pub(crate) static NO_OUTPUT:BytesRef<Rc<Vec<u8>>> ={let v = ByteSequenceOutputs::get(); v.get_no_output()};
    }
}
