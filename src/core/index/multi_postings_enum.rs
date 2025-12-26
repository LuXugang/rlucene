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
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::reader_slice::ReaderSlice;
use std::rc::Rc;

pub struct MultiPostingsEnum;
/// Holds a [`PostingsEnum`] along with the corresponding [`ReaderSlice`].
pub struct EnumWithSlice<PE>
where
    PE: PostingsEnum,
{
    /// [`PostingsEnum`] for this sub-reader
    postings_enum: Option<PE>,
    /// [`ReaderSlice`] describing how this sub-reader fits into the composite reader.
    slice: Rc<ReaderSlice>,
}
impl<PE> EnumWithSlice<PE>
where
    PE: PostingsEnum,
{
    /// Creates a new `EnumWithSlice`.
    pub fn new() -> Self {
        Self {
            postings_enum: None,
            slice: Rc::new(ReaderSlice::default()),
        }
    }
}
impl<PE> Default for EnumWithSlice<PE>
where
    PE: PostingsEnum,
{
    fn default() -> Self {
        Self::new()
    }
}
