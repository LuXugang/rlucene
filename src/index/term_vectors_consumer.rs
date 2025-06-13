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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::sorter::DocMap;
use crate::index::term_vectors_consumer_per_field::TermVectorsConsumerPerField;
use crate::index::terms_hash::{TermsHash, TermsHashBase};
use crate::index::terms_hash_per_field::TermsHashPerField;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::Result;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct TermVectorsConsumer {
    pub(crate) base: TermsHash,
}
impl TermsHashBase for TermVectorsConsumer {
    fn abort(&mut self) {
        todo!()
    }

    type TermsHashPerFieldBase = TermVectorsConsumerPerField;

    fn flush<D, N, DM, O, P, T>(
        &mut self,
        fields_to_flush: HashMap<String, TermsHashPerField<Self::TermsHashPerFieldBase, O, P, T>>,
        state: &mut SegmentWriteState<D>,
        sort_map: Option<Rc<DM>>,
        norms: &mut N,
    ) -> Result<()>
    where
        D: Directory,
        N: NormsProducer,
        DM: DocMap,
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
    {
        todo!()
    }

    fn add_field<O, P, T>(
        &mut self,
        _field_invert_state: Rc<FieldInvertState<O, P, T>>,
        _field_info: Rc<FieldInfo>,
    ) -> TermsHashPerField<Self::TermsHashPerFieldBase, O, P, T>
    where
        O: OffsetAttribute,
        P: PayloadAttribute,
        T: TermFrequencyAttribute,
    {
        todo!()
    }

    fn start_document(&mut self) -> Result<()> {
        todo!()
    }

    fn finish_document(&mut self, _doc_id: i32) -> Result<()> {
        todo!()
    }
}
