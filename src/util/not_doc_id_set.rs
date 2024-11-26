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
use crate::search::doc_id_set::DocIdSet;
use crate::search::doc_id_set_iterator::{DocIdSetIterator, NO_MORE_DOCS};
use crate::util::accountable::Accountable;
use crate::util::bits::Bits;
use std::rc::Rc;

#[allow(dead_code)]
const BASE_RAM_BYTES_USED: i64 = 0;
/**
 * This DocIdSet encodes the negation of another DocIdSet. It is cacheable and
 * supports random-access if the underlying set is cacheable and supports random-access.
 *
 */
pub struct NotDocIdSet<T>
where
    T: DocIdSet,
{
    max_doc: i32,
    set: T,
}

impl<T> NotDocIdSet<T>
where
    T: DocIdSet,
{
    pub fn new(max_doc: i32, set: T) -> Self {
        NotDocIdSet { max_doc, set }
    }
}

impl<T> Accountable for NotDocIdSet<T>
where
    T: DocIdSet,
{
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

impl<T> DocIdSet for NotDocIdSet<T>
where
    T: DocIdSet,
{
    type DISIType<'a> = NotDocDocIdSetIterator<T::DISIType<'a>> where Self: 'a;

    type BitType = NotDocIdBits<T::BitType>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        NotDocDocIdSetIterator::new(self.set.iterator(), self.max_doc)
    }

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        self.set
            .bits()
            .map(|in_bit_rc| Rc::new(NotDocIdBits::new(in_bit_rc)))
    }
}

pub struct NotDocIdBits<B: Bits> {
    in_bit: Rc<B>,
}

impl<B: Bits> NotDocIdBits<B> {
    pub fn new(in_bits: Rc<B>) -> NotDocIdBits<B> {
        NotDocIdBits { in_bit: in_bits }
    }
}

impl<B: Bits> Bits for NotDocIdBits<B> {
    fn get(&self, index: i32) -> bool {
        !self.in_bit.get(index)
    }

    fn length(&self) -> i32 {
        self.in_bit.length()
    }
}

pub struct NotDocDocIdSetIterator<D: DocIdSetIterator> {
    in_iterator: D,
    doc: i32,
    next_skipped_doc: i32,
    max_doc: i32,
}

impl<D: DocIdSetIterator> NotDocDocIdSetIterator<D> {
    fn new(in_iterator: Option<D>, max_doc: i32) -> Option<Self> {
        in_iterator.map(|iterator| NotDocDocIdSetIterator {
            in_iterator: iterator,
            doc: -1,
            next_skipped_doc: -1,
            max_doc,
        })
    }
}

impl<D: DocIdSetIterator> DocIdSetIterator for NotDocDocIdSetIterator<D> {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> i32 {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> i32 {
        self.doc = target;
        if self.doc > self.next_skipped_doc {
            self.next_skipped_doc = self.in_iterator.advance(self.doc);
        }
        loop {
            if self.doc >= self.max_doc {
                self.doc = NO_MORE_DOCS;
                break;
            }
            debug_assert!(self.doc <= self.next_skipped_doc);
            if self.doc != self.next_skipped_doc {
                return self.doc;
            }
            self.doc += 1;
            self.next_skipped_doc = self.in_iterator.next_doc();
        }
        self.doc
    }

    fn cost(&self) -> i64 {
        self.max_doc as i64
    }
}
