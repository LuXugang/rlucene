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

use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use once_cell::sync::Lazy;
use rand::prelude::IndexedRandom;
use rand::{Rng, random_range};

use crate::core::index::BytesRef;
use crate::core::index::postings_enum::{ALL, FREQS, OFFSETS, PAYLOADS, POSITIONS};
use crate::core::index::terms_enum::TermsEnum;
use crate::core::util::access::SharedAccessVec;
use crate::core::util::error::lucene_error::Result;

pub struct TestUtil;
const BLOCK_STARTS: &[u32] = &[
    0x0000, 0x0080, 0x0100, 0x0180, 0x0250, 0x02B0, 0x0300, 0x0370, 0x0400, 0x0500, 0x0530, 0x0590,
    0x0600, 0x0700, 0x0750, 0x0780, 0x07C0, 0x0800, 0x0900, 0x0980, 0x0A00, 0x0A80, 0x0B00, 0x0B80,
    0x0C00, 0x0C80, 0x0D00, 0x0D80, 0x0E00, 0x0E80, 0x0F00, 0x1000, 0x10A0, 0x1100, 0x1200, 0x1380,
    0x13A0, 0x1400, 0x1680, 0x16A0, 0x1700, 0x1720, 0x1740, 0x1760, 0x1780, 0x1800, 0x18B0, 0x1900,
    0x1950, 0x1980, 0x19E0, 0x1A00, 0x1A20, 0x1B00, 0x1B80, 0x1C00, 0x1C50, 0x1CD0, 0x1D00, 0x1D80,
    0x1DC0, 0x1E00, 0x1F00, 0x2000, 0x2070, 0x20A0, 0x20D0, 0x2100, 0x2150, 0x2190, 0x2200, 0x2300,
    0x2400, 0x2440, 0x2460, 0x2500, 0x2580, 0x25A0, 0x2600, 0x2700, 0x27C0, 0x27F0, 0x2800, 0x2900,
    0x2980, 0x2A00, 0x2B00, 0x2C00, 0x2C60, 0x2C80, 0x2D00, 0x2D30, 0x2D80, 0x2DE0, 0x2E00, 0x2E80,
    0x2F00, 0x2FF0, 0x3000, 0x3040, 0x30A0, 0x3100, 0x3130, 0x3190, 0x31A0, 0x31C0, 0x31F0, 0x3200,
    0x3300, 0x3400, 0x4DC0, 0x4E00, 0xA000, 0xA490, 0xA4D0, 0xA500, 0xA640, 0xA6A0, 0xA700, 0xA720,
    0xA800, 0xA830, 0xA840, 0xA880, 0xA8E0, 0xA900, 0xA930, 0xA960, 0xA980, 0xAA00, 0xAA60, 0xAA80,
    0xABC0, 0xAC00, 0xD7B0, 0xE000, 0xF900, 0xFB00, 0xFB50, 0xFE00, 0xFE10, 0xFE20, 0xFE30, 0xFE50,
    0xFE70, 0xFF00, 0xFFF0, 0x10000, 0x10080, 0x10100, 0x10140, 0x10190, 0x101D0, 0x10280, 0x102A0,
    0x10300, 0x10330, 0x10380, 0x103A0, 0x10400, 0x10450, 0x10480, 0x10800, 0x10840, 0x10900,
    0x10920, 0x10A00, 0x10A60, 0x10B00, 0x10B40, 0x10B60, 0x10C00, 0x10E60, 0x11080, 0x12000,
    0x12400, 0x13000, 0x1D000, 0x1D100, 0x1D200, 0x1D300, 0x1D360, 0x1D400, 0x1F000, 0x1F030,
    0x1F100, 0x1F200, 0x20000, 0x2A700, 0x2F800, 0xE0000, 0xE0100, 0xF0000, 0x100000,
];
const BLOCK_ENDS: &[u32] = &[
    0x007F, 0x00FF, 0x017F, 0x024F, 0x02AF, 0x02FF, 0x036F, 0x03FF, 0x04FF, 0x052F, 0x058F, 0x05FF,
    0x06FF, 0x074F, 0x077F, 0x07BF, 0x07FF, 0x083F, 0x097F, 0x09FF, 0x0A7F, 0x0AFF, 0x0B7F, 0x0BFF,
    0x0C7F, 0x0CFF, 0x0D7F, 0x0DFF, 0x0E7F, 0x0EFF, 0x0FFF, 0x109F, 0x10FF, 0x11FF, 0x137F, 0x139F,
    0x13FF, 0x167F, 0x169F, 0x16FF, 0x171F, 0x173F, 0x175F, 0x177F, 0x17FF, 0x18AF, 0x18FF, 0x194F,
    0x197F, 0x19DF, 0x19FF, 0x1A1F, 0x1AAF, 0x1B7F, 0x1BBF, 0x1C4F, 0x1C7F, 0x1CFF, 0x1D7F, 0x1DBF,
    0x1DFF, 0x1EFF, 0x1FFF, 0x206F, 0x209F, 0x20CF, 0x20FF, 0x214F, 0x218F, 0x21FF, 0x22FF, 0x23FF,
    0x243F, 0x245F, 0x24FF, 0x257F, 0x259F, 0x25FF, 0x26FF, 0x27BF, 0x27EF, 0x27FF, 0x28FF, 0x297F,
    0x29FF, 0x2AFF, 0x2BFF, 0x2C5F, 0x2C7F, 0x2CFF, 0x2D2F, 0x2D7F, 0x2DDF, 0x2DFF, 0x2E7F, 0x2EFF,
    0x2FDF, 0x2FFF, 0x303F, 0x309F, 0x30FF, 0x312F, 0x318F, 0x319F, 0x31BF, 0x31EF, 0x31FF, 0x32FF,
    0x33FF, 0x4DBF, 0x4DFF, 0x9FFF, 0xA48F, 0xA4CF, 0xA4FF, 0xA63F, 0xA69F, 0xA6FF, 0xA71F, 0xA7FF,
    0xA82F, 0xA83F, 0xA87F, 0xA8DF, 0xA8FF, 0xA92F, 0xA95F, 0xA97F, 0xA9DF, 0xAA5F, 0xAA7F, 0xAADF,
    0xABFF, 0xD7AF, 0xD7FF, 0xF8FF, 0xFAFF, 0xFB4F, 0xFDFF, 0xFE0F, 0xFE1F, 0xFE2F, 0xFE4F, 0xFE6F,
    0xFEFF, 0xFFEF, 0xFFFF, 0x1007F, 0x100FF, 0x1013F, 0x1018F, 0x101CF, 0x101FF, 0x1029F, 0x102DF,
    0x1032F, 0x1034F, 0x1039F, 0x103DF, 0x1044F, 0x1047F, 0x104AF, 0x1083F, 0x1085F, 0x1091F,
    0x1093F, 0x10A5F, 0x10A7F, 0x10B3F, 0x10B5F, 0x10B7F, 0x10C4F, 0x10E7F, 0x110CF, 0x123FF,
    0x1247F, 0x1342F, 0x1D0FF, 0x1D1FF, 0x1D24F, 0x1D35F, 0x1D37F, 0x1D7FF, 0x1F02F, 0x1F09F,
    0x1F1FF, 0x1F2FF, 0x2A6DF, 0x2B73F, 0x2FA1F, 0xE007F, 0xE01EF, 0xFFFFF, 0x10FFFF,
];

impl TestUtil {
    pub fn string_codepoint_comparator(a: &str, b: &str) -> std::cmp::Ordering {
        let mut a_chars = a.chars();
        let mut b_chars = b.chars();

        loop {
            match (a_chars.next(), b_chars.next()) {
                (Some(a_cp), Some(b_cp)) => {
                    if a_cp != b_cp {
                        return a_cp.cmp(&b_cp);
                    }
                },
                (None, None) => return std::cmp::Ordering::Equal,
                (None, _) => return std::cmp::Ordering::Less,
                (_, None) => return std::cmp::Ordering::Greater,
            }
        }
    }
    /// start and end are BOTH inclusive
    pub fn next_int<R: Rng + ?Sized>(random: &mut R, start: i32, end: i32) -> i32 {
        random.random_range(start..=end)
    }
    /// start and end are BOTH inclusive
    pub fn next_long<R: Rng + ?Sized>(random: &mut R, start: i64, end: i64) -> i64 {
        assert!(end >= start, "start={}, end={}", start, end);
        let range = BigInt::from(end) + BigInt::from(1) - BigInt::from(start);
        if range <= BigInt::from(i32::MAX) {
            start + random.random_range(0..range.to_i32().unwrap()) as i64
        } else {
            let augend =
                BigInt::from_f64(range.to_f64().unwrap() * random.random::<f64>()).unwrap();
            let result = BigInt::from(start) + augend;
            let result = result.to_i64().unwrap();
            assert!(result >= start);
            assert!(result <= end);
            result
        }
    }
    /// Returns a random big integer with `1 .. max_bytes` storage.
    pub fn next_big_integer<R: Rng + ?Sized>(random: &mut R, max_bytes: i32) -> BigInt {
        let length = Self::next_int(random, 1, max_bytes);
        let mut buffer = vec![0u8; length as usize];
        random.fill_bytes(&mut buffer);
        BigInt::from_signed_bytes_be(&buffer)
    }
    pub fn random_simple_string_with_len<R: Rng + ?Sized>(
        random: &mut R,
        max_length: usize,
    ) -> String {
        Self::random_simple_string_range(random, 0, max_length)
    }
    pub fn random_simple_string_range<R: Rng + ?Sized>(
        random: &mut R,
        min_length: usize,
        max_length: usize,
    ) -> String {
        let end = random.random_range(min_length..=max_length);
        if end == 0 {
            return String::new();
        }
        (0..end)
            .map(|_| random.random_range(b'a'..=b'z') as char)
            .collect()
    }

    pub fn random_simple_string<R: Rng + ?Sized>(random: &mut R) -> String {
        Self::random_simple_string_range(random, 0, 10)
    }

    pub fn random_htmlish_string<R: Rng + ?Sized>(random: &mut R, num_elements: usize) -> String {
        use std::fmt::Write;
        let end = random.random_range(0..=num_elements);
        if end == 0 {
            return String::new();
        }

        let mut sb = String::new();

        for _ in 0..end {
            match random.random_range(0..25) {
                0 => sb.push_str("<p>"),
                1 => {
                    sb.push('<');
                    sb.push_str(&"    "[..random.random_range(0..=4)]);
                    sb.push_str(&Self::random_simple_string(random));
                    for _ in 0..random.random_range(0..10) {
                        sb.push(' ');
                        sb.push_str(&Self::random_simple_string(random));
                        sb.push_str(&" "[..random.random_range(0..=1)]);
                        sb.push('=');
                        sb.push_str(&" "[..random.random_range(0..=1)]);
                        sb.push_str(&"\""[..random.random_range(0..=1)]);
                        sb.push_str(&Self::random_simple_string(random));
                        sb.push_str(&"\""[..random.random_range(0..=1)]);
                    }
                    sb.push_str(&"    "[..random.random_range(0..=4)]);
                    if random.random_bool(0.5) {
                        sb.push('/');
                    }
                    if random.random_bool(0.5) {
                        sb.push('>');
                    }
                },
                2 => {
                    sb.push_str("</");
                    sb.push_str(&"    "[..random.random_range(0..=4)]);
                    sb.push_str(&Self::random_simple_string(random));
                    sb.push_str(&"    "[..random.random_range(0..=4)]);
                    if random.random_bool(0.5) {
                        sb.push('>');
                    }
                },
                3 => sb.push('>'),
                4 => sb.push_str("</p>"),
                5 => sb.push_str("<!--"),
                6 => sb.push_str("<!--#"),
                7 => sb.push_str("<script><!-- f('"),
                8 => sb.push_str("</script>"),
                9 => sb.push_str("<?"),
                10 => sb.push_str("?>"),
                11 => sb.push('"'),
                12 => sb.push_str("\\\""),
                13 => sb.push('\''),
                14 => sb.push_str("\\'"),
                15 => sb.push_str("-->"),
                16 => {
                    sb.push('&');
                    match random.random_range(0..2) {
                        0 => sb.push_str(&Self::random_simple_string(random)),
                        1 => {
                            let entity = HTML_CHAR_ENTITIES.choose(random).unwrap();
                            sb.push_str(entity);
                        },
                        _ => {},
                    }
                    if random.random_bool(0.5) {
                        sb.push(';');
                    }
                },
                17 => {
                    sb.push_str("&#");
                    if random.random_bool(0.5) {
                        write!(sb, "{}", random.random::<u32>()).unwrap();
                        if random.random_bool(0.5) {
                            sb.push(';');
                        }
                    }
                },
                18 => {
                    sb.push_str("&#x");
                    if random.random_bool(0.5) {
                        write!(sb, "{:x}", random.random::<u32>()).unwrap();
                        if random.random_bool(0.5) {
                            sb.push(';');
                        }
                    }
                },
                19 => sb.push(';'),
                20 => {
                    write!(sb, "{}", random.random::<u32>()).unwrap();
                },
                21 => sb.push('\n'),
                22 => sb.push_str(&"          "[..random.random_range(0..=10)]),
                23 => {
                    sb.push('<');
                    if random.random_ratio(1, 3) {
                        sb.push_str(&"          "[..random.random_range(1..=10)]);
                    }
                    if random.random_bool(0.5) {
                        sb.push('/');
                        if random.random_ratio(1, 3) {
                            sb.push_str(&"          "[..random.random_range(1..=10)]);
                        }
                    }
                    let tag = match random.random_range(0..3) {
                        0 => Self::randomly_recase_codepoints(random, "script"),
                        1 => Self::randomly_recase_codepoints(random, "style"),
                        _ => Self::randomly_recase_codepoints(random, "br"),
                    };
                    sb.push_str(&tag);
                    if random.random_bool(0.5) {
                        sb.push('>');
                    }
                },
                _ => {
                    sb.push_str(&Self::random_simple_string(random));
                },
            }
        }
        sb
    }

    pub fn randomly_recase_codepoints<R: Rng + ?Sized>(random: &mut R, s: &str) -> String {
        let mut result = String::with_capacity(s.len());

        for ch in s.chars() {
            match random.random_range(0..=2) {
                0 => result.push_str(&ch.to_uppercase().to_string()),
                1 => result.push_str(&ch.to_lowercase().to_string()),
                _ => result.push(ch), // leave intact
            }
        }
        result
    }

    pub fn random_realistic_unicode_string<R: Rng + ?Sized>(random: &mut R) -> String {
        Self::random_realistic_unicode_string_with_len(random, 20)
    }

    pub fn random_realistic_unicode_string_with_len<R: Rng + ?Sized>(
        random: &mut R,
        max_length: usize,
    ) -> String {
        Self::random_realistic_unicode_string_range(random, 0, max_length)
    }

    pub fn random_realistic_unicode_string_range<R: Rng + ?Sized>(
        rng: &mut R,
        min_length: usize,
        max_length: usize,
    ) -> String {
        let end = rng.random_range(min_length..=max_length);

        // Choose a random Unicode block
        let block = rng.random_range(0..BLOCK_STARTS.len());
        let block_start = BLOCK_STARTS[block];
        let block_end = BLOCK_ENDS[block];

        // Generate random codepoints within the selected block
        let mut result = String::new();
        for _ in 0..end {
            let codepoint = rng.random_range(block_start..=block_end);
            if let Some(c) = char::from_u32(codepoint) {
                result.push(c);
            }
        }
        result
    }
    /// Returns random string, including full unicode range
    pub fn random_unicode_string<R: Rng + ?Sized>(random: &mut R) -> String {
        Self::random_unicode_string_with_len(random, 20)
    }
    /// Returns a random string up to a certain length.
    pub fn random_unicode_string_with_len<R: Rng + ?Sized>(
        random: &mut R,
        max_length: usize,
    ) -> String {
        let end = random.random_range(0..=max_length);
        if end == 0 {
            return "".to_string();
        }

        let mut buffer: Vec<u16> = Vec::with_capacity(end);
        Self::random_fixed_length_unicode_string(random, &mut buffer, end);
        String::from_utf16_lossy(&buffer)
    }
    pub fn random_fixed_length_unicode_string<R: Rng + ?Sized>(
        random: &mut R,
        buffer: &mut Vec<u16>,
        length: usize,
    ) {
        for _ in 0..length {
            let t = random.random_range(0..5);
            match t {
                0 => {
                    // Generate a surrogate pair (high and low surrogate)
                    buffer.push(random.random_range(0xD800..=0xDBFF) as u16);
                    buffer.push(random.random_range(0xDC00..=0xDFFF) as u16);
                },
                1 => {
                    buffer.push(random.random_range(0x00..=0x7F) as u16);
                },
                2 => {
                    buffer.push(random.random_range(0x80..=0x7FF) as u16);
                },
                3 => {
                    buffer.push(random.random_range(0x800..=0xD7FF) as u16);
                },
                4 => {
                    buffer.push(random.random_range(0xE000..=0xFFFF) as u16);
                },
                _ => unreachable!(),
            }
        }
    }
    pub fn random_fixed_length_unicode_string_with_chars<R: Rng + ?Sized>(
        random: &mut R,
        chars: &mut [u16],
        offset: usize,
        length: usize,
    ) {
        let mut i = offset;
        let end = offset + length;
        while i < end {
            let t = random.random_range(0..5);
            if t == 0 && i < end - 1 {
                chars[i] = random_range(0xd800..0xdbff);
                chars[i + 1] = random_range(0xdc00..0xdfff);
                i += 2;
            } else if t <= 1 {
                chars[i] = random.random_range(0x00..=0x7f);
                i += 1;
            } else if t == 2 {
                chars[i] = random_range(0x80..0x7ff);
                i += 1;
            } else if t == 3 {
                chars[i] = random_range(0x800..0xd7ff);
                i += 1;
            } else if t == 4 {
                chars[i] = random_range(0xe000..0xffff);
                i += 1;
            }
        }
    }
    /// Returns a string that's "regexpish" — it contains many characters
    /// typically found in regular expressions. If you call this enough
    /// times, you might get a valid regex!
    fn random_regexpish_string<R: Rng + ?Sized>(random: &mut R) -> String {
        Self::random_regexpish_string_with_len(random, 20)
    }
    const MAX_RECURSION_BOUND: usize = 5;

    /// Returns a string that's "regexpish" — it contains many characters
    /// typically found in regular expressions.
    ///
    /// If you call this enough times, you might get a valid regex!
    ///
    /// Note:
    /// To avoid practically endless backtracking patterns, this replaces `*`
    /// and `+` operators with bounded repetitions.  
    /// See LUCENE-4111 for more information.
    ///
    /// Parameters:
    /// - `max_length`: A hint for the maximum length of the regexpish string.
    ///   The result may exceed it slightly.
    fn random_regexpish_string_with_len<R: Rng + ?Sized>(random: &mut R, max_len: usize) -> String {
        let count = random.random_range(0..=max_len);
        let mut s = String::with_capacity(count);

        for _ in 0..count {
            if random.random_bool(0.5) {
                // a-z
                s.push((random.random_range(b'a'..=b'z')) as char);
            } else {
                // pick from ops
                s.push_str(OPS.choose(random).unwrap());
            }
        }
        s
    }

    /// Returns a random binary term.
    pub fn random_binary_term<AV: SharedAccessVec<u8>, R: Rng + ?Sized>(
        rng: &mut R,
    ) -> BytesRef<AV> {
        let len = rng.random_range(0..15);
        Self::random_binary_term_with_len(rng, len)
    }

    ///  Returns a random binary with a given length
    pub fn random_binary_term_with_len<AV: SharedAccessVec<u8>, R: Rng + ?Sized>(
        random: &mut R,
        length: usize,
    ) -> BytesRef<AV> {
        let mut bytes = vec![0u8; length];
        random.fill(&mut bytes[..]);
        let v = AV::from_vec(bytes);
        let mut b = BytesRef::from_bytes(v);
        b.length = length;
        b
    }
    pub fn random_substring<R: Rng + ?Sized>(
        random: &mut R,
        word_len: usize,
        simple: bool,
    ) -> String {
        if word_len == 0 {
            return String::new();
        }

        let evilness = TestUtil::next_int(random, 0, 20);

        let mut sb = String::new();
        while sb.chars().count() < word_len {
            if simple {
                if random.random_bool(0.5) {
                    sb.push_str(&Self::random_simple_string_with_len(random, word_len));
                } else {
                    sb.push_str(&Self::random_htmlish_string(random, word_len));
                }
            } else {
                match evilness {
                    0..=9 => sb.push_str(&Self::random_simple_string_with_len(random, word_len)),
                    10..=14 => {
                        assert!(sb.is_empty());
                        sb.push_str(&Self::random_realistic_unicode_string_range(
                            random, word_len, word_len,
                        ));
                    },
                    16 => sb.push_str(&Self::random_htmlish_string(random, word_len)),
                    17 => sb.push_str(&Self::random_regexpish_string_with_len(random, word_len)),
                    _ => sb.push_str(&Self::random_unicode_string_with_len(random, word_len)),
                }
            }
        }

        // truncate to exact length (UTF-16 safe, remove trailing high surrogate if
        // needed)
        let mut s: String = sb.chars().take(word_len).collect();
        if s.encode_utf16()
            .last()
            .is_some_and(|x| (0xD800..=0xDBFF).contains(&x))
        {
            s = s.chars().take(word_len - 1).collect();
        }

        if random.random_range(0..17) == 0 {
            Self::randomly_recase_codepoints(random, &s)
        } else {
            s
        }
    }

    pub fn docs<TE, R>(
        random: &mut R,
        terms_enum: &mut TE,
        reuse: Option<TE::PostingsEnum>,
        mut flags: i32,
    ) -> Result<TE::PostingsEnum>
    where
        TE: TermsEnum,
        R: Rng + ?Sized,
    {
        if random.random_bool(0.5) {
            if random.random_bool(0.5) {
                let pos_flags = match random.random_range(0..4) {
                    0 => POSITIONS,
                    1 => OFFSETS,
                    2 => PAYLOADS,
                    _ => ALL,
                };
                return terms_enum.postings_with_flags(None, pos_flags as i32);
            }

            flags |= FREQS as i32;
        }

        terms_enum.postings_with_flags(reuse, flags)
    }
}
static OPS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        ".".to_string(),
        "?".to_string(),
        format!("{{0,{}}}", TestUtil::MAX_RECURSION_BOUND), // replaces '*'
        format!("{{1,{}}}", TestUtil::MAX_RECURSION_BOUND), // replaces '+'
        "(".to_string(),
        ")".to_string(),
        "-".to_string(),
        "[".to_string(),
        "]".to_string(),
        "|".to_string(),
    ]
});
static HTML_CHAR_ENTITIES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "AElig", "Aacute", "Acirc", "Agrave", "Alpha", "AMP", "Aring", "Atilde", "Auml", "Beta",
        "COPY", "Ccedil", "Chi", "Dagger", "Delta", "ETH", "Eacute", "Ecirc", "Egrave", "Epsilon",
        "Eta", "Euml", "Gamma", "GT", "Iacute", "Icirc", "Igrave", "Iota", "Iuml", "Kappa",
        "Lambda", "LT", "Mu", "Ntilde", "Nu", "OElig", "Oacute", "Ocirc", "Ograve", "Omega",
        "Omicron", "Oslash", "Otilde", "Ouml", "Phi", "Pi", "Prime", "Psi", "QUOT", "REG", "Rho",
        "Scaron", "Sigma", "THORN", "Tau", "Theta", "Uacute", "Ucirc", "Ugrave", "Upsilon", "Uuml",
        "Xi", "Yacute", "Yuml", "Zeta", "aacute", "acirc", "acute", "aelig", "agrave", "alefsym",
        "alpha", "amp", "and", "ang", "apos", "aring", "asymp", "atilde", "auml", "bdquo", "beta",
        "brvbar", "bull", "cap", "ccedil", "cedil", "cent", "chi", "circ", "clubs", "cong", "copy",
        "crarr", "cup", "curren", "dArr", "dagger", "darr", "deg", "delta", "diams", "divide",
        "eacute", "ecirc", "egrave", "empty", "emsp", "ensp", "epsilon", "equiv", "eta", "eth",
        "euml", "euro", "exist", "fnof", "forall", "frac12", "frac14", "frac34", "frasl", "gamma",
        "ge", "gt", "hArr", "harr", "hearts", "hellip", "iacute", "icirc", "iexcl", "igrave",
        "image", "infin", "int", "iota", "iquest", "isin", "iuml", "kappa", "lArr", "lambda",
        "lang", "laquo", "larr", "lceil", "ldquo", "le", "lfloor", "lowast", "loz", "lrm",
        "lsaquo", "lsquo", "lt", "macr", "mdash", "micro", "middot", "minus", "mu", "nabla",
        "nbsp", "ndash", "ne", "ni", "not", "notin", "nsub", "ntilde", "nu", "oacute", "ocirc",
        "oelig", "ograve", "oline", "omega", "omicron", "oplus", "or", "ordf", "ordm", "oslash",
        "otilde", "otimes", "ouml", "para", "part", "permil", "perp", "phi", "pi", "piv", "plusmn",
        "pound", "prime", "prod", "prop", "psi", "quot", "rArr", "radic", "rang", "raquo", "rarr",
        "rceil", "rdquo", "real", "reg", "rfloor", "rho", "rlm", "rsaquo", "rsquo", "sbquo",
        "scaron", "sdot", "sect", "shy", "sigma", "sigmaf", "sim", "spades", "sub", "sube", "sum",
        "sup", "sup1", "sup2", "sup3", "supe", "szlig", "tau", "there4", "theta", "thetasym",
        "thinsp", "thorn", "tilde", "times", "trade", "uArr", "uacute", "uarr", "ucirc", "ugrave",
        "uml", "upsih", "upsilon", "uuml", "weierp", "xi", "yacute", "yen", "yuml", "zeta", "zwj",
        "zwnj",
    ]
});
