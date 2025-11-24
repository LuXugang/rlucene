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
use crate::core::index::terms_enum::{SeekStatus, TermsEnum};
use crate::core::index::{BytesRef, BytesRefBuilder};
use crate::core::util::ToInt;
use crate::core::util::bit_util::BitUtil;
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Wrapper around a [`TermsEnum`] and an integer that identifies it.
///
/// All operations that move the current position of the [`TermsEnum`]
/// must be performed via this wrapper, not directly on the wrapped [`TermsEnum`].
///
/// This follows the behavior of Lucene's `TermsEnumIndex`.
pub(crate) struct TermsEnumIndex<TE>
where
    TE: TermsEnum,
{
    sub_index: i32,
    terms_enum: Option<TE>,
    current_term: Option<BytesRef<Vec<u8>>>,
    current_term_prefix8: i64,
}
impl<TE> TermsEnumIndex<TE>
where
    TE: TermsEnum,
{
    pub fn new(terms_enum: Option<TE>, sub_index: i32) -> Self {
        Self {
            sub_index,
            terms_enum,
            current_term: None,
            current_term_prefix8: 0,
        }
    }

    pub fn term(&self) -> Option<&BytesRef<Vec<u8>>> {
        self.current_term.as_ref()
    }

    fn set_term(&mut self, term: Option<BytesRef<Vec<u8>>>) {
        if let Some(ref t) = term {
            self.current_term_prefix8 = prefix8_to_comparable_unsigned_long(t) as i64;
        } else {
            self.current_term_prefix8 = 0;
        }
        self.current_term = term;
    }

    pub(crate) fn next(&mut self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        let Some(terms_enum) = &mut self.terms_enum else {
            return Err(LuceneError::illegal_state("terms_enum is None"));
        };
        let term = terms_enum.next()?;
        let v = term.map(|t| t.into_owned());
        self.set_term(v);
        Ok(self.current_term.as_ref())
    }
    pub(crate) fn seek_ceil(&mut self, term: &BytesRef<Vec<u8>>) -> Result<SeekStatus> {
        let Some(terms_enum) = &mut self.terms_enum else {
            return Err(LuceneError::illegal_state("terms_enum is None"));
        };
        let status = terms_enum.seek_ceil(term)?;

        if status == SeekStatus::End {
            self.set_term(None);
        } else {
            let v = Some(terms_enum.term()?.into_owned());
            self.set_term(v);
        }

        Ok(status)
    }
    pub(crate) fn seek_exact(&mut self, term: &BytesRef<Vec<u8>>) -> Result<bool> {
        let Some(terms_enum) = &mut self.terms_enum else {
            return Err(LuceneError::illegal_state("terms_enum is None"));
        };
        let found = terms_enum.seek_exact(term)?;

        if found {
            let v = Some(terms_enum.term()?.into_owned());
            self.set_term(v);
        } else {
            self.set_term(None);
        }

        Ok(found)
    }
    pub(crate) fn reset(&mut self, mut other: Self) {
        self.terms_enum = other.terms_enum;
        self.current_term = other.current_term.take();
        self.current_term_prefix8 = other.current_term_prefix8;
    }
    pub(crate) fn compare_term_to(&self, that: &Self) -> Result<i32> {
        if self.current_term_prefix8 != that.current_term_prefix8 {
            let cmp = self
                .current_term_prefix8
                .cmp(&that.current_term_prefix8)
                .to_int();

            debug_assert_eq!(
                {
                    let current_term = self.current_term.as_ref().unwrap();
                    let that = that.current_term.as_ref().unwrap();
                    current_term.bytes
                        [current_term.offset..current_term.offset + current_term.length]
                        .cmp(&that.bytes[that.offset..that.offset + that.length])
                        .to_int()
                },
                cmp.signum()
            );

            return Ok(cmp);
        }
        match (self.current_term.as_ref(), that.current_term.as_ref()) {
            (Some(current_term), Some(that_term)) => Ok(current_term.bytes
                [current_term.offset..current_term.offset + current_term.length]
                .cmp(&that_term.bytes[that_term.offset..that_term.offset + that_term.length])
                .to_int()),
            _ => Err(LuceneError::illegal_state("Both terms must be non-null")),
        }
    }
    pub(crate) fn term_equals(&self, that: &TermState) -> Result<bool> {
        if self.current_term_prefix8 != that.term_prefix8 {
            return Ok(false);
        }

        let Some(current_term) = &self.current_term else {
            return Err(LuceneError::illegal_state("current_term is None"));
        };

        let term = &that.term;

        Ok(
            current_term.bytes[current_term.offset..current_term.offset + current_term.length]
                .cmp(&term.bytes_ref.bytes[0..term.length()])
                .to_int()
                == 0,
        )
    }
}
pub(crate) struct TermState {
    term: BytesRefBuilder<Vec<u8>>,
    pub(crate) term_prefix8: i64,
}
impl TermState {
    pub fn copy_from<TE: TermsEnum>(&mut self, tei: &TermsEnumIndex<TE>) -> Result<()> {
        match tei.term() {
            Some(t) => {
                self.term.copy_bytes_with_ref(t);
                self.term_prefix8 = tei.current_term_prefix8;
                Ok(())
            },
            None => Err(LuceneError::illegal_state("term in TermsEnumIndex is None")),
        }
    }
}
/// Copy the first 8 bytes of the given term as a comparable unsigned long.
///
/// In case the term has less than 8 bytes, missing bytes will be replaced with zeroes.
///
/// Note that two terms that produce the same long could still be different
/// due to the fact that missing bytes are replaced with zeroes, e.g.
/// `[1, 0]` and `[1]` get mapped to the same long.
///
/// This is used by `TermsEnumIndex` to perform fast prefix comparisons.
///
/// Ported from Lucene's `TermsEnumIndex.prefix8ToComparableUnsignedLong`.
pub fn prefix8_to_comparable_unsigned_long(term: &BytesRef<Vec<u8>>) -> u64 {
    let bytes = &term.bytes;
    let offset = term.offset;
    let len = term.length;

    if len >= BitUtil::LONG_BYTES {
        return BitUtil::get_i64_be(bytes, offset) as u64;
    }

    let mut l = 0;
    let mut o = 0;

    if len >= BitUtil::INT_BYTES {
        l = BitUtil::get_i32_be(bytes, offset) as u64;
        o = BitUtil::INT_BYTES;
    }

    if o + BitUtil::SHORT_BYTES <= len {
        let v = BitUtil::get_i16_be(bytes, offset + o) as u64;
        l = (l << i16::BITS) | v;
        o += BitUtil::SHORT_BYTES;
    }

    if o < len {
        let v = bytes[offset + o] as u64;
        l = (l << i8::BITS) | v;
    }

    let pad_bits = (BitUtil::LONG_BYTES - len) << 3;
    l << pad_bits
}
#[cfg(test)]
mod tests {
    use crate::core::index::BytesRef;
    use crate::core::index::terms_enum_index::prefix8_to_comparable_unsigned_long;

    #[test]
    fn test_prefix8_to_comparable_unsigned_long() {
        let b = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        assert_eq!(
            0u64,
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 1,
                length: 0,
            })
        );

        assert_eq!(
            4u64 << 56,
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 1,
            })
        );

        assert_eq!(
            (4u64 << 56) | (5u64 << 48),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 2,
            })
        );

        assert_eq!(
            (4u64 << 56) | (5u64 << 48) | (6u64 << 40),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 3,
            })
        );

        assert_eq!(
            (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 4,
            })
        );

        assert_eq!(
            (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32) | (8u64 << 24),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 5,
            })
        );

        assert_eq!(
            (4u64 << 56) | (5u64 << 48) | (6u64 << 40) | (7u64 << 32) | (8u64 << 24) | (9u64 << 16),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 6,
            })
        );

        assert_eq!(
            (4u64 << 56)
                | (5u64 << 48)
                | (6u64 << 40)
                | (7u64 << 32)
                | (8u64 << 24)
                | (9u64 << 16)
                | (10u64 << 8),
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 7,
            })
        );

        assert_eq!(
            (4u64 << 56)
                | (5u64 << 48)
                | (6u64 << 40)
                | (7u64 << 32)
                | (8u64 << 24)
                | (9u64 << 16)
                | (10u64 << 8)
                | 11u64,
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b.clone(),
                offset: 3,
                length: 8,
            })
        );

        assert_eq!(
            (4u64 << 56)
                | (5u64 << 48)
                | (6u64 << 40)
                | (7u64 << 32)
                | (8u64 << 24)
                | (9u64 << 16)
                | (10u64 << 8)
                | 11u64,
            prefix8_to_comparable_unsigned_long(&BytesRef {
                bytes: b,
                offset: 3,
                length: 9,
            })
        );
    }
}
