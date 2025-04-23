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
use std::fmt::{Display, Formatter};

use crate::codecs::block_term_state::BlockTermState;
use crate::codecs::lucene101::for_util::ForUtil;
use crate::index::term_state::{TermState, TermStateEnum};
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;

pub struct Lucene101PostingsFormat;
impl Lucene101PostingsFormat {
    /// Filename extension for some small metadata about how postings are
    /// encoded.
    pub const META_EXTENSION: &'static str = "psm";
    /// Filename extension for document number, frequencies, and skip data.
    /// See chapter: [Frequencies and Skip Data]
    pub const DOC_EXTENSION: &'static str = "doc";

    /// Filename extension for positions.
    /// See chapter: [Positions]
    pub const POS_EXTENSION: &'static str = "pos";

    /// Filename extension for payloads and offsets.
    /// See chapter: [Payloads and Offsets]
    pub const PAY_EXTENSION: &'static str = "pay";

    /// Size of blocks.
    pub const BLOCK_SIZE: usize = ForUtil::BLOCK_SIZE;

    #[allow(dead_code)]
    pub const BLOCK_MASK: usize = Self::BLOCK_SIZE - 1;

    /// We insert skip data on every block and every SKIP_FACTOR=32 blocks.
    pub const LEVEL1_FACTOR: i32 = 32;

    /// Total number of docs covered by level 1 skip data: 32 * 128 = 4,096
    pub const LEVEL1_NUM_DOCS: i32 = Self::LEVEL1_FACTOR * Self::BLOCK_SIZE as i32;

    pub const LEVEL1_MASK: i32 = Self::LEVEL1_NUM_DOCS - 1;

    pub(crate) const TERMS_CODEC: &'static str = "Lucene90PostingsWriterTerms";
    pub(crate) const META_CODEC: &'static str = "Lucene101PostingsWriterMeta";
    pub(crate) const DOC_CODEC: &'static str = "Lucene101PostingsWriterDoc";
    pub(crate) const POS_CODEC: &'static str = "Lucene101PostingsWriterPos";
    pub(crate) const PAY_CODEC: &'static str = "Lucene101PostingsWriterPay";

    pub(crate) const VERSION_START: i32 = 0;
    pub(crate) const VERSION_CURRENT: i32 = Self::VERSION_START;
}

/// Holds all state required for
/// [`Lucene101PostingsReader`](crate::codecs::lucene101::lucene101_postings_reader)
/// to produce a [`PostingsEnum`](crate::index::postings_enum::PostingsEnum)
/// without re-seeking the terms dict.
#[derive(Default, Clone)]
pub struct IntBlockTermState {
    /// file pointer to the start of the doc ids enumeration, in
    /// [`DOC_EXTENSION`](Lucene101PostingsFormat::DOC_EXTENSION) file
    pub doc_start_fp: i64,

    /// file pointer to the start of the positions enumeration, in
    /// [`POS_EXTENSION`](Lucene101PostingsFormat::POS_EXTENSION) file
    pub pos_start_fp: i64,

    /// file pointer to the start of the payloads enumeration, in
    /// [`PAY_EXTENSION`](Lucene101PostingsFormat::PAY_EXTENSION) file
    pub pay_start_fp: i64,

    /**
     * file offset for the last position in the last block, if there are
     * more than [`BLOCK_SIZE`](crate::codecs::lucene101) positions;
     * otherwise -1
     *
     * One might think to use total term frequency to track how many
     * positions are left to read as we decode the blocks, and decode
     * the last block differently when num_left_positions < BLOCK_SIZE.
     * Unfortunately this won't work since the tracking will be messed up
     * when we skip blocks as the skipper will only tell us new
     * position offset (start of block) and number of positions to skip
     * for that block, without telling us how many positions it has
     * skipped.
     */
    pub last_pos_block_offset: i64,

    /**
     * docid when there is a single pulsed posting, otherwise -1. freq is
     * always implicitly totalTermFreq in this case.
     */
    pub singleton_doc_id: i32,

    /// Base block term state
    pub base: BlockTermState,
}

impl Display for IntBlockTermState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} docStartFP={} posStartFP={} payStartFP={} lastPosBlockOffset={} singletonDocID={}",
            self.base,
            self.doc_start_fp,
            self.pos_start_fp,
            self.pay_start_fp,
            self.last_pos_block_offset,
            self.singleton_doc_id
        )
    }
}

impl TermState for IntBlockTermState {
    fn copy_from(&mut self, other: &TermStateEnum) -> Result<()> {
        match other {
            TermStateEnum::IntBlock(other) => {
                self.doc_start_fp = other.doc_start_fp;
                self.pos_start_fp = other.pos_start_fp;
                self.pay_start_fp = other.pay_start_fp;
                self.last_pos_block_offset = other.last_pos_block_offset;
                self.singleton_doc_id = other.singleton_doc_id;
                self.base = other.base.clone();
                Ok(())
            },
            _ => Err(LuceneError::illegal_state(
                "enum other should be IntBlockTermState",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use crate::codecs::competitive_impact_accumulator::CompetitiveImpactAccumulator;
    use crate::codecs::lucene101::lucene101_postings_reader::{
        lucene101_pr_util, MutableImpactList,
    };
    use crate::codecs::lucene101::lucene101_postings_writer::lucene101_pw_util;
    use crate::index::impact::Impact;
    use crate::store::directory::Directory;
    use crate::store::{ByteArrayDataInput, ByteArrayDataOutput, DataInput, IOContext, IndexInput};
    use crate::test::index::base_index_file_format_test_case::BaseIndexFileFormatTestCase;
    use crate::test::util::lucene_test_case::{new_directory, random};
    use crate::util::error::lucene_error::Result;

    struct TestLucene101PostingsFormat;
    impl BaseIndexFileFormatTestCase for TestLucene101PostingsFormat {
        // TODO
    }
    #[test]
    fn test_vint15() -> Result<()> {
        let buffer = vec![0u8; 5];
        let mut out = ByteArrayDataOutput::with_bytes(buffer);
        for &i in &[0i32, 1, 127, 128, 32767, 32768, i32::MAX] {
            out.reset()?;
            lucene101_pw_util::write_vint15(&mut out, i)?;
            let mut inp = ByteArrayDataInput::with_bytes(out.bytes.clone());
            let v = lucene101_pr_util::read_vint15(&mut inp)?;
            assert_eq!(v, i);
            assert_eq!(inp.get_position(), out.get_position());
        }
        Ok(())
    }
    #[test]
    fn test_vlong15() -> Result<()> {
        // buffer size should accommodate the largest encoded value
        let mut out = ByteArrayDataOutput::with_bytes(vec![0u8; 9]);
        for &i in &[0i64, 1, 127, 128, 32_767, 32_768, i32::MAX as i64, i64::MAX] {
            out.reset()?;
            lucene101_pw_util::write_vlong15(&mut out, i)?;
            let mut inp = ByteArrayDataInput::with_bytes(out.bytes.clone());
            let v = lucene101_pr_util::read_vlong15(&mut inp)?;
            assert_eq!(v, i);
            assert_eq!(inp.get_position(), out.get_position());
        }
        Ok(())
    }
    #[test]
    fn test_final_block() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_impact_serialization() -> Result<()> {
        let cases = vec![
            vec![Impact { freq: 1, norm: 1 }],
            vec![Impact { freq: 1, norm: 42 }],
            vec![Impact {
                freq: 1,
                norm: -100,
            }],
            vec![Impact { freq: 30, norm: 1 }],
            vec![Impact { freq: 500, norm: 1 }],
            vec![
                Impact { freq: 1, norm: 7 },
                Impact { freq: 3, norm: 9 },
                Impact { freq: 7, norm: 10 },
                Impact { freq: 15, norm: 11 },
                Impact { freq: 20, norm: 13 },
                Impact { freq: 28, norm: 14 },
            ],
            vec![
                Impact { freq: 2, norm: 2 },
                Impact { freq: 10, norm: 10 },
                Impact { freq: 12, norm: 50 },
                Impact {
                    freq: 50,
                    norm: -100,
                },
                Impact {
                    freq: 1000,
                    norm: -80,
                },
                Impact {
                    freq: 1005,
                    norm: -3,
                },
            ],
        ];

        for impacts in cases {
            do_test_impact_serialization(&impacts)?;
        }

        Ok(())
    }
    fn do_test_impact_serialization(impacts: &[Impact]) -> Result<()> {
        let mut random = random();
        let mut acc = CompetitiveImpactAccumulator::new();
        for imp in impacts {
            acc.add(imp.freq, imp.norm);
        }
        let mut dir = new_directory(&mut random)?;
        {
            let mut out = dir.create_output("foo", &IOContext::default_io_context()?)?;
            lucene101_pw_util::write_impacts(&acc.get_competitive_freq_norm_pairs(), &mut out)?;
        }
        let mut input = dir.open_input("foo", &IOContext::default_io_context()?)?;
        let len = input.length();
        let mut buffer = vec![0u8; len as usize];
        input.read_bytes(&mut buffer, 0, len as i32)?;

        let mut data_in = ByteArrayDataInput::with_bytes(buffer);
        let mut mutable_impacts_list =
            MutableImpactList::with_capacity(impacts.len() + random.random_range(0..3));
        let impacts2 = lucene101_pr_util::read_impacts(&mut data_in, &mut mutable_impacts_list)?;

        assert_eq!(impacts2, impacts);
        Ok(())
    }
}
