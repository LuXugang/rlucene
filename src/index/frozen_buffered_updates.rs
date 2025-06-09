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
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use crate::index::buffered_updates::MTBufferedUpdates;
use crate::index::buffered_updates_stream::SegmentState;
use crate::index::field_updates_buffer::FieldUpdatesBuffer;
use crate::index::fields::Fields;
use crate::index::postings_enum::postings_enum_util;
use crate::index::prefix_coded_terms::{PrefixCodedTerms, PrefixCodedTermsBuilder};
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::terms::Terms;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::BytesRef;
use crate::search::query::Query;
use crate::store::directory::Directory;
use crate::util::access::Access;
use crate::util::accountable::Accountable;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::info_stream::{InfoStream, InfoStreamEnum};
use crate::util::ToInt;

#[allow(unused)]
pub(crate) struct FrozenBufferedUpdates<D, Q, I>
where
    D: Directory,
    Q: Query,
    I: Access<InfoStreamEnum>,
{
    info_stream: I,
    pub delete_terms: PrefixCodedTerms,
    pub delete_queries: Vec<Arc<Q>>,
    delete_query_limits: Vec<i32>,
    applied: AtomicBool,
    pub(crate) apply_lock: Mutex<()>,
    field_updates: HashMap<String, FieldUpdatesBuffer>,
    total_del_count: i64,
    field_updates_count: i32,
    bytes_used: i32,
    del_gen: i64,
    private_segment: Option<Arc<SegmentCommitInfo<D>>>,
}

#[allow(unused)]
impl<D, Q, I> FrozenBufferedUpdates<D, Q, I>
where
    D: Directory,
    Q: Query,
    I: Access<InfoStreamEnum>,
{
    // NOTE: we now apply this frozen packet immediately on creation, yet this
    // process is heavy, and runs in multiple threads, and this compression
    // is sizable (~8.3% of the original size), so it's important
    // we run this before applying the deletes/updates.
    // Query we often undercount (say 24 bytes), plus int.
    const BYTES_PER_DEL_QUERY: i32 = 0;

    pub fn new_sync(
        info_stream: I,
        updates: &mut MTBufferedUpdates<Q>,
        private_segment: Option<Arc<SegmentCommitInfo<D>>>,
    ) -> Result<Self> {
        assert!(
            private_segment.is_none() || updates.delete_terms.is_empty(),
            "segment private packet should only have del queries"
        );

        let mut builder = PrefixCodedTermsBuilder::new();
        updates
            .delete_terms
            .for_each_ordered(|term, _| builder.add_term(term));
        let delete_terms = builder.finish();

        let (delete_queries, delete_query_limits) = {
            let mut queries = Vec::with_capacity(updates.delete_queries.len());
            let mut limits = Vec::with_capacity(updates.delete_queries.len());
            for (query, limit) in &updates.delete_queries {
                queries.push(query.clone());
                limits.push(*limit);
            }
            (queries, limits)
        };
        // TODO if a Term affects multiple fields, we could keep the updates
        // key'd by Term so that it maps to all fields it affects,
        // sorted by their docUpto, and traverse that Term only once,
        // applying the update to all fields that still need to be
        // updated.
        for value in updates.field_updates.values_mut() {
            value.finish()?
        }
        let field_updates = std::mem::take(&mut updates.field_updates);
        let field_updates_count = updates.num_field_updates.load(Ordering::Relaxed);

        // TODO: memory calculation not implemented
        let bytes_used = 0;

        info_stream.access_mut(|info_stream_guard| {
            if info_stream_guard.enabled("BD") {
                let private_segment_msg = if private_segment.is_none() {
                    "None".to_string()
                } else {
                    format!("; private segment {}", private_segment.as_ref().unwrap())
                };
                info_stream_guard.message(
                    "BD",
                    &format!(
                        "compressed {} to {} bytes ({:.2}%) for deletes/updates; private segment {}",
                        updates.ram_bytes_used()?,
                        bytes_used,
                        100.0 * bytes_used as f64 / updates.ram_bytes_used()? as f64,
                        private_segment_msg
                    ),
                );
            }
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        });

        Ok(Self {
            info_stream: info_stream.clone(),
            delete_terms,
            delete_queries,
            delete_query_limits,
            applied: AtomicBool::new(false),
            apply_lock: Mutex::new(()),
            field_updates,
            total_del_count: 0,
            bytes_used,
            field_updates_count,
            del_gen: 0,
            private_segment,
        })
    }

    /// Returns `true` if this buffered updates instance has already been
    /// applied.
    pub(crate) fn is_applied(&self) -> bool {
        assert!(
            self.apply_lock.try_lock().is_err(),
            "The lock must be held by the current thread before checking applied state."
        );
        self.applied.load(Ordering::Relaxed)
    }

    pub(crate) fn apply(&self, _seg_states: SegmentState) {
        unimplemented!()
    }
    pub(crate) fn any(&self) -> bool {
        self.delete_terms.size() > 0
            || !self.delete_queries.is_empty()
            || self.field_updates_count > 0
    }
}
/// This class helps iterating a term dictionary and consuming all the docs for each term.  
/// It accepts a (field, value) tuple and returns a [`DocIdSetIterator`](crate::search::doc_id_set_iterator::DocIdSetIterator) if the field has an entry  
/// for the given value.  
///
/// It has an optimized way of iterating the term dictionary if the terms are  
/// passed in sorted order and makes sure terms and postings are reused as much as possible.
pub(crate) struct TermDocsIterator<P>
where
    P: TermsProvider,
    <P as TermsProvider>::Terms:,
{
    provider: P,
    field: Option<String>,
    terms_enum: Option<<<P as TermsProvider>::Terms as Terms>::TermsEnum>,
    postings_enum:
        Option<<<<P as TermsProvider>::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum>,
    sorted_terms: bool,
    // TODO: we should avoid copy here
    reader_term: Option<BytesRef<Vec<u8>>>,
    #[cfg(debug_assertions)]
    last_term: Option<BytesRef<Vec<u8>>>, // only set with debug_assert
}

impl<P> TermDocsIterator<P>
where
    P: TermsProvider,
{
    pub(crate) fn new(provider: P, sorted_terms: bool) -> Self {
        TermDocsIterator {
            provider,
            field: None,
            terms_enum: None,
            postings_enum: None,
            sorted_terms,
            reader_term: None,
            #[cfg(debug_assertions)]
            last_term: None,
        }
    }
    fn set_field(&mut self, mut field: Option<String>) -> Result<()> {
        if field.is_some() && self.field.as_ref() != field.as_ref() {
            self.field = field.take();

            if let Some(terms) = self.provider.terms(field.as_ref().unwrap())? {
                let mut terms_enum = terms.iterator()?;
                if self.sorted_terms {
                    // need to reset otherwise we fail the assertSorted below since we sort per field
                    debug_assert!(self.last_term.is_none());
                    self.reader_term = Option::from(terms_enum.next()?.unwrap().into_owned());
                }
                self.terms_enum = Some(terms_enum);
            } else {
                self.terms_enum = None;
            }
        }
        Ok(())
    }
    pub(crate) fn next_term(
        &mut self,
        field: &str,
        term: &BytesRef<Vec<u8>>,
    ) -> Result<
        Option<&mut <<<P as TermsProvider>::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum>,
    > {
        self.set_field(Some(field.to_string()))?;

        if let Some(terms_enum) = self.terms_enum.as_mut() {
            if self.sorted_terms {
                #[cfg(debug_assertions)]
                Self::assert_sorted(self.sorted_terms, &mut self.last_term, term);
                // in the sorted case we can take advantage of the "seeking forward" property
                // this allows us depending on the term dict impl to reuse data-structures internally
                // which speed up iteration over terms and docs significantly.
                let cmp = term
                    .cmp(self.reader_term.as_ref().expect("reader_term must be set"))
                    .to_int();

                if cmp < 0 {
                    return Ok(None); // requested term does not exist in this segment
                } else if cmp == 0 {
                    return self.get_docs().map(Some);
                } else {
                    return match terms_enum.seek_ceil(term)? {
                        SeekStatus::Found => self.get_docs().map(Some),
                        SeekStatus::NotFound => {
                            self.reader_term = Some(terms_enum.term()?.into_owned());
                            Ok(None)
                        },
                        SeekStatus::End => {
                            self.terms_enum = None;
                            Ok(None)
                        },
                    };
                }
            } else if terms_enum.seek_exact(term)? {
                return self.get_docs().map(Some);
            }
        }

        Ok(None)
    }
    #[cfg(debug_assertions)]
    fn assert_sorted(
        sorted_terms: bool,
        last_term: &mut Option<BytesRef<Vec<u8>>>,
        term: &BytesRef<Vec<u8>>,
    ) {
        debug_assert!(sorted_terms);
        if let Some(last) = last_term {
            debug_assert!(
                term >= last,
                "boom: {:?} last: {:?}",
                term.utf8_to_string(),
                last.utf8_to_string()
            );
        }
        *last_term = Some(BytesRef::deep_copy_of(term));
    }
    fn get_docs(
        &mut self,
    ) -> Result<&mut <<<P as TermsProvider>::Terms as Terms>::TermsEnum as TermsEnum>::PostingsEnum>
    {
        debug_assert!(self.terms_enum.is_some());

        let terms_enum = self.terms_enum.as_mut().unwrap();
        let postings_enum = terms_enum
            .postings_with_flags(self.postings_enum.take(), postings_enum_util::NONE as i32)?;
        self.postings_enum = Some(postings_enum);
        Ok(self.postings_enum.as_mut().unwrap())
    }
}

pub(crate) trait TermsProvider {
    type Terms: Terms;
    fn terms(&mut self, field: &str) -> Result<Option<Self::Terms>>;
}
pub(crate) struct TermsProviderImpl1<F>
where
    F: Fields,
{
    fields: F,
}
impl<F> TermsProvider for TermsProviderImpl1<F>
where
    F: Fields,
{
    type Terms = F::Terms;

    fn terms(&mut self, field: &str) -> Result<Option<Self::Terms>> {
        self.fields.terms(field)
    }
}
