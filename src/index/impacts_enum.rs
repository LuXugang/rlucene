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
use crate::index::impacts::EitherImpacts;
use crate::index::impacts_source::ImpactsSource;
use crate::index::postings_enum::PostingsEnum;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error;
use std::borrow::Cow;

/// Extension of `PostingsEnum` which also provides information about upcoming
/// impacts.
pub trait ImpactsEnum: PostingsEnum + ImpactsSource {}

pub enum EitherImpactsEnum<F, S> {
    F(F),
    S(S),
}

impl<F, S> PostingsEnum for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
    fn freq(&mut self) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.freq(),
            EitherImpactsEnum::S(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.next_position(),
            EitherImpactsEnum::S(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.start_offset(),
            EitherImpactsEnum::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.end_offset(),
            EitherImpactsEnum::S(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> lucene_error::Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        match self {
            EitherImpactsEnum::F(t) => t.get_payload(),
            EitherImpactsEnum::S(s) => s.get_payload(),
        }
    }
}

impl<F, S> DocIdSetIterator for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherImpactsEnum::F(t) => t.doc_id(),
            EitherImpactsEnum::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.next_doc(),
            EitherImpactsEnum::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.advance(target),
            EitherImpactsEnum::S(s) => s.advance(target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> lucene_error::Result<i32> {
        match self {
            EitherImpactsEnum::F(t) => t.slow_advance(target),
            EitherImpactsEnum::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> lucene_error::Result<i64> {
        match self {
            EitherImpactsEnum::F(t) => t.cost(),
            EitherImpactsEnum::S(s) => s.cost(),
        }
    }
}

impl<F, S> ImpactsSource for EitherImpactsEnum<F, S>
where
    S: ImpactsEnum,
    F: ImpactsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> lucene_error::Result<()> {
        match self {
            EitherImpactsEnum::F(t) => t.advance_shallow(target),
            EitherImpactsEnum::S(s) => s.advance_shallow(target),
        }
    }

    type Impacts = EitherImpacts<F::Impacts, S::Impacts>;

    fn get_impacts(&mut self) -> lucene_error::Result<Self::Impacts> {
        match self {
            EitherImpactsEnum::F(t) => {
                let impacts = t.get_impacts()?;
                Ok(EitherImpacts::F(impacts))
            },
            EitherImpactsEnum::S(s) => {
                let impacts = s.get_impacts()?;
                Ok(EitherImpacts::S(impacts))
            },
        }
    }
}

impl<F, S> ImpactsEnum for EitherImpactsEnum<F, S>
where
    F: ImpactsEnum,
    S: ImpactsEnum,
{
}
