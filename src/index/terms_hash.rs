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
use crate::index::freq_prox_terms_writer::FreqProxTermsWriter;
use crate::index::term_vectors_consumer::TermVectorsConsumer;
use crate::util::int_block_pool::{AllocatorIntEnum, IntBlockPool};
use crate::util::{AllocatorByteEnum, ByteBlockPool, CounterEnum};
use std::cell::RefCell;
use std::rc::Rc;
#[allow(unused)]
pub(crate) struct TermsHash {
    next_terms_hash: Option<TermsHashEnum>,
    int_pool: Rc<RefCell<IntBlockPool>>,
    byte_pool: Rc<RefCell<ByteBlockPool>>,
    term_byte_pool: Option<Rc<RefCell<ByteBlockPool>>>,
    bytes_used: Rc<RefCell<CounterEnum>>,
}
#[allow(unused)]
impl TermsHash {
    pub(crate) fn new(
        int_block_allocator: Rc<RefCell<AllocatorIntEnum>>,
        byte_block_allocator: Rc<RefCell<AllocatorByteEnum>>,
        bytes_used: Rc<RefCell<CounterEnum>>,
        next_terms_hash: Option<TermsHashEnum>,
    ) -> Self {
        let term_byte_pool = None;

        let mut terms_hash = TermsHash {
            next_terms_hash,
            int_pool: Rc::new(RefCell::new(IntBlockPool::with_allocator(
                int_block_allocator,
            ))),
            byte_pool: Rc::new(RefCell::new(ByteBlockPool::new(byte_block_allocator))),
            term_byte_pool,
            bytes_used,
        };

        if let Some(next) = &mut terms_hash.next_terms_hash {
            // If we are the primary, share the byte pool
            terms_hash.term_byte_pool = Option::from(terms_hash.byte_pool.clone());
            next.set_term_byte_pool(terms_hash.term_byte_pool.clone());
        }

        terms_hash
    }
}
#[allow(unused)]
pub(crate) trait TermsHashBase {
    fn get_term_byte_pool(&self) -> Option<Rc<RefCell<ByteBlockPool>>>;
    fn set_term_byte_pool(&mut self, term_byte_pool: Option<Rc<RefCell<ByteBlockPool>>>);
}
#[allow(unused)]
pub(crate) enum TermsHashEnum {
    FreqProx(FreqProxTermsWriter),
    TermVectors(TermVectorsConsumer),
}
impl TermsHashEnum {}
impl TermsHashBase for TermsHashEnum {
    fn get_term_byte_pool(&self) -> Option<Rc<RefCell<ByteBlockPool>>> {
        match self {
            TermsHashEnum::FreqProx(writer) => writer.get_term_byte_pool().clone(),
            TermsHashEnum::TermVectors(consumer) => consumer.get_term_byte_pool().clone(),
        }
    }

    fn set_term_byte_pool(&mut self, term_byte_pool: Option<Rc<RefCell<ByteBlockPool>>>) {
        match self {
            TermsHashEnum::FreqProx(writer) => writer.set_term_byte_pool(term_byte_pool),
            TermsHashEnum::TermVectors(consumer) => consumer.set_term_byte_pool(term_byte_pool),
        }
    }
}
