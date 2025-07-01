/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;
use rand::Rng;

use crate::codecs::live_docs_format::LiveDocsFormat;
use crate::codecs::{Codec, LATEST_CODEC};
use crate::index::index_writer::index_writer_util;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_info::SegmentInfo;
use crate::store::IOContext;
use crate::test::util::lucene_test_case::new_directory;
use crate::test::util::test_util::TestUtil;
use crate::util::bit_set::BitSet;
use crate::util::bits::Bits;
use crate::util::error::lucene_error::Result;
use crate::util::fixed_bit_set::FixedBitSet;
use crate::util::{StringHelper, LATEST};

pub trait BaseLiveDocsFormatTestCase {
    fn test_dense_live_docs<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let max_doc = TestUtil::next_int(random, 3, 1000);
        Self::test_serialization(random, max_doc, max_doc - 1, false)?;
        Self::test_serialization(random, max_doc, max_doc - 1, true)?;
        Ok(())
    }
    fn test_empty_live_docs<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let max_doc = TestUtil::next_int(random, 3, 1000);
        Self::test_serialization(random, max_doc, 0, false)?;
        Self::test_serialization(random, max_doc, 0, true)?;

        Ok(())
    }
    fn test_sparse_live_docs<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        let max_doc = TestUtil::next_int(random, 3, 1000);
        Self::test_serialization(random, max_doc, 1, false)?;
        Self::test_serialization(random, max_doc, 1, true)?;
        Ok(())
    }
    fn test_over_flow<R: Rng + ?Sized>(&self, random: &mut R) -> Result<()> {
        Self::test_serialization(
            random,
            index_writer_util::MAX_DOCS,
            index_writer_util::MAX_DOCS - 7,
            true,
        )?;
        Ok(())
    }

    fn test_serialization<R: Rng + ?Sized>(
        random: &mut R,
        max_doc: i32,
        num_live_docs: i32,
        fixed_bit_set: bool,
    ) -> Result<()> {
        let format = LATEST_CODEC.live_docs_format();
        let mut live_docs = FixedBitSet::new(max_doc);
        if num_live_docs > max_doc / 2 {
            live_docs.set_with_range(0, max_doc);
            for _ in 0..(max_doc - num_live_docs) {
                let mut clear_bit;
                loop {
                    clear_bit = random.random_range(0..max_doc);
                    if live_docs.get(clear_bit) {
                        break;
                    }
                }
                live_docs.clear_with_index(clear_bit);
            }
        } else {
            for _ in 0..num_live_docs {
                let mut set_bit;
                loop {
                    set_bit = random.random_range(0..max_doc);
                    if !live_docs.get(set_bit) {
                        break;
                    }
                }
                live_docs.set(set_bit);
            }
        }
        let bits = if fixed_bit_set {
            TestBitsEnum::Fixed(live_docs)
        } else {
            TestBitsEnum::Test(TestBits::new(live_docs))
        };
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let si = SegmentInfo::new(
            dir.clone(),
            Option::from(LATEST.clone()),
            Option::from(LATEST.clone()),
            "foo".to_string(),
            max_doc,
            rand::random(),
            false,
            HashMap::new(),
            StringHelper::random_id(),
            HashMap::new(),
            None,
        )?;
        let io_context = IOContext::default_io_context()?;
        let si1 = si.clone();
        let mut sci = SegmentCommitInfo::new(
            Rc::new(si),
            0,
            0,
            0,
            -1,
            -1,
            Option::from(StringHelper::random_id()),
        )?;
        format.write_live_docs(
            &bits,
            &mut *dir.lock(),
            &sci,
            max_doc - num_live_docs,
            &io_context,
        )?;

        sci = SegmentCommitInfo::new(
            Rc::new(si1),
            max_doc - num_live_docs,
            0,
            1,
            -1,
            -1,
            Option::from(StringHelper::random_id()),
        )?;
        let io_context = IOContext::read_once_io_context()?;
        let mut dir = dir.lock();
        let bits2 = format.read_live_docs(&mut *dir, &sci, &io_context)?;

        assert_eq!(max_doc, bits2.length());
        for i in 0..max_doc {
            assert_eq!(bits.get(i), bits2.get(i));
        }
        Ok(())
    }
}

pub struct TestBits {
    live_docs: FixedBitSet,
}
impl TestBits {
    pub fn new(live_docs: FixedBitSet) -> Self {
        TestBits { live_docs }
    }
}
impl Bits for TestBits {
    fn get(&self, index: i32) -> bool {
        self.live_docs.get(index)
    }

    fn length(&self) -> i32 {
        self.live_docs.length()
    }
}

enum TestBitsEnum {
    Test(TestBits),
    Fixed(FixedBitSet),
}
impl Bits for TestBitsEnum {
    fn get(&self, index: i32) -> bool {
        match self {
            TestBitsEnum::Test(test) => test.get(index),
            TestBitsEnum::Fixed(fixed) => fixed.get(index),
        }
    }
    fn length(&self) -> i32 {
        match self {
            TestBitsEnum::Test(test) => test.length(),
            TestBitsEnum::Fixed(fixed) => fixed.length(),
        }
    }
}
