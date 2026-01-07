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
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::packed::direct_reader::FromSlice;

/// Abstraction over an array of longs.
pub trait LongValues {
    fn get_mut(&mut self, _index: usize) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "mutable get method not implemented",
        ))
    }

    /// Add an extra, immutable version of the method.
    /// If you need to call get in an immutable context, you can implement this method.
    fn get(&self, _index: usize) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "Immutable get method not implemented",
        ))
    }
}

pub struct Identity;
impl LongValues for Identity {
    fn get_mut(&mut self, index: usize) -> Result<i64> {
        Ok(index as i64)
    }

    fn get(&self, index: usize) -> Result<i64> {
        Ok(index as i64)
    }
}

pub struct Zeroes;
impl LongValues for Zeroes {
    fn get_mut(&mut self, _index: usize) -> Result<i64> {
        Ok(0)
    }

    fn get(&self, _index: usize) -> Result<i64> {
        Ok(0)
    }
}
impl<R> FromSlice<R> for Zeroes
where
    R: RandomAccessInput,
{
    fn read_from_slice(&mut self, _index: usize, _slice: Option<&mut R>) -> Result<i64> {
        Ok(0)
    }
}

macro_rules! either_long_values {
    ($vis:vis $name:ident { $( $Variant:ident : $T:ident ),+ $(,)? }) => {
        $vis enum $name<$( $T ),+> {
            $( $Variant($T), )+
        }

        impl<$( $T ),+> LongValues for $name<$( $T ),+>
        where
            $( $T: LongValues ),+
        {
            fn get_mut(&mut self, index: usize) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.get_mut(index), )+
                }
            }

            fn get(&self, index: usize) -> Result<i64> {
                match self {
                    $( Self::$Variant(inner) => inner.get(index), )+
                }
            }
        }
    };
}
either_long_values!(pub LongValuesEnum2 { A: A, B: B });
either_long_values!(pub LongValuesEnum3 { A: A, B: B, C:C });
either_long_values!(pub LongValuesEnum5 { A:A,B:B,C:C,D:D,E:E });
either_long_values!(pub LongValuesEnum16 { A:A,B:B,C:C,D:D,E:E,F:F,G:G,H:H,I:I,J:J,K:K,L:L,M:M,N:N,O:O,P:P});
