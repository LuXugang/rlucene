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
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::io::Cursor;

use crate::core::index::field_term_iterator::FieldTermIterator;
use crate::core::index::term::Term;
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::store::byte_buffers_data_input::{ByteBuffersDataInput, ByteBuffersDataInputRef};
use crate::core::store::random_access_input::RandomAccessInput;
use crate::core::store::{ByteBuffersDataOutput, DataInput, DataOutput};
use crate::core::util::StringHelper;
use crate::core::util::access::WritableVec;
use crate::core::util::accountable::Accountable;
use crate::core::util::bytes_ref_iterator::BytesRefIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};

/// Prefix codes term instances (prefixes are shared). This is expected to be
/// faster to build than an FST and might also be more compact if there are no
/// common suffixes.
///
/// # Lucene Internal
#[derive(Debug)]
pub struct PrefixCodedTerms {
    content: Vec<Cursor<Vec<u8>>>,
    content_len: i64,
    size: i64,
    del_gen: i64,

    lazy_hash: i32,
}

impl PrefixCodedTerms {
    pub fn new(content: Vec<Cursor<Vec<u8>>>, content_len: i64, size: i64) -> Self {
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

    pub fn iterator(&self) -> Result<TermIterator<'_>> {
        let content = self
            .content
            .iter()
            .map(|cursor| {
                let slice = cursor.get_ref().as_slice();
                let mut cursor = Cursor::new(slice);
                cursor.set_position(0);
                cursor
            })
            .collect();
        Ok(TermIterator::new(
            self.del_gen,
            ByteBuffersDataInput::new(content, self.content_len as usize)?,
        ))
    }
}
impl Hash for PrefixCodedTerms {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for cursor in &self.content {
            cursor.get_ref().hash(state);
        }
        self.size.hash(state);
        self.del_gen.hash(state);
    }
}
impl PartialEq for PrefixCodedTerms {
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

impl Eq for PrefixCodedTerms {}
impl Accountable for PrefixCodedTerms {
    fn ram_bytes_used(&self) -> Result<i64> {
        //TODO: memory calculation not implement
        Ok(0)
    }
}

/// Builder for `PrefixCodedTerms`: call `add` repeatedly, then `finish`.
pub struct PrefixCodedTermsBuilder {
    output: ByteBuffersDataOutput,
    last_term: Term,
    last_term_bytes: BytesRefBuilder<Vec<u8>>,
    size: i64,
}

impl Default for PrefixCodedTermsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PrefixCodedTermsBuilder {
    /// Sole constructor.
    pub fn new() -> Self {
        Self {
            output: ByteBuffersDataOutput::new_resettable_instance(),
            last_term: Term::from_empty("".to_string()),
            last_term_bytes: BytesRefBuilder::new(),
            size: 0,
        }
    }
    /// add a term.
    pub fn add_term(&mut self, term: &Term) -> Result<()> {
        self.add(term.field.to_string(), &term.bytes)
    }
    /// Add a term. This fully consumes the incoming [`BytesRef`].
    pub fn add(&mut self, field: String, bytes: &BytesRef<Vec<u8>>) -> Result<()> {
        debug_assert!(
            self.last_term == Term::from_empty("".to_string())
                || Term::new(field.clone(), bytes.clone()).cmp(&self.last_term)
                    == Ordering::Greater,
        );

        let prefix;
        if self.size > 0 && field == self.last_term.field {
            // Same field as the last term
            prefix = StringHelper::bytes_difference(&self.last_term.bytes, bytes)?;
            self.output.write_vint((prefix << 1) as i32)?;
        } else {
            // Field change
            prefix = 0;
            self.output.write_vint(1)?;
            self.output.write_string(&field)?;
        }

        let suffix = bytes.length - prefix;
        self.output.write_vint(suffix as i32)?;
        self.output.write_bytes_range(
            &bytes.bytes[(bytes.offset + prefix)..(bytes.offset + prefix + suffix)],
            0,
            suffix,
        )?;
        self.last_term_bytes.copy_bytes_from_ref(bytes);
        self.last_term.bytes = self.last_term_bytes.get_bytes_owner();
        self.last_term.field = field;
        self.size += 1;

        Ok(())
    }
    /// return finalized form.
    pub fn finish(&mut self) -> PrefixCodedTerms {
        let content = self.output.get_buffer_list_owner(false);
        PrefixCodedTerms::new(content.1, content.0 as i64, self.size)
    }
}
/// An iterator over the list of terms stored in a [`PrefixCodedTerms`].
pub struct TermIterator<'a> {
    input: ByteBuffersDataInputRef<'a>,
    pub(crate) builder: BytesRefBuilder<Vec<u8>>,
    end: i64,
    del_gen: i64,
    pub(crate) field: String,
}

impl<'a> TermIterator<'a> {
    pub fn new(del_gen: i64, input: ByteBuffersDataInputRef<'a>) -> Self {
        let builder = BytesRefBuilder::new();
        let end = input.length() as i64;
        Self {
            input,
            builder,
            end,
            del_gen,
            field: "".to_string(),
        }
    }
    // TODO: maybe we should freeze to FST or automaton instead?
    pub fn read_term_bytes(&mut self, prefix: i32, suffix: i32) -> Result<()> {
        let len = (prefix + suffix) as usize;
        self.builder.grow(len);
        self.builder.bytes_ref().bytes.access_mut(|bytes| {
            DataInput::read_bytes(&mut self.input, bytes, prefix as usize, suffix as usize)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;

        self.builder.set_length(len);
        Ok(())
    }
}

impl BytesRefIterator for TermIterator<'_> {
    fn next(&mut self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        let v = self.set_next()?;
        if v {
            Ok(Some(Cow::Borrowed(self.builder.bytes_ref())))
        } else {
            Ok(None)
        }
    }

    fn set_next(&mut self) -> Result<bool> {
        if self.input.position()? < self.end as usize {
            let code = self.input.read_vint()?;
            let new_field = (code & 1) != 0;
            if new_field {
                self.field = self.input.read_string()?
            }
            let prefix = code >> 1;
            let suffix = self.input.read_vint()?;
            self.read_term_bytes(prefix, suffix)?;
            return Ok(true);
        } else {
            self.field.clear();
        }
        Ok(false)
    }
}

impl FieldTermIterator for TermIterator<'_> {
    /// Returns current field. This method should not be called after iteration
    /// is done. Note that you may use == to detect a change in field.
    fn field(&self) -> &str {
        &self.field
    }

    /// Del gen of the current term
    fn del_gen(&self) -> i64 {
        self.del_gen
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::core::index::field_term_iterator::FieldTermIterator;
    use crate::core::index::prefix_coded_terms::PrefixCodedTermsBuilder;
    use crate::core::index::term::Term;
    use crate::core::util::bytes_ref_iterator::BytesRefIterator;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{at_least, random};
    use crate::test::util::test_util::TestUtil;

    #[allow(dead_code)] // for quick search
    pub struct TestPrefixCodedTerms;

    #[test]
    fn test_empty() -> Result<()> {
        let mut builder = PrefixCodedTermsBuilder::new();
        let prefix_coded_terms = builder.finish();
        let mut iter = prefix_coded_terms.iterator()?;
        assert!(iter.next()?.is_none());
        Ok(())
    }
    #[test]
    fn test_one() -> Result<()> {
        let term = Term::from_text("foo".to_string(), "bogus");
        let mut builder = PrefixCodedTermsBuilder::new();
        builder.add_term(&term)?;
        let prefix_coded_terms = builder.finish();
        let mut iter = prefix_coded_terms.iterator()?;
        let first_term = iter.next()?.expect("Expected a term, but got None");
        assert_eq!(first_term.utf8_to_string()?, "bogus");
        assert_eq!(iter.field(), "foo");
        assert!(iter.next()?.is_none());

        Ok(())
    }

    #[test]
    fn test_random() -> Result<()> {
        let mut random = random();
        let mut terms = BTreeSet::new();
        let nterms = at_least(&mut random, 10_000);

        for _ in 0..nterms {
            let field = TestUtil::random_unicode_string_with_len(&mut random, 2);
            let text = TestUtil::random_unicode_string(&mut random);
            let term = Term::from_text(field, &text);
            terms.insert(term);
        }
        let mut builder = PrefixCodedTermsBuilder::new();
        for term in &terms {
            builder.add_term(term)?;
        }
        let pb = builder.finish();
        let mut iter = pb.iterator()?;
        let mut expected = terms.iter();

        assert_eq!(terms.len(), pb.size() as usize);

        while let Some(actual_bytes) = iter.next()? {
            let actual_bytes = actual_bytes.into_owned();
            let expected_term = expected.next();
            assert!(expected_term.is_some());
            let actual_term = Term::new(iter.field().to_string(), actual_bytes);

            assert_eq!(*expected_term.unwrap(), actual_term);
        }
        assert!(expected.next().is_none());
        Ok(())
    }
}
