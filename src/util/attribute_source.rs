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
use crate::index::BytesRef;
use std::borrow::Cow;

pub trait AttributeSource {
    // OffsetAttribute
    fn start_offset(&self) -> Option<i32> {
        None
    }

    fn end_offset(&self) -> Option<i32> {
        None
    }

    // PositionIncrementAttribute
    fn get_position_increment(&self) -> Option<i32> {
        None
    }

    // PayloadAttribute;
    fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
        None
    }

    // TermToBytesRefAttribute;
    fn get_bytes_ref(&self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
        None
    }

    // TermFrequencyAttribute;
    fn get_term_frequency(&self) -> Option<i32> {
        None
    }
}

pub struct EmptyAttributeSource;
impl AttributeSource for EmptyAttributeSource {}

// AttributeSource
pub enum EitherAttributeSource<F, S> {
    F(F),
    S(S),
}

impl<F, S> AttributeSource for EitherAttributeSource<F, S>
where
    F: AttributeSource,
    S: AttributeSource,
{
    fn start_offset(&self) -> Option<i32> {
        match self {
            EitherAttributeSource::F(t) => t.start_offset(),
            EitherAttributeSource::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Option<i32> {
        match self {
            EitherAttributeSource::F(t) => t.end_offset(),
            EitherAttributeSource::S(s) => s.end_offset(),
        }
    }

    fn get_position_increment(&self) -> Option<i32> {
        match self {
            EitherAttributeSource::F(t) => t.get_position_increment(),
            EitherAttributeSource::S(s) => s.get_position_increment(),
        }
    }

    fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
        match self {
            EitherAttributeSource::F(t) => t.get_payload(),
            EitherAttributeSource::S(s) => s.get_payload(),
        }
    }

    fn get_bytes_ref(&self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
        match self {
            EitherAttributeSource::F(t) => t.get_bytes_ref(),
            EitherAttributeSource::S(s) => s.get_bytes_ref(),
        }
    }

    fn get_term_frequency(&self) -> Option<i32> {
        match self {
            EitherAttributeSource::F(t) => t.get_term_frequency(),
            EitherAttributeSource::S(s) => s.get_term_frequency(),
        }
    }
}
