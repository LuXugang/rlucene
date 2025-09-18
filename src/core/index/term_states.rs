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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::term::Term;
use crate::core::index::term_state::TermState;
use crate::core::util::error::lucene_error::Result;
use std::rc::Rc;

pub struct TermStates<TS>
where
    TS: TermState + Default,
{
    top_reader_context_identity: Rc<()>,
    states: Vec<TS>,
    term: Option<Rc<Term>>,
    doc_freq: i32,
    total_term_freq: i64,
}
impl<TS> TermStates<TS>
where
    TS: TermState + Default,
{
    pub fn new(term: Option<Rc<Term>>, context: &impl IndexReaderContext) -> Result<Self> {
        debug_assert!(context.base().is_top_level);

        let num_leaves = context.leaves()?.len();

        Ok(TermStates {
            top_reader_context_identity: context.base().identity.clone(),
            doc_freq: 0,
            total_term_freq: 0,
            states: vec![TS::default(); num_leaves],
            term,
        })
    }

    pub fn new_empty(context: &impl IndexReaderContext) -> Result<Self> {
        Self::new(None, context)
    }
    pub fn was_built_for(&self, context: &impl IndexReaderContext) -> bool {
        Rc::ptr_eq(&self.top_reader_context_identity, &context.base().identity)
    }

    pub fn register(&mut self, state: TS, ord: usize) {
        debug_assert!(ord < self.states.len(), "ord {} out of bounds", ord);
        self.states[ord] = state;
    }
    pub fn accumulate_statistics(&mut self, doc_freq: i32, total_term_freq: i64) {
        debug_assert!(doc_freq >= 0);
        debug_assert!(total_term_freq >= 0);
        debug_assert!(
            (doc_freq as i64) <= total_term_freq,
            "doc_freq must not exceed total_term_freq"
        );
        self.doc_freq += doc_freq;
        self.total_term_freq += total_term_freq;
    }
}
