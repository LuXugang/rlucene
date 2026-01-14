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
use crate::core::codecs::DefaultPostingsFormat;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::fields::Fields;
use crate::core::util::error::lucene_error::Result;
use std::sync::Arc;
pub trait FieldsProducer: Fields + Clone {
    /// Checks consistency of this reader.
    ///
    /// Note that this may be costly in terms of I/O, e.g. may involve computing
    /// a checksum value against large data files.
    fn check_integrity(&self) -> Result<()>;
    /// Returns an instance optimized for merging. This instance may only be
    /// cloned # Note
    /// Returning None means returning itself.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        Ok(None)
    }
}
pub type DefaultFieldsProducer<I> = <DefaultPostingsFormat as PostingsFormat>::FieldsProducer<I>;

impl<T> FieldsProducer for Arc<T>
where
    T: FieldsProducer,
{
    fn check_integrity(&self) -> Result<()> {
        (**self).check_integrity()
    }

    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        let v = match (**self).get_merge_instance()? {
            Some(v) => Arc::new(v),
            None => return Ok(None),
        };
        Ok(Some(v))
    }
}
