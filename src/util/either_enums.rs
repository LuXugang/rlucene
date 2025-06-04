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
use crate::index::impact::Impact;
use crate::index::impacts::Impacts;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::impacts_source::ImpactsSource;
use crate::index::postings_enum::PostingsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
/// # Either Enums for Unified Trait Implementations
///
/// This module defines `EitherXXX` enums that act as static-dispatch-compatible
/// wrappers over two different types implementing the same trait.
///
/// These enums are useful when an algorithm or component can operate over
/// multiple implementations of a common trait (e.g., `PostingsEnum`, `ImpactsEnum`),
/// and a concrete type is needed without resorting to dynamic dispatch.
///
/// ## Examples
/// - [`EitherPostingsEnum<T, S>`] wraps two types implementing `PostingsEnum`.
/// - [`EitherImpactsEnum<T, S>`] wraps two types implementing `ImpactsEnum`.
/// - [`EitherImpacts`] wraps two types implementing `Impacts`.
///
/// Each enum forwards all trait method calls to the underlying variant,
/// enabling seamless use in performance-critical paths without heap allocation or virtual dispatch.
///
/// This approach avoids the overhead of `Box<dyn Trait>` and keeps all behavior
/// statically resolved by the compiler.
// ImpactsEnum
pub enum EitherImpactsEnum<T, S> {
    T(T),
    S(S),
}

impl<T, S> PostingsEnum for EitherImpactsEnum<T, S>
where
    T: ImpactsEnum,
    S: ImpactsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.freq(),
            EitherImpactsEnum::S(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.next_position(),
            EitherImpactsEnum::S(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.start_offset(),
            EitherImpactsEnum::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.end_offset(),
            EitherImpactsEnum::S(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        match self {
            EitherImpactsEnum::T(t) => t.get_payload(),
            EitherImpactsEnum::S(s) => s.get_payload(),
        }
    }
}

impl<T, S> DocIdSetIterator for EitherImpactsEnum<T, S>
where
    T: ImpactsEnum,
    S: ImpactsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherImpactsEnum::T(t) => t.doc_id(),
            EitherImpactsEnum::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.next_doc(),
            EitherImpactsEnum::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.advance(_target),
            EitherImpactsEnum::S(s) => s.advance(_target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherImpactsEnum::T(t) => t.slow_advance(target),
            EitherImpactsEnum::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherImpactsEnum::T(t) => t.cost(),
            EitherImpactsEnum::S(s) => s.cost(),
        }
    }
}

impl<T, S> ImpactsSource for EitherImpactsEnum<T, S>
where
    S: ImpactsEnum,
    T: ImpactsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> Result<()> {
        match self {
            EitherImpactsEnum::T(t) => t.advance_shallow(target),
            EitherImpactsEnum::S(s) => s.advance_shallow(target),
        }
    }

    type ImpactsType<'a>
    where
        Self: 'a,
    = EitherImpacts<T::ImpactsType<'a>, S::ImpactsType<'a>>;

    fn get_impacts(&mut self) -> Result<Self::ImpactsType<'_>> {
        match self {
            EitherImpactsEnum::T(t) => {
                let impacts = t.get_impacts()?;
                Ok(EitherImpacts::T(impacts))
            },
            EitherImpactsEnum::S(s) => {
                let impacts = s.get_impacts()?;
                Ok(EitherImpacts::S(impacts))
            },
        }
    }
}

impl<T, S> ImpactsEnum for EitherImpactsEnum<T, S>
where
    T: ImpactsEnum,
    S: ImpactsEnum,
{
}

// Impacts
pub enum EitherImpacts<T, S>
where
    T: Impacts,
    S: Impacts,
{
    T(T),
    S(S),
}
impl<T, S> Impacts for EitherImpacts<T, S>
where
    T: Impacts,
    S: Impacts,
{
    fn num_levels(&self) -> i32 {
        match self {
            EitherImpacts::T(t) => t.num_levels(),
            EitherImpacts::S(s) => s.num_levels(),
        }
    }

    fn get_doc_id_up_to(&self, level: i32) -> i32 {
        match self {
            EitherImpacts::T(t) => t.get_doc_id_up_to(level),
            EitherImpacts::S(s) => s.get_doc_id_up_to(level),
        }
    }

    fn get_impacts(&mut self, level: i32) -> Result<&[Impact]> {
        match self {
            EitherImpacts::T(t) => t.get_impacts(level),
            EitherImpacts::S(s) => s.get_impacts(level),
        }
    }
}

// PostingsEnum
pub enum EitherPostingsEnum<T, S> {
    T(T),
    S(S),
}

impl<T, S> DocIdSetIterator for EitherPostingsEnum<T, S>
where
    T: PostingsEnum,
    S: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        match self {
            EitherPostingsEnum::T(t) => t.doc_id(),
            EitherPostingsEnum::S(s) => s.doc_id(),
        }
    }

    fn next_doc(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.next_doc(),
            EitherPostingsEnum::S(s) => s.next_doc(),
        }
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.advance(_target),
            EitherPostingsEnum::S(s) => s.advance(_target),
        }
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.slow_advance(target),
            EitherPostingsEnum::S(s) => s.slow_advance(target),
        }
    }

    fn cost(&self) -> Result<i64> {
        match self {
            EitherPostingsEnum::T(t) => t.cost(),
            EitherPostingsEnum::S(s) => s.cost(),
        }
    }
}

impl<T, S> PostingsEnum for EitherPostingsEnum<T, S>
where
    T: PostingsEnum,
    S: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.freq(),
            EitherPostingsEnum::S(s) => s.freq(),
        }
    }

    fn next_position(&mut self) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.next_position(),
            EitherPostingsEnum::S(s) => s.next_position(),
        }
    }

    fn start_offset(&self) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.start_offset(),
            EitherPostingsEnum::S(s) => s.start_offset(),
        }
    }

    fn end_offset(&self) -> Result<i32> {
        match self {
            EitherPostingsEnum::T(t) => t.end_offset(),
            EitherPostingsEnum::S(s) => s.end_offset(),
        }
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        match self {
            EitherPostingsEnum::T(t) => t.get_payload(),
            EitherPostingsEnum::S(s) => s.get_payload(),
        }
    }
}
