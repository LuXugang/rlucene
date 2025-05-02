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
use std::borrow::Cow;

use crate::util::automation::automaton::Automaton;
use crate::util::automation::finite_strings_iterator::FiniteStringsIterator;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::ints_ref::IntsRef;

pub struct LimitedFiniteStringsIterator<'a> {
    limit: i32,
    count: i32,
    base: FiniteStringsIterator<'a>,
}
impl<'a> LimitedFiniteStringsIterator<'a> {
    pub fn new(automaton: &'a Automaton, limit: i32) -> Result<Self> {
        if limit != -1 && limit <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "limit must be -1 (which means no limit), or > 0; got: {}",
                limit
            )));
        }

        Ok(Self {
            limit: if limit > 0 { limit } else { i32::MAX },
            count: 0,
            base: FiniteStringsIterator::new(automaton),
        })
    }

    /// Number of iterated finite strings so far
    pub fn size(&self) -> i32 {
        self.count
    }
    pub fn next(&mut self) -> Result<Option<Cow<IntsRef<Vec<i32>>>>> {
        if self.count >= self.limit {
            return Ok(None);
        }

        if let Some(result) = self.base.next()? {
            self.count += 1;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}
