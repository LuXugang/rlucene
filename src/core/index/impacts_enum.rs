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
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::Result;
use std::borrow::Cow;

/// Extension of `PostingsEnum` which also provides information about upcoming
/// impacts.
pub trait ImpactsEnum: PostingsEnum + ImpactsSource {}
macro_rules! define_impacts_enum_enum {
    (
        $enum_name:ident,
        $impacts_wrapper:ident, // ImpactsEnum2 / ImpactsEnum3 / ...
        [$($V:ident),+ $(,)?]
    ) => {
        pub enum $enum_name<$($V),+> {
            $($V($V)),+
        }

        impl<$($V),+> PostingsEnum for $enum_name<$($V),+>
        where
            $($V: ImpactsEnum,)+
        {
            fn freq(&mut self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.freq(),)+
                }
            }

            fn next_position(&mut self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.next_position(),)+
                }
            }

            fn start_offset(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.start_offset(),)+
                }
            }

            fn end_offset(&self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.end_offset(),)+
                }
            }

            fn get_payload(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
                match self {
                    $(Self::$V(t) => t.get_payload(),)+
                }
            }
        }

        impl<$($V),+> crate::core::search::doc_id_set_iterator::DocIdSetIteratorExtensions for $enum_name<$($V),+>
        where
            $($V: ImpactsEnum,)+
        {}
        impl<$($V),+> DocIdSetIterator for $enum_name<$($V),+>
        where
            $($V: ImpactsEnum,)+
        {
            fn doc_id(&self) -> i32 {
                match self {
                    $(Self::$V(t) => t.doc_id(),)+
                }
            }

            fn next_doc(&mut self) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.next_doc(),)+
                }
            }

            fn advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.advance(target),)+
                }
            }

            fn slow_advance(&mut self, target: i32) -> Result<i32> {
                match self {
                    $(Self::$V(t) => t.slow_advance(target),)+
                }
            }

            fn cost(&self) -> Result<i64> {
                match self {
                    $(Self::$V(t) => t.cost(),)+
                }
            }
        }

        impl<$($V),+> ImpactsSource for $enum_name<$($V),+>
        where
            $($V: ImpactsEnum,)+
        {
            fn advance_shallow(&mut self, target: i32) -> Result<()> {
                match self {
                    $(Self::$V(t) => t.advance_shallow(target),)+
                }
            }

            type Impacts<'a> = $impacts_wrapper<$($V::Impacts<'a>),+>
            where
                Self: 'a;

            fn get_impacts(&self) -> Result<Self::Impacts<'_>> {
                match self {
                    $(Self::$V(t) => Ok($impacts_wrapper::$V(t.get_impacts()?)),)+
                }
            }
        }

        impl<$($V),+> ImpactsEnum for $enum_name<$($V),+>
        where
            $($V: ImpactsEnum,)+
        {}
    };
}
define_impacts_enum_enum!(ImpactsEnumEnum2, ImpactsEnum2, [A, B]);
