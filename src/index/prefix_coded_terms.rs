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
use crate::index::field_term_iterator::FieldTermIterator;
use crate::index::term::Term;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::byte_buffers_data_input::ByteBuffersDataInput;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::util::accountable::Accountable;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::error::lucene_error::LuceneError;
use crate::util::StringHelper;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::io::Cursor;

/// Prefix codes term instances (prefixes are shared). This is expected to be faster to build than an
/// FST and might also be more compact if there are no common suffixes.
///
/// # Lucene Internal
#[derive(Debug)]
pub struct PrefixCodedTerms<'a> {
    content: Vec<Cursor<&'a [u8]>>,
    content_len: i64,
    size: i64,
    del_gen: i64,
    #[allow(unused)]
    lazy_hash: i32,
}

impl<'a> PrefixCodedTerms<'a> {
    pub fn new(content: Vec<Cursor<&'a [u8]>>, content_len: i64, size: i64) -> Self {
        debug_assert!(!content.is_empty());
        PrefixCodedTerms {
            content,
            content_len,
            size,
            del_gen: 0,
            lazy_hash: 0,
        }
    }

    /// Records del gen for this packet.
    pub fn set_del_gen(&mut self, del_gen: i64) {
        self.del_gen = del_gen;
    }
    /// Return the number of terms stored in this [`PrefixCodedTerms`].
    pub fn size(&self) -> i64 {
        self.size
    }
    pub fn iterator(&self) -> TermIterator {
        TermIterator::new(
            self.del_gen,
            ByteBuffersDataInput::new(self.content.clone(), self.content_len),
        )
    }
}
impl Hash for PrefixCodedTerms<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for cursor in &self.content {
            cursor.get_ref().hash(state);
        }
        self.size.hash(state);
        self.del_gen.hash(state);
    }
}
impl PartialEq for PrefixCodedTerms<'_> {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        self.del_gen == other.del_gen
            && self.size() == other.size()
            && self.content.len() == other.content.len()
            && self
                .content
                .iter()
                .zip(&other.content)
                .all(|(a, b)| a.get_ref() == b.get_ref())
    }
}

impl Eq for PrefixCodedTerms<'_> {}
impl Accountable for PrefixCodedTerms<'_> {
    fn ram_bytes_used(&self) -> i64 {
        //TODO: memory calculation not implemented
        0
    }
}

/// Builder for `PrefixCodedTerms`: call `add` repeatedly, then `finish`.
pub struct PrefixCodedTermsBuilder {
    output: ByteBuffersDataOutput,
    last_term: Term,
    last_term_bytes: BytesRefBuilder,
    size: i64,
}

impl PrefixCodedTermsBuilder {
    /// Sole constructor.
    pub fn new() -> Result<Self, LuceneError> {
        Ok(Self {
            output: ByteBuffersDataOutput::with_resettable_instance()?,
            last_term: Term::from_empty("".to_string()),
            last_term_bytes: BytesRefBuilder::new(),
            size: 0,
        })
    }
    /// add a term.
    pub fn add_term(&mut self, term: &Term) -> Result<(), LuceneError> {
        self.add(term.field.to_string(), &term.bytes)
    }
    /// Add a term. This fully consumes the incoming [`BytesRef`](BytesRef).
    pub fn add(&mut self, field: String, bytes: &BytesRef) -> Result<(), LuceneError> {
        debug_assert!(
            self.last_term == Term::from_empty("".to_string())
                || Term::new(field.clone(), bytes.clone()).cmp(&self.last_term)
                    == Ordering::Greater,
        );

        let prefix: i32;
        if self.size > 0 && field == self.last_term.field {
            // Same field as the last term
            prefix = StringHelper::bytes_difference(&self.last_term.bytes, bytes)?;
            self.output.write_vint(prefix << 1)?;
        } else {
            // Field change
            prefix = 0;
            self.output.write_vint(1)?;
            self.output.write_string(&field)?;
        }

        let suffix = bytes.length - prefix;
        self.output.write_vint(suffix)?;
        self.output.write_bytes_range(
            &bytes.bytes
                [(bytes.offset + prefix) as usize..(bytes.offset + prefix + suffix) as usize],
            0,
            suffix,
        )?;
        self.last_term_bytes.copy_bytes_with_ref(bytes)?;
        self.last_term.bytes = self.last_term_bytes.get_bytes_ref();
        self.last_term.field = field;
        self.size += 1;

        Ok(())
    }
    /// return finalized form.
    pub fn finish(&mut self) -> Result<PrefixCodedTerms, LuceneError> {
        let content = self.output.to_buffer_list();
        Ok(PrefixCodedTerms::new(content.1, content.0, self.size))
    }
}
/// An iterator over the list of terms stored in a [`PrefixCodedTerms`].
pub struct TermIterator<'a> {
    input: ByteBuffersDataInput<'a>,
    builder: BytesRefBuilder,
    bytes: BytesRef,
    end: i64,
    del_gen: i64,
    field: String,
}

impl<'a> TermIterator<'a> {
    pub fn new(del_gen: i64, input: ByteBuffersDataInput<'a>) -> Self {
        let mut builder = BytesRefBuilder::new();
        let bytes = builder.get_bytes_ref();
        let end = input.length();
        Self {
            input,
            builder,
            bytes,
            end,
            del_gen,
            field: "".to_string(),
        }
    }
    // TODO: maybe we should freeze to FST or automaton instead?
    pub fn read_term_bytes(&mut self, prefix: i32, suffix: i32) -> Result<(), LuceneError> {
        self.builder.grow(prefix + suffix)?;
        DataInput::read_bytes(
            &mut self.input,
            &mut self.builder.bytes_ref().bytes,
            prefix,
            suffix,
        )?;
        self.builder.set_length(prefix + suffix);
        self.bytes = self.builder.get_bytes_ref();
        Ok(())
    }
}

impl BytesRefIterator for TermIterator<'_> {
    fn next(&mut self) -> Result<Option<BytesRef>, LuceneError> {
        if self.input.position() < self.end {
            let code = self.input.read_vint()?;
            let new_field = (code & 1) != 0;
            if new_field {
                self.field = self.input.read_string()?
            }
            let prefix = code >> 1;
            let suffix = self.input.read_vint()?;
            self.read_term_bytes(prefix, suffix)?;
            return Ok(Some(std::mem::take(&mut self.bytes)));
        } else {
            self.field.clear();
        }
        Ok(None)
    }
}

impl FieldTermIterator for TermIterator<'_> {
    /// Returns current field. This method should not be called after iteration is done. Note that
    /// you may use == to detect a change in field.
    fn field(&self) -> &str {
        &self.field
    }

    /// Del gen of the current term
    fn del_gen(&self) -> i64 {
        self.del_gen
    }
}
