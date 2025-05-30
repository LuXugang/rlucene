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
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::codecs::norms_producer::NormsProducer;
use crate::index::field_info::FieldInfo;
use crate::index::field_invert_state::FieldInvertState;
use crate::index::merge_state::DocMap;
use crate::index::segment_write_state::SegmentWriteState;
use crate::index::terms_hash_per_field::TermsHashPerField;
use crate::store::directory::Directory;
use crate::util::allocator_byte::AllocatorByteEnum;
use crate::util::error::lucene_error::Result;
use crate::util::int_block_pool::{AllocatorIntEnum, IntBlockPool};
use crate::util::{ByteBlockPool, ByteBlockPoolBorrow, CounterEnumBorrow};
pub(crate) struct TermsHash<S>
where
    S: TermsHashBase,
{
    next_terms_hash: Option<Box<TermsHash<S>>>,
    int_pool: IntBlockPool,
    byte_pool: ByteBlockPoolBorrow,
    term_byte_pool: Option<ByteBlockPoolBorrow>,
    bytes_used: CounterEnumBorrow,
    sub: S,
}
impl<S> TermsHash<S>
where
    S: TermsHashBase,
{
    pub(crate) fn new(
        int_block_allocator: Rc<RefCell<AllocatorIntEnum>>,
        byte_block_allocator: AllocatorByteEnum<CounterEnumBorrow>,
        bytes_used: CounterEnumBorrow,
        next_terms_hash: Option<Box<TermsHash<S>>>,
        sub: S,
    ) -> Self {
        let term_byte_pool = None;

        let mut terms_hash = TermsHash {
            next_terms_hash,
            int_pool: IntBlockPool::with_allocator(int_block_allocator),
            byte_pool: Rc::new(RefCell::new(ByteBlockPool::new(byte_block_allocator))),
            term_byte_pool,
            bytes_used,
            sub,
        };

        if let Some(next_terms_hash) = &mut terms_hash.next_terms_hash {
            // If we are the primary, share the byte pool
            terms_hash.term_byte_pool = Option::from(terms_hash.byte_pool.clone());
            next_terms_hash.term_byte_pool = Option::from(terms_hash.byte_pool.clone());
        }
        terms_hash
    }
}
impl<S> TermsHashBase for TermsHash<S>
where
    S: TermsHashBase,
{
    fn abort(&mut self) {
        self.reset();
        if self.next_terms_hash.is_some() {
            self.next_terms_hash.as_mut().unwrap().abort();
        }
    }

    fn reset(&mut self) {
        self.int_pool.reset(false, false);
        self.byte_pool.borrow_mut().reset(false, false)
    }

    fn flush<D, N, M>(
        &mut self,
        fields_to_flush: &mut HashMap<String, TermsHashPerField>,
        state: &SegmentWriteState<D>,
        sort_map: &M,
        norms: &mut N,
    ) -> Result<()>
    where
        D: Directory,
        N: NormsProducer,
        M: DocMap,
    {
        todo!()
    }

    fn add_field(
        &mut self,
        field_invert_state: &FieldInvertState,
        field_info: &FieldInfo,
    ) -> TermsHashPerField {
        self.sub.add_field(field_invert_state, field_info)
    }

    fn start_document(&mut self) -> Result<()> {
        if self.next_terms_hash.is_some() {
            self.next_terms_hash.as_mut().unwrap().start_document()?;
        }
        Ok(())
    }

    fn finish_document(&mut self, doc_id: i32) -> Result<()> {
        if self.next_terms_hash.is_some() {
            self.next_terms_hash
                .as_mut()
                .unwrap()
                .finish_document(doc_id)?;
        }
        Ok(())
    }
}

pub(crate) trait TermsHashBase {
    fn abort(&mut self);
    fn reset(&mut self);
    fn flush<D, N, M>(
        &mut self,
        fields_to_flush: &mut HashMap<String, TermsHashPerField>,
        state: &SegmentWriteState<D>,
        sort_map: &M,
        norms: &mut N,
    ) -> Result<()>
    where
        D: Directory,
        N: NormsProducer,
        M: DocMap;

    fn add_field(
        &mut self,
        field_invert_state: &FieldInvertState,
        field_info: &FieldInfo,
    ) -> TermsHashPerField;

    fn start_document(&mut self) -> Result<()>;

    fn finish_document(&mut self, doc_id: i32) -> Result<()>;
}
