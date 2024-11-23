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
use crate::doc_id_set::DocIdSet;
use crate::{DocIdSetIterator, MatchNoBits, NO_MORE_DOCS};
use std::cmp::min;
use std::rc::Rc;

// TODO
#[allow(dead_code)]
const BASE_RAM_BYTES_USED: i64 = 0;

/**
 * A doc id set based on sorted int array.
 */
pub struct IntArrayDocIdSet {
    docs: Vec<i32>,
    length: i32,
}
/**
 * Build an IntArrayDocIdSet by an int array and len.
 *
 * param docs A docs array whose length need to be greater than the param len. It needs to be
 *     sorted from 0(inclusive) to the len(exclusive), and the len-th doc in docs need to be
 *     DocIdSetIterator#NO_MORE_DOCS.
 * param len The valid docs length in array.
 */
impl IntArrayDocIdSet {
    pub fn new(docs: Vec<i32>, length: i32) -> Result<IntArrayDocIdSet, String> {
        if docs[length as usize] != NO_MORE_DOCS {
            return Err(format!("last value must be {}", NO_MORE_DOCS));
        }
        let ordered = docs.windows(2).all(|w| w[0] < w[1]);
        if !ordered {
            return Err(format!(
                "IntArrayDocIdSet need docs to be sorted:{}",
                docs.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ));
        }
        Ok(IntArrayDocIdSet { docs, length })
    }
}

impl DocIdSet for IntArrayDocIdSet {
    type DISIType<'a> = IntArrayDocIdSetIterator<'a>;

    fn iterator(&self) -> Option<Self::DISIType<'_>> {
        Some(IntArrayDocIdSetIterator::new(&self.docs, self.length))
    }

    type BitType = MatchNoBits;

    fn bits(&self) -> Option<Rc<Self::BitType>> {
        None
    }
}

impl Accountable for IntArrayDocIdSet {
    fn ram_bytes_used(&self) -> i64 {
        todo!()
    }
}

pub struct IntArrayDocIdSetIterator<'a> {
    docs: &'a Vec<i32>,
    length: i32,
    i: i32,
    doc: i32,
}
impl<'a> IntArrayDocIdSetIterator<'a> {
    pub fn new(docs: &'a Vec<i32>, length: i32) -> IntArrayDocIdSetIterator {
        IntArrayDocIdSetIterator {
            docs,
            length,
            i: 0,
            doc: -1,
        }
    }
}
impl<'a> DocIdSetIterator for IntArrayDocIdSetIterator<'a> {
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> i32 {
        self.doc = self.docs[self.i as usize];
        self.i += 1;
        self.doc
    }

    fn advance(&mut self, target: i32) -> i32 {
        let mut bound = 1;
        // given that we use this for small arrays only, this is very unlikely to overflow
        while (self.i + bound < self.length)
            && (self.docs[self.i as usize + bound as usize] < target)
        {
            bound *= 2;
        }
        let mut start = self.i as usize + (bound / 2) as usize;
        let end = min(self.i + bound + 1, self.length - 1) as usize;
        let index = self.docs[start..end]
            .binary_search(&target)
            .unwrap_or_else(|index| index);
        start += index;
        self.doc = self.docs[start];
        self.i = start as i32 + 1;
        self.doc
    }

    fn cost(&self) -> i64 {
        self.length as i64
    }
}
