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
use crate::core::index::BytesRef;
use crate::core::index::impacts::ImpactsEnum2;
use crate::core::index::impacts_source::ImpactsSource;
use crate::core::index::postings_enum::PostingsEnum;
use crate::core::index::slow_impacts_enum::DummyImpacts;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error;
use crate::core::util::error::lucene_error::LuceneError;
use std::borrow::Cow;

/// Extension of `PostingsEnum` which also provides information about upcoming
/// impacts.
pub trait ImpactsEnum: PostingsEnum + ImpactsSource {}

pub enum ImpactsEnumEnum2<A, B>
where
    A: ImpactsEnum,
    B: ImpactsEnum,
{
    A(A),
    B(B),
}

impl<A, B> PostingsEnum for ImpactsEnumEnum2<A, B>
where
    A: ImpactsEnum,
    B: ImpactsEnum,
{
    fn freq(&mut self) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.freq(),
            ImpactsEnumEnum2::B(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.next_position(),
            ImpactsEnumEnum2::B(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.start_offset(),
            ImpactsEnumEnum2::B(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.end_offset(),
            ImpactsEnumEnum2::B(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> lucene_error::Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        match self {
            ImpactsEnumEnum2::A(t) => t.get_payload(),
            ImpactsEnumEnum2::B(s) => s.get_payload(),
        }
    }
}

impl<A, B> DocIdSetIterator for ImpactsEnumEnum2<A, B>
where
    A: ImpactsEnum,
    B: ImpactsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            ImpactsEnumEnum2::A(t) => t.doc_id(),
            ImpactsEnumEnum2::B(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.next_doc(),
            ImpactsEnumEnum2::B(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.advance(target),
            ImpactsEnumEnum2::B(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        match self {
            ImpactsEnumEnum2::A(t) => t.slow_advance(target),
            ImpactsEnumEnum2::B(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> lucene_error::Result<i64> {
        match self {
            ImpactsEnumEnum2::A(t) => t.cost(),
            ImpactsEnumEnum2::B(s) => s.cost(),
        }
    }
}

impl<A, B> ImpactsSource for ImpactsEnumEnum2<A, B>
where
    B: ImpactsEnum,
    A: ImpactsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> lucene_error::Result<()> {
        match self {
            ImpactsEnumEnum2::A(t) => t.advance_shallow(target),
            ImpactsEnumEnum2::B(s) => s.advance_shallow(target),
        }
    }

    type Impacts<'a>
        = ImpactsEnum2<A::Impacts<'a>, B::Impacts<'a>>
    where
        Self: 'a;

    fn get_impacts(&self) -> lucene_error::Result<Self::Impacts<'_>> {
        match self {
            ImpactsEnumEnum2::A(t) => {
                let impacts = t.get_impacts()?;
                Ok(ImpactsEnum2::A(impacts))
            },
            ImpactsEnumEnum2::B(s) => {
                let impacts = s.get_impacts()?;
                Ok(ImpactsEnum2::B(impacts))
            },
        }
    }
}

impl<A, B> ImpactsEnum for ImpactsEnumEnum2<A, B>
where
    A: ImpactsEnum,
    B: ImpactsEnum,
{
}

pub struct ImpactsEnumNoImpactsSource<PE>
where
    PE: PostingsEnum,
{
    pub(crate) postings_enum: PE,
}

impl<PE> PostingsEnum for ImpactsEnumNoImpactsSource<PE>
where
    PE: PostingsEnum,
{
    fn freq(&mut self) -> lucene_error::Result<i32> {
        self.postings_enum.freq()
    }

    fn next_position(&mut self) -> lucene_error::Result<i32> {
        self.postings_enum.next_position()
    }

    fn start_offset(&self) -> lucene_error::Result<i32> {
        self.postings_enum.start_offset()
    }

    fn end_offset(&self) -> lucene_error::Result<i32> {
        self.postings_enum.end_offset()
    }

    fn get_payload(&self) -> lucene_error::Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.postings_enum.get_payload()
    }
}

impl<PE> DocIdSetIterator for ImpactsEnumNoImpactsSource<PE>
where
    PE: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        self.postings_enum.doc_id()
    }

    fn next_doc(&mut self) -> lucene_error::Result<i32> {
        self.postings_enum.next_doc()
    }

    fn advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        self.postings_enum.advance(target)
    }

    fn slow_advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        self.postings_enum.slow_advance(target)
    }

    fn cost(&self) -> lucene_error::Result<i64> {
        self.postings_enum.cost()
    }
}

impl<PE> ImpactsSource for ImpactsEnumNoImpactsSource<PE>
where
    PE: PostingsEnum,
{
    fn advance_shallow(&mut self, _target: i32) -> lucene_error::Result<()> {
        Err(LuceneError::unsupported_operation(""))
    }

    type Impacts<'a>
        = DummyImpacts
    where
        Self: 'a;

    fn get_impacts(&self) -> lucene_error::Result<Self::Impacts<'_>> {
        Err(LuceneError::unsupported_operation(""))
    }
}

impl<PE> ImpactsEnum for ImpactsEnumNoImpactsSource<PE> where PE: PostingsEnum {}
