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
use crate::accountable::Accountable;
use crate::bit_sets::bit_set::BitSet;
use crate::bit_sets::fixed_bit_set::FixedBitSet;
use crate::bit_sets::sparse_fixed_bit_set::SparseFixedBitSet;
use crate::util::error::runtime_error::RuntimeError;
use crate::{Bits, DocIdSetIterator};

pub enum BitSetType {
    Sparse(SparseFixedBitSet),
    Fixed(FixedBitSet),
}

impl Bits for BitSetType {
    fn get(&self, index: i32) -> bool {
        match self {
            Self::Sparse(s) => s.get(index),
            Self::Fixed(s) => s.get(index),
        }
    }

    fn length(&self) -> i32 {
        match self {
            BitSetType::Sparse(s) => s.length(),
            BitSetType::Fixed(f) => f.length(),
        }
    }
}

impl Accountable for BitSetType {
    fn ram_bytes_used(&self) -> i64 {
        match self {
            BitSetType::Sparse(s) => s.ram_bytes_used(),
            BitSetType::Fixed(f) => f.ram_bytes_used(),
        }
    }
}

impl BitSet for BitSetType {
    fn clear(&mut self) {
        match self {
            Self::Sparse(s) => s.clear(),
            Self::Fixed(fixed) => fixed.clear(),
        }
    }

    fn set(&mut self, i: i32) {
        match self {
            Self::Sparse(s) => s.set(i),
            Self::Fixed(fixed) => fixed.set(i),
        }
        todo!()
    }

    fn get_and_set(&mut self, i: i32) -> bool {
        match self {
            Self::Sparse(s) => s.get_and_set(i),
            Self::Fixed(fixed) => fixed.get_and_set(i),
        }
    }

    fn clear_with_index(&mut self, i: i32) {
        match self {
            Self::Sparse(s) => s.clear_with_index(i),
            Self::Fixed(fixed) => fixed.clear_with_index(i),
        }
    }

    fn clear_range(&mut self, start_index: i32, end_index: i32) {
        match self {
            Self::Sparse(s) => s.clear_range(start_index, end_index),
            Self::Fixed(fixed) => fixed.clear_range(start_index, end_index),
        }
    }

    fn cardinality(&self) -> i32 {
        match self {
            Self::Sparse(s) => s.cardinality(),
            Self::Fixed(fixed) => fixed.cardinality(),
        }
    }

    fn approximate_cardinality(&self) -> i32 {
        match self {
            Self::Sparse(s) => s.approximate_cardinality(),
            Self::Fixed(fixed) => fixed.approximate_cardinality(),
        }
    }

    fn prev_set_bit(&self, index: i32) -> i32 {
        match self {
            Self::Sparse(s) => s.prev_set_bit(index),
            Self::Fixed(fixed) => fixed.prev_set_bit(index),
        }
    }

    fn next_set_bit(&self, index: i32) -> i32 {
        match self {
            Self::Sparse(s) => s.next_set_bit(index),
            Self::Fixed(fixed) => fixed.next_set_bit(index),
        }
    }

    fn next_set_bit_range(&self, start: i32, end: i32) -> i32 {
        match self {
            Self::Sparse(s) => s.next_set_bit_range(start, end),
            Self::Fixed(fixed) => fixed.next_set_bit_range(start, end),
        }
    }

    fn or<T: DocIdSetIterator>(&mut self, iter: T) -> Result<(), RuntimeError> {
        match self {
            Self::Sparse(s) => s.or(iter),
            Self::Fixed(fixed) => BitSet::or(fixed, iter),
        }
    }
}
