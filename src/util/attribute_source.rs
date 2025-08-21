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
pub enum Either2AttributeSource<A, B> {
    A(A),
    B(B),
}

impl<A, B> AttributeSource for Either2AttributeSource<A, B>
where
    A: AttributeSource,
    B: AttributeSource,
{
    fn start_offset(&self) -> Option<i32> {
        match self {
            Either2AttributeSource::A(t) => t.start_offset(),
            Either2AttributeSource::B(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Option<i32> {
        match self {
            Either2AttributeSource::A(t) => t.end_offset(),
            Either2AttributeSource::B(s) => s.end_offset(),
        }
    }

    fn get_position_increment(&self) -> Option<i32> {
        match self {
            Either2AttributeSource::A(t) => t.get_position_increment(),
            Either2AttributeSource::B(s) => s.get_position_increment(),
        }
    }

    fn get_payload(&self) -> Option<&BytesRef<Vec<u8>>> {
        match self {
            Either2AttributeSource::A(t) => t.get_payload(),
            Either2AttributeSource::B(s) => s.get_payload(),
        }
    }

    fn get_bytes_ref(&self) -> Option<Cow<'_, BytesRef<Vec<u8>>>> {
        match self {
            Either2AttributeSource::A(t) => t.get_bytes_ref(),
            Either2AttributeSource::B(s) => s.get_bytes_ref(),
        }
    }

    fn get_term_frequency(&self) -> Option<i32> {
        match self {
            Either2AttributeSource::A(t) => t.get_term_frequency(),
            Either2AttributeSource::B(s) => s.get_term_frequency(),
        }
    }
}
