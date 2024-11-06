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
use crate::{Bits, DocIdSetIterator, EmptyDISI, MatchAllBits, MatchNoBits};
use std::rc::Rc;

pub trait DocIdSet: Accountable {
    type DISIType<'a>: DocIdSetIterator + 'a
    where
        Self: 'a;
    fn iterator(&self) -> Option<Self::DISIType<'_>>;

    // TODO: somehow this class should express the cost of
    // iteration vs the cost of random access Bits; for
    // expensive Filters (e.g. distance < 1 km) we should use
    // bits() after all other Query/Filters have matched, but
    // this is the opposite of what bits() is for now
    // (down-low filtering using e.g. FixedBitSet)

    /**
     * Optionally provides a `Bits\ interface for random access to matching documents.
     *
     * return `None`, if this `DocIdSet` does not support random access. In contrast to
     * `iterator()`, a return value of `None` **does not** imply that no documents
     * match the filter! The default implementation does not provide random access, so you only
     * need to implement this method if your DocIdSet can guarantee random access to every doc id
     * in O(1) time without external disk access. This is generally true for bit sets like
     * `FixedBitSet`, which return itself if they are used as `DocIdSet`.
     */

    type BitType: Bits;
    fn bits(&self) -> Option<Rc<Self::BitType>>;
}

struct All {
    max_doc: i32,
    bits: Option<Rc<MatchAllBits>>,
}
impl All {
    fn new(max_doc: i32) -> Self {
        let bits = Some(Rc::new(MatchAllBits::new(max_doc)));
        All { max_doc, bits }
    }
}
/** A `DocIdSet` that matches all doc ids up to a specified doc (exclusive). */
impl DocIdSet for All {
    type DISIType<'a> = crate::AllDocIdSetIterator;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        Some(crate::AllDocIdSetIterator::new(self.max_doc))
    }

    type BitType = MatchAllBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        self.bits.clone()
    }
}

impl Accountable for All {
    fn ram_bytes_used(&self) -> i64 {
        std::mem::size_of::<i32>() as i64
    }
}

pub struct EmptyDocIdSet;
impl Accountable for EmptyDocIdSet {
    fn ram_bytes_used(&self) -> i64 {
        0
    }
}
impl DocIdSet for EmptyDocIdSet {
    type DISIType<'a> = EmptyDISI;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        None
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}
