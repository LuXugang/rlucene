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
use std::rc::Rc;

use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::terms_hash_per_field::{TermsHashPerField, TermsHashPerFieldBase};
use crate::util::error::lucene_error::Result;
/// This struct receives each token produced by the analyzer on each field
/// during indexing, and stores them in a hash table. It also allocates separate
/// byte streams per token.
///
/// Consumers of this struct, such as [`FreqProxTermsWriter`] and
/// [`TermVectorsConsumer`], write their own byte streams associated with each
/// term.
pub(crate) trait TermsHashBase {
    fn abort(&mut self);
    type TermsHashPerFieldBase: TermsHashPerFieldBase;
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
        unimplemented!("This method should be implemented by the specific TermsHashPerField type.");
    }

    fn start_document(&mut self) -> Result<()>;

    fn finish_document(&mut self, doc_id: i32) -> Result<()>;
}
