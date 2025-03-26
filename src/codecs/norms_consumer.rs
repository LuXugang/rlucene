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
use crate::codecs::lucene90_norms_producer::NumericDocValuesEnum;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::field_info::FieldInfo;
use crate::index::merge_state::DocMapEnum;
use crate::index::SubBase;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;
use std::rc::Rc;
/// Consumes normalization values.
///
/// Concrete implementations actually do *something* with the norms,
/// such as writing them into the index in a specific format.
///
/// # Lifecycle
///
/// 1. `NormsConsumer` is created by [`NormsFormat::norms_consumer`](crate::codecs::norms_format::NormsFormat::norms_consumer).
/// 2. [`add_norms_field`](NormsConsumer::add_norms_field) is called for each field with normalization values.
///    The API is *pull*-based rather than *push*-based; the implementation is free
///    to iterate over the values multiple times.
/// 3. After all fields are added, the consumer is [`close`]d.
pub trait NormsConsumer {
    /// Writes normalization values for a field.
    ///
    /// # Arguments
    /// * `field` - Field metadata
    /// * `norms_producer` - Provides numeric norms for the field
    ///
    /// # Errors
    /// If an I/O error occurs during writing.
    fn add_norms_field<I>(
        &mut self,
        field: &Rc<FieldInfo>,
        norms_producer: &mut impl NormsProducer<I>,
    ) -> Result<()>
    where
        I: IndexInput;
}

/// Tracks state of one numeric sub-reader that we are merging.
struct NumericDocValuesSub<I>
where
    I: IndexInput,
{
    values: NumericDocValuesEnum<I>,
    doc_map: Rc<DocMapEnum>,
}
impl<I> NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn new(doc_map: Rc<DocMapEnum>, values: NumericDocValuesEnum<I>) -> Self {
        debug_assert!(values.doc_id() == -1);
        NumericDocValuesSub { values, doc_map }
    }
}
impl<I> SubBase for NumericDocValuesSub<I>
where
    I: IndexInput,
{
    fn next_doc(&mut self) -> Result<i32> {
        self.values.next_doc()
    }

    fn get_doc_map(&self) -> Result<&Rc<DocMapEnum>> {
        todo!()
    }
}

pub enum NormsConsumerEnum {}
