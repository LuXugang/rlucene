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
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::codecs::indexed_disi::IndexedDISI;
use crate::codecs::lucene90::dov_values_inner_enum::{
    BaseSortedDocValuesEnum, BaseSortedSetDocValuesEnum, DenseBinaryDocValuesBaseEnum,
    DenseNumericDocValuesSubEnum, LongValuesEnum, SparseBinaryDocValuesBaseEnum,
    SparseNumericDocValuesSubEnum,
};
use crate::codecs::lucene90::lucene90_doc_values_enums::{
    Lucene90NumericDocValuesEnums, Lucene90SortedNumericDocValuesEnums,
    Lucene90SortedSetDocValuesEnum,
};
use crate::codecs::lucene90_doc_values_enums::Lucene90BinaryDocValuesEnum;
use crate::codecs::lucene90_doc_values_format::{
    Lucene90DocValuesFormat, SKIP_INDEX_JUMP_LENGTH_PER_LEVEL,
};
use crate::codecs::CodecUtil;
use crate::index::binary_doc_values::BinaryDocValues;
use crate::index::doc_values::{DocValues, EmptyBinary, EmptyNumeric};
use crate::index::doc_values_iterator::DocValuesIterator;
use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_skipper::DocValuesSkipper;
use crate::index::dummy::dummy_impacts_enum::DummyImpactsEnum;
use crate::index::dummy::dummy_postings_enum::DummyPostingsEnum;
use crate::index::dummy::dummy_term_state_type::DummyTermState;
use crate::index::dummy::dummy_terms_enum::DummyTermsEnum;
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::FieldInfos;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::index::segment_read_state::SegmentReadState;
use crate::index::sorted_doc_values::SortedDocValues;
use crate::index::sorted_numeric_doc_values::SortedNumericDocValues;
use crate::index::sorted_set_doc_values::SortedSetDocValues;
use crate::index::term_state::TermStateEnum;
use crate::index::terms_enum::{SeekStatus, TermsEnum};
use crate::index::{BytesRef, IndexFileNames};
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::store::directory::Directory;
use crate::store::random_access_input::RandomAccessInput;
use crate::store::{ByteArrayDataInput, DataInput, IndexInput, ReadAdvice};
use crate::util::access::AccessVec;
use crate::util::attribute_source::AttributeSource;
use crate::util::bit_util::BitUtil;
use crate::util::bytes_ref_iterator::BytesRefIterator;
use crate::util::compress::lz4::LZ4;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use crate::util::long_values::{LongValues, Zeroes};
use crate::util::packed::direct_monotonic_reader::direct_monotonic::Meta;
use crate::util::packed::direct_monotonic_reader::{
    direct_monotonic_reader_util, DirectMonotonicReader,
};
use crate::util::packed::direct_reader::{DirectPackedEnum, DirectReader};
use crate::util::{SliceCopyOps, ToInt};

pub struct Lucene90DocValuesProducer<I>
where
    I: IndexInput,
{
    numerics: HashMap<i32, Rc<NumericEntry>>,
    binaries: HashMap<i32, Rc<BinaryEntry>>,
    sorted: HashMap<i32, Rc<SortedEntry>>,
    sorted_sets: HashMap<i32, Rc<SortedSetEntry>>,
    sorted_numerics: HashMap<i32, Rc<SortedNumericEntry>>,
    skippers: HashMap<i32, Rc<DocValuesSkipperEntry>>,
    data: I,
    max_doc: i32,
    version: i32,
    merging: bool,
}

impl<I> Lucene90DocValuesProducer<I>
where
    I: IndexInput,
{
    pub fn new<D>(
        state: &SegmentReadState<D>,
        data_codec: &str,
        data_extension: &str,
        meta_codec: &str,
        meta_extension: &str,
    ) -> Result<Self>
    where
        D: Directory<IndexInputType = I>,
    {
        let meta_name = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            meta_extension,
        );

        let max_doc = state.segment_info.max_doc()?;
        let mut version = -1;

        let mut numerics = HashMap::new();
        let mut binaries = HashMap::new();
        let mut sorted = HashMap::new();
        let mut sorted_sets = HashMap::new();
        let mut sorted_numerics = HashMap::new();
        let mut skippers = HashMap::new();

        {
            let dir = state.directory.lock();
            let mut input = dir.open_checksum_input(&meta_name)?;

            let mut prior_error = None;

            match CodecUtil::check_index_header(
                &mut input,
                meta_codec,
                Lucene90DocValuesFormat::VERSION_START,
                Lucene90DocValuesFormat::VERSION_CURRENT,
                &state.segment_info.get_id(),
                &state.segment_suffix,
            ) {
                Ok(v) => {
                    version = v;
                    Self::read_fields(
                        &mut input,
                        &state.field_infos,
                        &mut numerics,
                        &mut binaries,
                        &mut sorted,
                        &mut sorted_sets,
                        &mut sorted_numerics,
                        &mut skippers,
                    )?;
                },
                Err(e) => {
                    prior_error = Some(e);
                },
            }

            if let Some(e) = prior_error.as_mut() {
                return Err(CodecUtil::check_footer_with_error(&mut input, e));
            } else {
                CodecUtil::check_footer(&mut input)?;
            }
        }

        let data_name = IndexFileNames::segment_file_name(
            &state.segment_info.name,
            &state.segment_suffix,
            data_extension,
        );
        // Doc-values have a forward-only access pattern, so pass
        // ReadAdvice.NORMAL to perform readahead.
        let mut data;
        {
            let dir = state.directory.lock();
            data = dir.open_input(
                &data_name,
                &state.context.with_read_advice(ReadAdvice::Normal)?,
            )?;
        }

        let version2 = CodecUtil::check_index_header(
            &mut data,
            data_codec,
            Lucene90DocValuesFormat::VERSION_START,
            Lucene90DocValuesFormat::VERSION_CURRENT,
            &state.segment_info.get_id(),
            &state.segment_suffix,
        )?;

        if version != version2 {
            return Err(LuceneError::corrupt_index(format!(
                "Format versions mismatch: meta={}, data={} (resource={})",
                version, version2, data
            )));
        }
        // NOTE: data file is too costly to verify checksum against all the
        // bytes on open, but for now we at least verify proper
        // structure of the checksum footer: which looks
        // for FOOTER_MAGIC + algorithmID. This is cheap and can detect some
        // forms of corruption such as file truncation.
        CodecUtil::retrieve_checksum(&mut data)?;

        Ok(Self {
            numerics,
            binaries,
            sorted,
            sorted_sets,
            sorted_numerics,
            skippers,
            data,
            max_doc,
            version,
            merging: false,
        })
    }
    #[allow(clippy::too_many_arguments)]
    fn new_with_merging(
        numerics: HashMap<i32, Rc<NumericEntry>>,
        binaries: HashMap<i32, Rc<BinaryEntry>>,
        sorted: HashMap<i32, Rc<SortedEntry>>,
        sorted_sets: HashMap<i32, Rc<SortedSetEntry>>,
        sorted_numerics: HashMap<i32, Rc<SortedNumericEntry>>,
        skippers: HashMap<i32, Rc<DocValuesSkipperEntry>>,
        data: &I,
        max_doc: i32,
        version: i32,
    ) -> Result<Self> {
        Ok(Self {
            numerics,
            binaries,
            sorted,
            sorted_sets,
            sorted_numerics,
            skippers,
            data: data.try_clone()?,
            max_doc,
            version,
            merging: true,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn read_fields(
        meta: &mut impl IndexInput,
        infos: &Rc<FieldInfos>,
        numerics: &mut HashMap<i32, Rc<NumericEntry>>,
        binaries: &mut HashMap<i32, Rc<BinaryEntry>>,
        sorted: &mut HashMap<i32, Rc<SortedEntry>>,
        sorted_sets: &mut HashMap<i32, Rc<SortedSetEntry>>,
        sorted_numerics: &mut HashMap<i32, Rc<SortedNumericEntry>>,
        skippers: &mut HashMap<i32, Rc<DocValuesSkipperEntry>>,
    ) -> Result<()> {
        loop {
            let field_number = meta.read_int()?;
            if field_number == -1 {
                break;
            }
            let info = infos.field_info_by_number(field_number)?;
            match info {
                Some(info) => {
                    let type_byte = meta.read_byte()?;

                    if info.doc_values_skip_index_type() != &DocValuesSkipIndexType::None {
                        let skipper = Rc::new(Self::read_doc_value_skipper_meta(meta)?);
                        skippers.insert(info.number, skipper);
                    }

                    match type_byte {
                        t if t == Lucene90DocValuesFormat::NUMERIC => {
                            let entry = Rc::new(Self::read_numeric(meta)?);
                            numerics.insert(info.number, entry);
                        },
                        t if t == Lucene90DocValuesFormat::BINARY => {
                            let entry = Self::read_binary(meta)?;
                            binaries.insert(info.number, Rc::new(entry));
                        },
                        t if t == Lucene90DocValuesFormat::SORTED => {
                            let entry = Rc::new(Self::read_sorted(meta)?);
                            sorted.insert(info.number, entry);
                        },
                        t if t == Lucene90DocValuesFormat::SORTED_SET => {
                            let entry = Rc::new(Self::read_sorted_set(meta)?);
                            sorted_sets.insert(info.number, entry);
                        },
                        t if t == Lucene90DocValuesFormat::SORTED_NUMERIC => {
                            let entry = Rc::new(Self::read_sorted_numeric(meta)?);
                            sorted_numerics.insert(info.number, entry);
                        },
                        _ => {
                            return Err(LuceneError::corrupt_index(format!(
                                "Invalid doc values type: {}",
                                type_byte
                            )));
                        },
                    }
                },
                None => {
                    return Err(LuceneError::corrupt_index(format!(
                        "Field number {} not found in field infos, resource {}",
                        field_number, meta
                    )));
                },
            }
        }
        Ok(())
    }
    fn read_numeric(meta: &mut impl IndexInput) -> Result<NumericEntry> {
        let mut entry = NumericEntry::default();
        Self::read_numeric_with_entry(meta, &mut entry)?;
        Ok(entry)
    }
    fn read_doc_value_skipper_meta(meta: &mut impl IndexInput) -> Result<DocValuesSkipperEntry> {
        let offset = meta.read_long()?;
        let length = meta.read_long()?;
        let max_value = meta.read_long()?;
        let min_value = meta.read_long()?;
        let doc_count = meta.read_int()?;
        let max_doc_id = meta.read_int()?;

        Ok(DocValuesSkipperEntry {
            offset,
            length,
            min_value,
            max_value,
            doc_count,
            max_doc_id,
        })
    }
    fn read_numeric_with_entry(meta: &mut impl IndexInput, entry: &mut NumericEntry) -> Result<()> {
        entry.docs_with_field_offset = meta.read_long()?;
        entry.docs_with_field_length = meta.read_long()?;
        entry.jump_table_entry_count = meta.read_short()?;
        entry.dense_rank_power = meta.read_byte()? as i8;
        entry.num_values = meta.read_long()?;

        let table_size = meta.read_int()?;
        if table_size > 256 {
            return Err(LuceneError::corrupt_index(format!(
                "invalid table size: {} resource {}",
                table_size, meta
            )));
        }

        entry.table = if table_size >= 0 {
            let mut table = Vec::with_capacity(table_size as usize);
            for _ in 0..table_size {
                table.push(meta.read_long()?);
            }
            Option::from(Rc::new(table))
        } else {
            Some(Rc::new(Vec::new()))
        };

        entry.block_shift = if table_size < -1 { -2 - table_size } else { -1 };

        entry.bits_per_value = meta.read_byte()?;
        entry.min_value = meta.read_long()?;
        entry.gcd = meta.read_long()?;
        entry.values_offset = meta.read_long()?;
        entry.values_length = meta.read_long()?;
        entry.value_jump_table_offset = meta.read_long()?;
        Ok(())
    }
    fn read_binary(meta: &mut impl IndexInput) -> Result<BinaryEntry> {
        let data_offset = meta.read_long()?;
        let data_length = meta.read_long()?;
        let docs_with_field_offset = meta.read_long()?;
        let docs_with_field_length = meta.read_long()?;
        let jump_table_entry_count = meta.read_short()?;
        let dense_rank_power = meta.read_byte()?;
        let num_docs_with_field = meta.read_int()?;
        let min_length = meta.read_int()?;
        let max_length = meta.read_int()?;

        let mut addresses_offset = 0;
        let mut addresses_meta = None;
        let mut addresses_length = 0;

        if min_length < max_length {
            addresses_offset = meta.read_long()?;
            // Old count of uncompressed addresses
            let num_addresses = num_docs_with_field as i64 + 1;
            let block_shift = meta.read_vint()?; // 注意这里是 VInt
            addresses_meta = Some(direct_monotonic_reader_util::load_meta(
                meta,
                num_addresses,
                block_shift,
            )?);
            addresses_length = meta.read_long()?;
        }

        Ok(BinaryEntry {
            data_offset,
            data_length,
            docs_with_field_offset,
            docs_with_field_length,
            jump_table_entry_count,
            dense_rank_power,
            num_docs_with_field,
            min_length,
            max_length,
            addresses_offset,
            addresses_length,
            addresses_meta,
        })
    }
    fn read_sorted(meta: &mut impl IndexInput) -> Result<SortedEntry> {
        let mut ords_entry = NumericEntry::default();
        Self::read_numeric_with_entry(meta, &mut ords_entry)?;
        let mut terms_dict_entry = TermsDictEntry::default();
        Self::read_term_dict_with_entry(meta, &mut terms_dict_entry)?;
        Ok(SortedEntry {
            ords_entry: Rc::new(ords_entry),
            terms_dict_entry: Rc::new(terms_dict_entry),
        })
    }
    fn read_sorted_set(meta: &mut impl IndexInput) -> Result<SortedSetEntry> {
        let multi_valued = meta.read_byte()?;
        let mut entry = match multi_valued {
            0 => {
                let single_value_entry = Rc::new(Self::read_sorted(meta)?);
                SortedSetEntry {
                    single_value_entry: Some(single_value_entry),
                    ords_entry: None,
                    terms_dict_entry: None,
                }
            },
            1 => SortedSetEntry::default(),
            _ => {
                return Err(LuceneError::corrupt_index(format!(
                    "Invalid multiValued flag: {} resource {}",
                    multi_valued, meta
                )))
            },
        };
        let mut ords_entry = SortedNumericEntry::default();
        Self::read_sorted_numeric_with_entry(meta, &mut ords_entry)?;
        let mut terms_dict_entry = TermsDictEntry::default();
        // The definitions of terms_addresses_meta and
        // terms_index_addresses_meta are set as Option for ease of
        // initialization. However, in the current implementation, these
        // two values are guaranteed not to be None, so we add an assert here.
        debug_assert!(terms_dict_entry.terms_addresses_meta.is_some());
        debug_assert!(terms_dict_entry.terms_index_addresses_meta.is_some());
        Self::read_term_dict_with_entry(meta, &mut terms_dict_entry)?;

        entry.ords_entry = Some(Rc::new(ords_entry));
        entry.terms_dict_entry = Some(Rc::new(terms_dict_entry));
        Ok(entry)
    }
    fn read_term_dict_with_entry(
        meta: &mut impl IndexInput,
        entry: &mut TermsDictEntry,
    ) -> Result<()> {
        entry.terms_dict_size = meta.read_vlong()?;
        let block_shift = meta.read_int()?;

        let addresses_size = (entry.terms_dict_size
            + (1 << Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT)
            - 1)
            >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT;

        entry.terms_addresses_meta = Some(direct_monotonic_reader_util::load_meta(
            meta,
            addresses_size,
            block_shift,
        )?);

        entry.max_term_length = meta.read_int()?;
        entry.max_block_length = meta.read_int()?;
        entry.terms_data_offset = meta.read_long()?;
        entry.terms_data_length = meta.read_long()?;
        entry.terms_addresses_offset = meta.read_long()?;
        entry.terms_addresses_length = meta.read_long()?;
        entry.terms_dict_index_shift = meta.read_int()?;

        let index_size = (entry.terms_dict_size + (1 << entry.terms_dict_index_shift) - 1)
            >> entry.terms_dict_index_shift;

        entry.terms_index_addresses_meta = Some(direct_monotonic_reader_util::load_meta(
            meta,
            1 + index_size,
            block_shift,
        )?);
        entry.terms_index_offset = meta.read_long()?;
        entry.terms_index_length = meta.read_long()?;
        entry.terms_index_addresses_offset = meta.read_long()?;
        entry.terms_index_addresses_length = meta.read_long()?;

        Ok(())
    }
    fn read_sorted_numeric(meta: &mut impl IndexInput) -> Result<SortedNumericEntry> {
        let mut entry = SortedNumericEntry::default();
        Self::read_sorted_numeric_with_entry(meta, &mut entry)?;
        Ok(entry)
    }
    fn read_sorted_numeric_with_entry(
        meta: &mut impl IndexInput,
        entry: &mut SortedNumericEntry,
    ) -> Result<()> {
        debug_assert!(*entry.base == NumericEntry::default());
        let mut numeric_entry = NumericEntry::default();
        Self::read_numeric_with_entry(meta, &mut numeric_entry)?;
        entry.base = Rc::new(numeric_entry);
        entry.num_docs_with_field = meta.read_int()?;
        entry.addresses_offset = 0;
        entry.addresses_meta = None;
        entry.addresses_length = 0;

        if entry.num_docs_with_field as i64 != entry.base.num_values {
            entry.addresses_offset = meta.read_long()?;
            let block_shift = meta.read_vint()?;
            entry.addresses_meta = Some(direct_monotonic_reader_util::load_meta(
                meta,
                entry.num_docs_with_field as i64 + 1,
                block_shift,
            )?);
            entry.addresses_length = meta.read_long()?;
        }
        Ok(())
    }

    fn get_numeric(&mut self, entry: Rc<NumericEntry>) -> Result<Lucene90NumericDocValuesEnums<I>> {
        if entry.docs_with_field_offset == -2 {
            // empty
            Ok(Lucene90NumericDocValuesEnums::Empty(EmptyNumeric::new()))
        } else if entry.docs_with_field_offset == -1 {
            // dense
            let dense_numeric_doc_values_base_enum = if entry.bits_per_value == 0 {
                DenseNumericDocValuesSubEnum::Dense(DenseNumericDocValuesBaseImpl {
                    min_values: entry.min_value,
                })
            } else {
                let mut slice = self
                    .data
                    .random_access_slice(entry.values_offset, entry.values_length)?;
                // Prefetch the first page of data. Following pages are expected
                // to get prefetched through read-ahead.
                if slice.length() > 0 {
                    slice.prefetch(0, 1)?
                }
                if entry.block_shift >= 0 {
                    let vbpv_reader =
                        VaryingBPVReader::new(entry.clone(), slice, &mut self.data, self.merging)?;
                    DenseNumericDocValuesSubEnum::Dense1(DenseNumericDocValuesBaseImpl1 {
                        vbpv_reader,
                    })
                } else {
                    let values = lucene90_dvp_util::get_direct_reader_instance::<I>(
                        self.merging,
                        Rc::new(RefCell::new(slice)),
                        entry.bits_per_value as i32,
                        9,
                        entry.num_values,
                    )?;
                    match entry.table {
                        Some(ref table) => {
                            DenseNumericDocValuesSubEnum::Dense2(DenseNumericDocValuesBaseImpl2 {
                                table: table.clone(),
                                values,
                            })
                        },
                        None => {
                            if entry.gcd == 1 && entry.min_value == 0 {
                                DenseNumericDocValuesSubEnum::Dense3(
                                    DenseNumericDocValuesBaseImpl3 { values },
                                )
                            } else {
                                DenseNumericDocValuesSubEnum::Dense4(
                                    DenseNumericDocValuesBaseImpl4 {
                                        values,
                                        mul: entry.gcd,
                                        delta: entry.min_value,
                                    },
                                )
                            }
                        },
                    }
                }
            };
            return Ok(Lucene90NumericDocValuesEnums::Dense(
                DenseNumericDocValues::new(dense_numeric_doc_values_base_enum, self.max_doc),
            ));
        } else {
            let disi = IndexedDISI::new(
                &mut self.data,
                entry.docs_with_field_offset,
                entry.docs_with_field_length,
                entry.jump_table_entry_count as i32,
                entry.dense_rank_power,
                entry.num_values,
            )?;
            let sparse_numeric_doc_values_base_enum = if entry.bits_per_value == 0 {
                SparseNumericDocValuesSubEnum::Sparse(SparseNumericDocValuesBaseImpl {
                    min_values: entry.min_value,
                    _phantom: Default::default(),
                })
            } else {
                let mut slice = self
                    .data
                    .random_access_slice(entry.values_offset, entry.values_length)?;
                // Prefetch the first page of data. Following pages are expected
                // to get prefetched through read-ahead.
                if slice.length() > 0 {
                    slice.prefetch(0, 1)?
                }
                if entry.block_shift >= 0 {
                    SparseNumericDocValuesSubEnum::Sparse1(SparseNumericDocValuesBaseImpl1 {
                        vbpv_reader: VaryingBPVReader::new(
                            entry.clone(),
                            slice,
                            &mut self.data,
                            self.merging,
                        )?,
                    })
                } else {
                    let values = lucene90_dvp_util::get_direct_reader_instance::<I>(
                        self.merging,
                        Rc::new(RefCell::new(slice)),
                        entry.bits_per_value as i32,
                        0,
                        entry.num_values,
                    )?;
                    match entry.table {
                        Some(ref table) => SparseNumericDocValuesSubEnum::Sparse2(
                            SparseNumericDocValuesBaseImpl2 {
                                table: table.clone(),
                                values,
                            },
                        ),
                        None => {
                            if entry.gcd == 1 && entry.min_value == 0 {
                                SparseNumericDocValuesSubEnum::Sparse3(
                                    SparseNumericDocValuesBaseImpl3 { values },
                                )
                            } else {
                                SparseNumericDocValuesSubEnum::Sparse4(
                                    SparseNumericDocValuesBaseImpl4 {
                                        values,
                                        mul: entry.gcd,
                                        delta: entry.min_value,
                                    },
                                )
                            }
                        },
                    }
                }
            };
            Ok(Lucene90NumericDocValuesEnums::Sparse(
                SparseNumericDocValues::new(sparse_numeric_doc_values_base_enum, disi),
            ))
        }
    }
    fn get_numeric_values(&mut self, entry: &Rc<NumericEntry>) -> Result<LongValuesEnum<I>>
    where
        I: IndexInput,
    {
        let long_values = if entry.bits_per_value == 0 {
            LongValuesEnum::Impl(LongValuesImpl {
                min_values: entry.min_value,
            })
        } else {
            let mut slice = self
                .data
                .random_access_slice(entry.values_offset, entry.values_length)?;
            if slice.length() > 0 {
                slice.prefetch(0, 1)?
            }
            if entry.block_shift >= 0 {
                LongValuesEnum::Impl1(LongValuesImpl1 {
                    vbpv_reader: VaryingBPVReader::new(
                        entry.clone(),
                        slice,
                        &mut self.data,
                        self.merging,
                    )?,
                })
            } else {
                let values = lucene90_dvp_util::get_direct_reader_instance::<I>(
                    self.merging,
                    Rc::new(RefCell::new(slice)),
                    entry.bits_per_value as i32,
                    9,
                    entry.num_values,
                )?;
                match entry.table {
                    Some(ref table) => LongValuesEnum::Impl2(LongValuesImpl2 {
                        table: table.clone(),
                        values,
                    }),
                    None => {
                        if entry.gcd == 1 && entry.min_value == 0 {
                            LongValuesEnum::Impl3(LongValuesImpl3 {
                                values,
                                gcd: entry.gcd,
                                min_value: entry.min_value,
                            })
                        } else {
                            LongValuesEnum::Impl4(LongValuesImpl4 {
                                values,
                                min_value: entry.min_value,
                            })
                        }
                    },
                }
            }
        };
        Ok(long_values)
    }

    fn get_sorted(&mut self, entry: Rc<SortedEntry>) -> Result<BaseSortedDocValues<I>> {
        let ords_entry = &entry.ords_entry;

        if ords_entry.block_shift < 0 && ords_entry.bits_per_value > 0 {
            if ords_entry.gcd != 1 || ords_entry.min_value != 0 || ords_entry.table.is_some() {
                return Err(LuceneError::illegal_state(
                    "Ordinals shouldn't use GCD, offset or table compression",
                ));
            }

            let mut slice = self
                .data
                .random_access_slice(ords_entry.values_offset, ords_entry.values_length)?;
            if slice.length() > 0 {
                slice.prefetch(0, 1)?;
            }

            let values = lucene90_dvp_util::get_direct_reader_instance::<I>(
                self.merging,
                Rc::new(RefCell::new(slice)),
                ords_entry.bits_per_value as i32,
                0,
                ords_entry.num_values,
            )?;

            let sub = if ords_entry.docs_with_field_offset == -1 {
                //dense
                BaseSortedDocValuesEnum::Dense(DenseBaseSortedDocValues::new(self.max_doc, values))
            } else {
                let disi = IndexedDISI::new(
                    &mut self.data,
                    ords_entry.docs_with_field_offset,
                    ords_entry.docs_with_field_length,
                    ords_entry.jump_table_entry_count as i32,
                    ords_entry.dense_rank_power,
                    ords_entry.num_values,
                )?;
                BaseSortedDocValuesEnum::Sparse(SparseBaseSortedDocValues::new(disi, values))
            };
            return BaseSortedDocValues::new(entry.clone(), &mut self.data, sub, self.merging);
        }

        let ords = self.get_numeric(ords_entry.clone())?;
        let sub = BaseSortedDocValuesEnum::Impl(BaseSortedDocValuesImpl::new(ords));
        BaseSortedDocValues::new(entry.clone(), &mut self.data, sub, self.merging)
    }

    fn get_sorted_numeric(
        &mut self,
        entry: &SortedNumericEntry,
    ) -> Result<Lucene90SortedNumericDocValuesEnums<I>>
    where
        I: IndexInput,
    {
        if entry.base.num_values == entry.num_docs_with_field as i64 {
            return Ok(Lucene90SortedNumericDocValuesEnums::Singleton(
                DocValues::singleton_numeric(self.get_numeric(entry.base.clone())?)?,
            ));
        }

        let mut addresses_input = self
            .data
            .random_access_slice(entry.addresses_offset, entry.addresses_length)?;
        // Prefetch the first page of data. Following pages are expected to get
        // prefetched through read-ahead.
        if addresses_input.length() > 0 {
            addresses_input.prefetch(0, 1)?;
        }

        let addresses = match entry.addresses_meta {
            Some(ref meta) => DirectMonotonicReader::get_instance_with_merging(
                meta,
                Rc::new(RefCell::new(addresses_input)),
                self.merging,
            )?,
            None => {
                return Err(LuceneError::illegal_state(
                    "addresses_meta is None".to_string(),
                ))?;
            },
        };

        let values = self.get_numeric_values(&entry.base)?;

        if entry.base.docs_with_field_offset == -1 {
            // dense
            Ok(Lucene90SortedNumericDocValuesEnums::Dense(
                DenseSortedNumericDocValues::new(self.max_doc, 0, 0, values, addresses),
            ))
        } else {
            // sparse
            let disi = IndexedDISI::new(
                &mut self.data,
                entry.base.docs_with_field_offset,
                entry.base.docs_with_field_length,
                entry.base.jump_table_entry_count as i32,
                entry.base.dense_rank_power,
                entry.num_docs_with_field as i64,
            )?;

            Ok(Lucene90SortedNumericDocValuesEnums::Sparse(
                SpareSortedNumericDocValues::new(disi, values, addresses),
            ))
        }
    }
}
pub mod lucene90_dvp_util {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::store::IndexInput;
    use crate::util::error::lucene_error::Result;
    use crate::util::packed::direct_reader::{DirectPackedEnum, DirectReader};

    pub(super) fn get_direct_reader_instance<I: IndexInput>(
        merging: bool,
        slice: Rc<RefCell<I::RandomAccessSlice>>,
        bits_per_value: i32,
        offset: i64,
        num_values: i64,
    ) -> Result<DirectPackedEnum<I::RandomAccessSlice>>
    where
        I: IndexInput,
    {
        if merging {
            Ok(DirectReader::get_merge_instance_with_base_offset(
                slice,
                bits_per_value,
                offset,
                num_values,
            ))
        } else {
            Ok(DirectReader::get_instance_with_offset(
                slice,
                bits_per_value,
                offset,
            ))
        }
    }
}
impl<I> DocValuesProducer for Lucene90DocValuesProducer<I>
where
    I: IndexInput,
{
    type NumericDocValues = Lucene90NumericDocValuesEnums<I>;

    fn get_numeric(&mut self, field: &Rc<FieldInfo>) -> Result<Lucene90NumericDocValuesEnums<I>> {
        let entry = self.numerics.get(&field.number);
        match entry {
            Some(entry) => self.get_numeric(entry.clone()),
            None => Err(LuceneError::illegal_state(format!(
                "Missing numeric entry for field {}",
                field.number
            ))),
        }
    }

    type BinaryDocValues = Lucene90BinaryDocValuesEnum<I>;

    fn get_binary(&mut self, field: &Rc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        let entry = self.binaries.get(&field.number);
        match entry {
            Some(entry) => {
                if entry.docs_with_field_offset == -1 {
                    return Ok(Lucene90BinaryDocValuesEnum::Empty(EmptyBinary::new()));
                }
                let mut bytes_slice = self
                    .data
                    .random_access_slice(entry.data_offset, entry.data_length)?;
                // Prefetch the first page of data. Following pages are expected
                // to get prefetched through read-ahead.
                if bytes_slice.length() > 0 {
                    bytes_slice.prefetch(0, 1)?;
                }
                if entry.docs_with_field_offset == -1 {
                    let dense = if entry.min_length == entry.max_length {
                        // fixed length
                        let vec = vec![0u8; entry.max_length as usize];
                        let base = DenseBinaryDocValuesBaseImpl {
                            bytes_slice,
                            length: entry.max_length,
                            bytes: BytesRef::from_slice(vec, 0, entry.max_length as usize),
                        };
                        DenseBinaryDocValuesBaseEnum::Dense(base)
                    } else {
                        let mut addresses_data = self
                            .data
                            .random_access_slice(entry.data_offset, entry.data_length)?;
                        // Prefetch the first page of data. Following pages are
                        // expected to get prefetched through
                        // read-ahead.
                        if addresses_data.length() > 0 {
                            addresses_data.prefetch(0, 1)?;
                        }
                        match entry.addresses_meta {
                            Some(ref meta) => {
                                let addresses = DirectMonotonicReader::get_instance_with_merging(
                                    meta,
                                    Rc::new(RefCell::new(addresses_data)),
                                    self.merging,
                                )?;
                                let vec = vec![0u8; entry.max_length as usize];
                                let base = DenseBinaryDocValuesBaseImpl1 {
                                    bytes_slice,
                                    bytes: BytesRef::from_slice(vec, 0, entry.max_length as usize),
                                    addresses,
                                };
                                DenseBinaryDocValuesBaseEnum::Dense1(base)
                            },
                            None => {
                                return Err(LuceneError::illegal_state(
                                    "addresses_meta is None".to_string(),
                                ))?;
                            },
                        }
                    };
                    Ok(Lucene90BinaryDocValuesEnum::Dense(
                        DenseBinaryDocValues::new(dense, self.max_doc),
                    ))
                } else {
                    let disi = IndexedDISI::new(
                        &mut self.data,
                        entry.docs_with_field_offset,
                        entry.docs_with_field_length,
                        entry.jump_table_entry_count as i32,
                        entry.dense_rank_power as i8,
                        entry.num_docs_with_field as i64,
                    )?;

                    let sub = if entry.min_length == entry.max_length {
                        // fixed-length
                        let length = entry.max_length;
                        SparseBinaryDocValuesBaseEnum::Sparse(SparseBinaryDocValuesBaseImpl {
                            bytes_slice,
                            bytes: BytesRef::from_slice(
                                vec![0u8; length as usize],
                                0,
                                length as usize,
                            ),
                            length,
                        })
                    } else {
                        // variable-length
                        let mut addresses_data = self
                            .data
                            .random_access_slice(entry.addresses_offset, entry.addresses_length)?;
                        if addresses_data.length() > 0 {
                            addresses_data.prefetch(0, 1)?;
                        }
                        let addresses = match entry.addresses_meta {
                            Some(ref meta) => DirectMonotonicReader::get_instance_with_merging(
                                meta,
                                Rc::new(RefCell::new(addresses_data)),
                                self.merging,
                            )?,
                            None => {
                                return Err(LuceneError::illegal_state(
                                    "addresses_meta is None".to_string(),
                                ))?;
                            },
                        };
                        SparseBinaryDocValuesBaseEnum::Sparse1(SparseBinaryDocValuesBaseImpl1 {
                            bytes_slice,
                            bytes: BytesRef::from_slice(
                                vec![0u8; entry.max_length as usize],
                                0,
                                entry.max_length as usize,
                            ),
                            addresses,
                        })
                    };
                    Ok(Lucene90BinaryDocValuesEnum::Sparse(
                        SparseBinaryDocValues::new(sub, disi),
                    ))
                }
            },
            None => Err(LuceneError::illegal_state(format!(
                "Missing binary entry for field {}",
                field.number
            ))),
        }
    }

    type SortedDocValues = BaseSortedDocValues<I>;

    fn get_sorted(&mut self, field: &Rc<FieldInfo>) -> Result<Self::SortedDocValues> {
        let entry = self.sorted.get(&field.number);
        match entry {
            Some(entry) => Ok(self.get_sorted(entry.clone())?),
            None => Err(LuceneError::illegal_state(format!(
                "Missing sorted entry for field {}",
                field.number
            ))),
        }
    }

    type SortedNumericDocValues = Lucene90SortedNumericDocValuesEnums<I>;

    fn get_sorted_numeric(
        &mut self,
        field: &Rc<FieldInfo>,
    ) -> Result<Self::SortedNumericDocValues> {
        let entry = self.sorted_numerics.get(&field.number);
        match entry {
            Some(entry) => self.get_sorted_numeric(&entry.clone()),
            None => Err(LuceneError::illegal_state(format!(
                "Missing sorted numeric entry for field {}",
                field.number
            ))),
        }
    }

    type SortedSetDocValues = Lucene90SortedSetDocValuesEnum<I>;

    fn get_sorted_set(&mut self, field: &Rc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        let field_number = field.number;
        let entry = self.sorted_sets.get(&field_number);
        match entry {
            Some(entry) => {
                let entry_clone = entry.clone();
                if let Some(ref single_value_entry) = entry.single_value_entry {
                    let singleton =
                        DocValues::singleton_sorted(self.get_sorted(single_value_entry.clone())?)?;
                    return Ok(Lucene90SortedSetDocValuesEnum::Singleton(singleton));
                }
                // Specialize the common case for ordinals: single block of
                // packed integers.
                match entry.ords_entry {
                    Some(ref ords_entry) => {
                        let ords_entry_clone = ords_entry.clone();
                        if ords_entry.base.block_shift < 0 && ords_entry.base.bits_per_value > 0 {
                            if ords_entry.base.gcd != 1
                                || ords_entry.base.min_value != 0
                                || ords_entry.base.table.is_some()
                            {
                                return Err(LuceneError::illegal_state(
                                    "Ordinals shouldn't use GCD, offset or table compression",
                                ));
                            }

                            let mut addresses_input = self.data.random_access_slice(
                                ords_entry.addresses_offset,
                                ords_entry.addresses_length,
                            )?;
                            if addresses_input.length() > 0 {
                                addresses_input.prefetch(0, 1)?;
                            }
                            let addresses = match ords_entry.addresses_meta {
                                Some(ref meta) => DirectMonotonicReader::get_instance_with_merging(
                                    meta,
                                    Rc::new(RefCell::new(addresses_input)),
                                    self.merging,
                                )?,
                                None => {
                                    return Err(LuceneError::illegal_state(
                                        "addresses_meta is None".to_string(),
                                    ))?;
                                },
                            };

                            let mut slice = self.data.random_access_slice(
                                ords_entry.base.values_offset,
                                ords_entry.base.values_length,
                            )?;
                            if slice.length() > 0 {
                                slice.prefetch(0, 1)?;
                            }
                            let values = DirectReader::get_instance(
                                Rc::new(RefCell::new(slice)),
                                ords_entry.base.bits_per_value as i32,
                            );

                            let sbu = if ords_entry.base.docs_with_field_offset == -1 {
                                BaseSortedSetDocValuesEnum::Dense(DenseBaseSortedSetDocValues::new(
                                    self.max_doc,
                                    values,
                                    addresses,
                                ))
                            } else {
                                //sparse
                                let disi = IndexedDISI::new(
                                    &mut self.data,
                                    ords_entry.base.docs_with_field_offset,
                                    ords_entry.base.docs_with_field_length,
                                    ords_entry.base.jump_table_entry_count as i32,
                                    ords_entry.base.dense_rank_power,
                                    ords_entry.num_docs_with_field as i64,
                                )?;
                                BaseSortedSetDocValuesEnum::Sparse(
                                    SparseBaseSortedSetDocValues::new(disi, values, addresses),
                                )
                            };
                            return Ok(Lucene90SortedSetDocValuesEnum::Base(
                                BaseSortedSetDocValues::new(
                                    entry_clone.clone(),
                                    &mut self.data,
                                    sbu,
                                    self.merging,
                                )?,
                            ));
                        }

                        let ords = self.get_sorted_numeric(&ords_entry_clone)?;
                        let sub =
                            BaseSortedSetDocValuesEnum::Impl(BaseSortedSetDocValuesImpl::new(ords));
                        Ok(Lucene90SortedSetDocValuesEnum::Base(
                            BaseSortedSetDocValues::new(
                                entry_clone,
                                &mut self.data,
                                sub,
                                self.merging,
                            )?,
                        ))
                    },
                    None => Err(LuceneError::illegal_state("ords_entry is None".to_string()))?,
                }
            },
            None => Err(LuceneError::illegal_state(format!(
                "Missing sorted set entry for field {}",
                field_number
            ))),
        }
    }

    type DocValuesSkipper = DocValuesSkipperImpl<I>;

    fn get_skipper(&mut self, field: &Rc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        let entry = self.skippers.get(&field.number);
        match entry {
            Some(entry) => {
                let mut input = self
                    .data
                    .slice("doc value skipper", entry.offset, entry.length)?;
                if input.length() > 0 {
                    input.prefetch(0, 1)?;
                }
                // TODO: should we write to disk the actual max level for this
                // segment?
                Ok(DocValuesSkipperImpl::new(input, entry.clone()))
            },
            None => Err(LuceneError::illegal_state(format!(
                "Missing skipper entry for field {}",
                field.number
            ))),
        }
    }

    fn check_integrity(&self) -> Result<()> {
        CodecUtil::checksum_entire_file(&self.data)?;
        Ok(())
    }

    // fn get_merge_instance(&mut self) ->
    // Result<Option<DocValuesProducerEnum<I>>> {
    //     Ok(Some(DocValuesProducerEnum::Lucene90(
    //         Lucene90DocValuesProducer::new_with_merging(
    //             self.numerics.clone(),
    //             self.binaries.clone(),
    //             self.sorted.clone(),
    //             self.sorted_sets.clone(),
    //             self.sorted_numerics.clone(),
    //             self.skippers.clone(),
    //             &self.data,
    //             self.max_doc,
    //             self.version,
    //         )?,
    //     )))
    // }
}
#[derive(Debug, Clone, Copy)]
pub struct DocValuesSkipperEntry {
    pub offset: i64,
    pub length: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub doc_count: i32,
    pub max_doc_id: i32,
}
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct NumericEntry {
    pub table: Option<Rc<Vec<i64>>>,
    pub block_shift: i32,
    pub bits_per_value: u8,
    pub docs_with_field_offset: i64,
    pub docs_with_field_length: i64,
    pub jump_table_entry_count: i16,
    pub dense_rank_power: i8,
    pub num_values: i64,
    pub min_value: i64,
    pub gcd: i64,
    pub values_offset: i64,
    pub values_length: i64,
    pub value_jump_table_offset: i64, // -1 if no jump-table
}

pub struct BinaryEntry {
    pub data_offset: i64,
    pub data_length: i64,
    pub docs_with_field_offset: i64,
    pub docs_with_field_length: i64,
    pub jump_table_entry_count: i16,
    pub dense_rank_power: u8,
    pub num_docs_with_field: i32,
    pub min_length: i32,
    pub max_length: i32,
    pub addresses_offset: i64,
    pub addresses_length: i64,
    pub addresses_meta: Option<Meta>,
}

#[derive(Default)]
pub struct TermsDictEntry {
    pub terms_dict_size: i64,
    pub terms_addresses_meta: Option<Meta>,
    pub max_term_length: i32,
    pub terms_data_offset: i64,
    pub terms_data_length: i64,
    pub terms_addresses_offset: i64,
    pub terms_addresses_length: i64,
    pub terms_dict_index_shift: i32,
    pub terms_index_addresses_meta: Option<Meta>,
    pub terms_index_offset: i64,
    pub terms_index_length: i64,
    pub terms_index_addresses_offset: i64,
    pub terms_index_addresses_length: i64,
    pub max_block_length: i32,
}

pub struct SortedEntry {
    pub ords_entry: Rc<NumericEntry>,
    pub terms_dict_entry: Rc<TermsDictEntry>,
}

#[derive(Default)]
pub struct SortedSetEntry {
    pub single_value_entry: Option<Rc<SortedEntry>>,
    pub ords_entry: Option<Rc<SortedNumericEntry>>,
    pub terms_dict_entry: Option<Rc<TermsDictEntry>>,
}
#[derive(Default)]
pub struct SortedNumericEntry {
    pub base: Rc<NumericEntry>,
    pub num_docs_with_field: i32,
    pub addresses_meta: Option<Meta>,
    pub addresses_offset: i64,
    pub addresses_length: i64,
}
pub struct DenseNumericDocValues<I>
where
    I: IndexInput,
{
    sub: DenseNumericDocValuesSubEnum<I>,
    max_doc: i32,
    doc: i32,
}
impl<I> DenseNumericDocValues<I>
where
    I: IndexInput,
{
    fn new(base: DenseNumericDocValuesSubEnum<I>, max_doc: i32) -> Self {
        Self {
            sub: base,
            max_doc,
            doc: -1,
        }
    }
}

impl<I> DocValuesIterator for DenseNumericDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(true)
    }
}

impl<I> DocIdSetIterator for DenseNumericDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target >= self.max_doc {
            self.doc = NO_MORE_DOCS;
        } else {
            self.doc = target;
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

impl<I> NumericDocValues for DenseNumericDocValues<I>
where
    I: IndexInput,
{
    fn long_value(&mut self) -> Result<i64> {
        self.sub.long_value(self.doc)
    }
}

pub struct SparseNumericDocValues<I>
where
    I: IndexInput,
{
    sub: SparseNumericDocValuesSubEnum<I>,
    disi: IndexedDISI<I>,
}
impl<I> SparseNumericDocValues<I>
where
    I: IndexInput,
{
    fn new(sub: SparseNumericDocValuesSubEnum<I>, disi: IndexedDISI<I>) -> Self {
        Self { sub, disi }
    }
}

impl<I> DocValuesIterator for SparseNumericDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.disi.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for SparseNumericDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.disi.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.disi.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

impl<I> NumericDocValues for SparseNumericDocValues<I>
where
    I: IndexInput,
{
    fn long_value(&mut self) -> Result<i64> {
        self.sub.long_value(&mut self.disi)
    }
}

pub struct DenseBinaryDocValues<I>
where
    I: IndexInput,
{
    sub: DenseBinaryDocValuesBaseEnum<I>,
    max_doc: i32,
    doc: i32,
}
impl<I> DenseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn new(sub: DenseBinaryDocValuesBaseEnum<I>, max_doc: i32) -> Self {
        Self {
            sub,
            max_doc,
            doc: -1,
        }
    }
}

impl<I> DocValuesIterator for DenseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(true)
    }
}

impl<I> DocIdSetIterator for DenseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if self.doc >= self.max_doc {
            self.doc = NO_MORE_DOCS;
        } else {
            self.doc = target;
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

impl<I> BinaryDocValues for DenseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        self.sub.binary_value(self.doc)
    }
}

pub struct SparseBinaryDocValues<I>
where
    I: IndexInput,
{
    sub: SparseBinaryDocValuesBaseEnum<I>,
    disi: IndexedDISI<I>,
}
impl<I> SparseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn new(sub: SparseBinaryDocValuesBaseEnum<I>, disi: IndexedDISI<I>) -> Self {
        Self { sub, disi }
    }
}

impl<I> DocValuesIterator for SparseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.disi.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for SparseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.disi.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.disi.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

impl<I> BinaryDocValues for SparseBinaryDocValues<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        self.sub.binary_value(&mut self.disi)
    }
}

/// Reader for longs split into blocks of different bits per value.
/// The longs are requested by index and must be accessed in monotonically
/// increasing order.
///
/// Note: The order requirement could be removed as the jump tables allow for
/// backwards iteration.
///
/// Note 2: The `rank_slice` is only used if an advance of more than one block
/// is called. Its construction could be lazy.
struct VaryingBPVReader<I>
where
    I: IndexInput,
{
    // 2 slices to avoid cache thrashing when using rank
    slice: Rc<RefCell<I::RandomAccessSlice>>,
    rank_slice: Option<I::RandomAccessSlice>,
    entry: Rc<NumericEntry>,

    shift: i32,
    mul: i64,
    mask: i64,

    block: i64,
    delta: i64,
    offset: i64,
    block_end_offset: i64,

    values: Option<DirectPackedEnum<I::RandomAccessSlice>>,
    merging: bool,
}

impl<I> VaryingBPVReader<I>
where
    I: IndexInput,
{
    fn new(
        entry: Rc<NumericEntry>,
        slice: I::RandomAccessSlice,
        data: &mut I,
        merging: bool,
    ) -> Result<Self> {
        let rank_slice = if entry.value_jump_table_offset == -1 {
            None
        } else {
            let mut slice = data.random_access_slice(
                entry.value_jump_table_offset,
                data.length() - entry.value_jump_table_offset,
            )?;
            if slice.length() > 0 {
                slice.prefetch(0, 1)?;
            }
            Some(slice)
        };

        let shift = entry.block_shift;
        let mul = entry.gcd;
        let mask = (1 << shift) - 1;

        Ok(Self {
            slice: Rc::new(RefCell::new(slice)),
            rank_slice,
            entry,
            shift,
            mul,
            mask: mask as i64,
            block: -1,
            delta: 0,
            offset: 0,
            block_end_offset: 0,
            values: None,
            merging,
        })
    }

    fn get_long_value(&mut self, index: i64) -> Result<i64> {
        let block = ((index as u64) >> self.shift) as i64;

        if self.block != block {
            loop {
                let bits_per_value;
                if let Some(ref mut rank_slice) = self.rank_slice {
                    if block != self.block + 1 {
                        self.block_end_offset = rank_slice
                            .read_long(block * BitUtil::LONG_BYTES as i64)?
                            - self.entry.values_offset;
                        self.block = block - 1;
                    }
                }

                {
                    let mut slice = self.slice.borrow_mut();
                    self.offset = self.block_end_offset;
                    bits_per_value = slice.read_byte(self.offset)? as i32;
                    self.offset += 1;

                    self.delta = slice.read_long(self.offset)?;
                    self.offset += BitUtil::LONG_BYTES as i64;

                    if bits_per_value == 0 {
                        self.block_end_offset = self.offset;
                    } else {
                        let length = slice.read_int(self.offset)? as i64;
                        self.offset += BitUtil::INT_BYTES as i64;
                        self.block_end_offset = self.offset + length;
                    }
                }

                self.block += 1;

                if self.block == block {
                    let num_values: i32 = std::cmp::min(
                        1 << self.shift,
                        self.entry.num_values - (block << self.shift),
                    )
                    .try_into()?;

                    self.values = if bits_per_value == 0 {
                        Some(DirectPackedEnum::Zeroes(Zeroes))
                    } else {
                        Some(lucene90_dvp_util::get_direct_reader_instance::<I>(
                            self.merging,
                            Rc::clone(&self.slice),
                            bits_per_value,
                            self.offset,
                            num_values as i64,
                        )?)
                    };

                    break;
                }
            }
        }
        match self.values {
            Some(ref mut values) => Ok(self.mul * values.get(index & self.mask)? + self.delta),
            None => Err(LuceneError::illegal_state(
                "values should not be None".to_string(),
            )),
        }
    }
}

pub struct DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    min_doc_id: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    max_doc_id: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    min_value: [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    max_value: [i64; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    doc_count: [i32; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
    levels: i32,
    input: I::Slice,
    entry: Rc<DocValuesSkipperEntry>,
}
impl<I> DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    pub fn new(input: I::Slice, entry: Rc<DocValuesSkipperEntry>) -> Self {
        Self {
            min_doc_id: [-1; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            max_doc_id: [-1; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            min_value: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            max_value: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            doc_count: [0; Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL],
            levels: 1,
            input,
            entry,
        }
    }
}
impl<I> DocValuesSkipper for DocValuesSkipperImpl<I>
where
    I: IndexInput,
{
    fn advance(&mut self, target: i32) -> Result<()> {
        if target > self.entry.max_doc_id {
            // skipper is exhausted
            for i in 0..Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL {
                self.min_doc_id[i] = NO_MORE_DOCS;
                self.max_doc_id[i] = NO_MORE_DOCS;
            }
        } else {
            // find next interval
            debug_assert!(
                target > self.max_doc_id[0],
                "target must be bigger than current interval"
            );

            loop {
                self.levels = self.input.read_byte()? as i32;

                debug_assert!(
                    self.levels <= Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL as i32
                        && self.levels > 0,
                    "level out of range [{}]",
                    self.levels
                );

                let mut valid = true;

                // check if current interval is competitive or we can jump to
                // the next position
                for level in (0..self.levels as usize).rev() {
                    let max_doc = self.input.read_int()?;
                    self.max_doc_id[level] = max_doc;
                    if max_doc < target {
                        IndexInput::skip_bytes(
                            &mut self.input,
                            SKIP_INDEX_JUMP_LENGTH_PER_LEVEL[level],
                        )?;
                        valid = false;
                        break;
                    }
                    self.min_doc_id[level] = self.input.read_int()?;
                    self.max_value[level] = self.input.read_long()?;
                    self.min_value[level] = self.input.read_long()?;
                    self.doc_count[level] = self.input.read_int()?;
                }

                if valid {
                    // adjust levels
                    while (self.levels as usize) < Lucene90DocValuesFormat::SKIP_INDEX_MAX_LEVEL
                        && self.max_doc_id[self.levels as usize] >= target
                    {
                        self.levels += 1;
                    }
                    break;
                }
            }
        }
        Ok(())
    }

    fn num_levels(&self) -> i32 {
        self.levels
    }

    fn min_doc_id(&self, level: i32) -> i32 {
        self.min_doc_id[level as usize]
    }

    fn max_doc_id(&self, level: i32) -> i32 {
        self.max_doc_id[level as usize]
    }

    fn min_value(&self, level: i32) -> i64 {
        self.min_value[level as usize]
    }

    fn max_value(&self, level: i32) -> i64 {
        self.max_value[level as usize]
    }

    fn doc_count_level(&self, level: i32) -> i32 {
        self.doc_count[level as usize]
    }

    fn global_min_value(&self) -> i64 {
        self.entry.min_value
    }

    fn global_max_value(&self) -> i64 {
        self.entry.max_value
    }

    fn global_doc_count(&self) -> i32 {
        self.entry.doc_count
    }
}

pub trait DenseNumericDocValuesBase {
    fn long_value(&mut self, doc: i32) -> Result<i64>;
}
pub struct DenseNumericDocValuesBaseImpl {
    min_values: i64,
}
impl DenseNumericDocValuesBase for DenseNumericDocValuesBaseImpl {
    fn long_value(&mut self, _doc: i32) -> Result<i64> {
        Ok(self.min_values)
    }
}
pub struct DenseNumericDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    vbpv_reader: VaryingBPVReader<I>,
}
impl<I> DenseNumericDocValuesBase for DenseNumericDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, doc: i32) -> Result<i64> {
        self.vbpv_reader.get_long_value(doc as i64)
    }
}
pub struct DenseNumericDocValuesBaseImpl2<I>
where
    I: IndexInput,
{
    table: Rc<Vec<i64>>,
    values: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> DenseNumericDocValuesBase for DenseNumericDocValuesBaseImpl2<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, doc: i32) -> Result<i64> {
        Ok(self.table[self.values.get(doc as i64)? as usize])
    }
}
pub struct DenseNumericDocValuesBaseImpl3<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> DenseNumericDocValuesBase for DenseNumericDocValuesBaseImpl3<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, doc: i32) -> Result<i64> {
        self.values.get(doc as i64)
    }
}
pub struct DenseNumericDocValuesBaseImpl4<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
    mul: i64,
    delta: i64,
}
impl<I> DenseNumericDocValuesBase for DenseNumericDocValuesBaseImpl4<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, doc: i32) -> Result<i64> {
        Ok(self.mul * self.values.get(doc as i64)? + self.delta)
    }
}

pub trait SparseNumericDocValuesBase<I> {
    fn long_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput;
}
pub struct SparseNumericDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    min_values: i64,
    _phantom: PhantomData<I>,
}
impl<I> SparseNumericDocValuesBase<I> for SparseNumericDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, _disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput,
    {
        Ok(self.min_values)
    }
}
pub struct SparseNumericDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    vbpv_reader: VaryingBPVReader<I>,
}
impl<I> SparseNumericDocValuesBase<I> for SparseNumericDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput,
    {
        let index = disi.index();
        self.vbpv_reader.get_long_value(index as i64)
    }
}
pub struct SparseNumericDocValuesBaseImpl2<I>
where
    I: IndexInput,
{
    table: Rc<Vec<i64>>,
    values: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> SparseNumericDocValuesBase<I> for SparseNumericDocValuesBaseImpl2<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput,
    {
        Ok(self.table[self.values.get(disi.index() as i64)? as usize])
    }
}
pub struct SparseNumericDocValuesBaseImpl3<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> SparseNumericDocValuesBase<I> for SparseNumericDocValuesBaseImpl3<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput,
    {
        self.values.get(disi.index() as i64)
    }
}
pub struct SparseNumericDocValuesBaseImpl4<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
    mul: i64,
    delta: i64,
}
impl<I> SparseNumericDocValuesBase<I> for SparseNumericDocValuesBaseImpl4<I>
where
    I: IndexInput,
{
    fn long_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<i64>
    where
        I: IndexInput,
    {
        Ok(self.mul * self.values.get(disi.index() as i64)? + self.delta)
    }
}

pub struct LongValuesImpl {
    min_values: i64,
}
impl LongValues for LongValuesImpl {
    fn get(&mut self, _index: i64) -> Result<i64> {
        Ok(self.min_values)
    }
}
pub struct LongValuesImpl1<I>
where
    I: IndexInput,
{
    vbpv_reader: VaryingBPVReader<I>,
}
impl<I> LongValues for LongValuesImpl1<I>
where
    I: IndexInput,
{
    fn get(&mut self, index: i64) -> Result<i64> {
        self.vbpv_reader.get_long_value(index)
    }
}
pub struct LongValuesImpl2<I>
where
    I: IndexInput,
{
    table: Rc<Vec<i64>>,
    values: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> LongValues for LongValuesImpl2<I>
where
    I: IndexInput,
{
    fn get(&mut self, index: i64) -> Result<i64> {
        Ok(self.table[self.values.get(index)? as usize])
    }
}
pub struct LongValuesImpl3<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
    gcd: i64,
    min_value: i64,
}
impl<I> LongValues for LongValuesImpl3<I>
where
    I: IndexInput,
{
    fn get(&mut self, index: i64) -> Result<i64> {
        Ok(self.gcd * self.values.get(index)? + self.min_value)
    }
}
pub struct LongValuesImpl4<I>
where
    I: IndexInput,
{
    values: DirectPackedEnum<I::RandomAccessSlice>,
    min_value: i64,
}
impl<I> LongValues for LongValuesImpl4<I>
where
    I: IndexInput,
{
    fn get(&mut self, index: i64) -> Result<i64> {
        Ok(self.values.get(index)? + self.min_value)
    }
}

pub trait DenseBinaryDocValuesBase {
    fn binary_value(&mut self, doc: i32) -> Result<&BytesRef<Vec<u8>>>;
}

pub struct DenseBinaryDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    bytes_slice: I::RandomAccessSlice,
    length: i32,
    bytes: BytesRef<Vec<u8>>,
}
impl<I> DenseBinaryDocValuesBase for DenseBinaryDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self, doc: i32) -> Result<&BytesRef<Vec<u8>>> {
        self.bytes_slice.read_bytes(
            (doc * self.length) as i64,
            &mut self.bytes.bytes,
            0,
            self.length,
        )?;
        Ok(&self.bytes)
    }
}
pub struct DenseBinaryDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    bytes_slice: I::RandomAccessSlice,
    bytes: BytesRef<Vec<u8>>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
}
impl<I> DenseBinaryDocValuesBase for DenseBinaryDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self, doc: i32) -> Result<&BytesRef<Vec<u8>>> {
        let start_offset = self.addresses.get(doc as i64)?;
        self.bytes.length = (self.addresses.get((doc + 1) as i64)? - start_offset) as usize;
        self.bytes_slice.read_bytes(
            start_offset,
            &mut self.bytes.bytes,
            0,
            self.bytes.length as i32,
        )?;
        Ok(&self.bytes)
    }
}

pub trait SparseBinaryDocValuesBase<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<&BytesRef<Vec<u8>>>;
}
pub struct SparseBinaryDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    bytes_slice: I::RandomAccessSlice,
    bytes: BytesRef<Vec<u8>>,
    length: i32,
}
impl<I> SparseBinaryDocValuesBase<I> for SparseBinaryDocValuesBaseImpl<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<&BytesRef<Vec<u8>>> {
        let pos = (disi.index() * self.length) as i64;
        self.bytes_slice
            .read_bytes(pos, &mut self.bytes.bytes, 0, self.length)?;
        Ok(&self.bytes)
    }
}
pub struct SparseBinaryDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    bytes_slice: I::RandomAccessSlice,
    bytes: BytesRef<Vec<u8>>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
}
impl<I> SparseBinaryDocValuesBase<I> for SparseBinaryDocValuesBaseImpl1<I>
where
    I: IndexInput,
{
    fn binary_value(&mut self, disi: &mut IndexedDISI<I>) -> Result<&BytesRef<Vec<u8>>> {
        let index = disi.index() as i64;
        let start_offset = self.addresses.get(index)?;
        self.bytes.length = (self.addresses.get(index + 1)? - start_offset) as usize;
        self.bytes_slice.read_bytes(
            start_offset,
            &mut self.bytes.bytes,
            0,
            self.bytes.length as i32,
        )?;
        Ok(&self.bytes)
    }
}

pub struct DenseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    doc: i32,
    max_doc: i32,
    value: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> DenseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn new(max_doc: i32, value: DirectPackedEnum<I::RandomAccessSlice>) -> Self {
        Self {
            doc: -1,
            max_doc,
            value,
        }
    }
}

impl<I> DocValuesIterator for DenseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.doc = target;
        Ok(true)
    }
}

impl<I> DocIdSetIterator for DenseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target >= self.max_doc {
            self.doc = NO_MORE_DOCS;
        } else {
            self.doc = target;
        }
        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

impl<I> SortedDocValues for DenseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.value.get(self.doc as i64)? as i32)
    }

    type AV = Vec<u8>;

    type TermsEnum = DummyTermsEnum;
}

pub struct SparseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    disi: IndexedDISI<I>,
    value: DirectPackedEnum<I::RandomAccessSlice>,
}
impl<I> SparseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn new(disi: IndexedDISI<I>, value: DirectPackedEnum<I::RandomAccessSlice>) -> Self {
        Self { disi, value }
    }
}

impl<I> DocValuesIterator for SparseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.disi.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for SparseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.disi.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.disi.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

impl<I> SortedDocValues for SparseBaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.value.get(self.disi.index() as i64)? as i32)
    }

    type AV = Vec<u8>;

    type TermsEnum = DummyTermsEnum;
}
pub struct BaseSortedDocValuesImpl<I>
where
    I: IndexInput,
{
    ords: Lucene90NumericDocValuesEnums<I>,
}
impl<I> BaseSortedDocValuesImpl<I>
where
    I: IndexInput,
{
    fn new(ords: Lucene90NumericDocValuesEnums<I>) -> Self {
        Self { ords }
    }
}

impl<I> DocValuesIterator for BaseSortedDocValuesImpl<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.ords.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for BaseSortedDocValuesImpl<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.ords.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.ords.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.ords.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.ords.cost()
    }
}

impl<I> SortedDocValues for BaseSortedDocValuesImpl<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        Ok(self.ords.long_value()? as i32)
    }

    type AV = Vec<u8>;

    type TermsEnum = DummyTermsEnum;
}

pub struct BaseSortedDocValues<I>
where
    I: IndexInput,
{
    entry: Rc<SortedEntry>,
    // if copy is heavy, we could change `Vec<u8>` as `Rc<RefCell<Vec<u8>>>`
    terms_enum: TermsDict<I, Vec<u8>>,
    sub: BaseSortedDocValuesEnum<I>,
}

impl<I> BaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn new(
        entry: Rc<SortedEntry>,
        data: &mut I,
        sub: BaseSortedDocValuesEnum<I>,
        merging: bool,
    ) -> Result<Self> {
        let terms_enum = TermsDict::new(entry.terms_dict_entry.clone(), data, merging)?;
        Ok(Self {
            entry,
            terms_enum,
            sub,
        })
    }
}

impl<I> DocValuesIterator for BaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.sub.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for BaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.sub.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.sub.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.sub.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.sub.cost()
    }
}

impl<I> SortedDocValues for BaseSortedDocValues<I>
where
    I: IndexInput,
{
    fn ord_value(&mut self) -> Result<i32> {
        self.sub.ord_value()
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i32) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.terms_enum.seek_exact_with_ord(ord as i64)?;
        self.terms_enum.term()
    }

    fn get_value_count(&mut self) -> Result<i32> {
        let v: i32 = self.entry.terms_dict_entry.terms_dict_size.try_into()?;
        Ok(v)
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i32> {
        match self.terms_enum.seek_ceil(key)? {
            SeekStatus::Found => {
                let v = self.terms_enum.ord()?.try_into()?;
                Ok(v)
            },
            SeekStatus::NotFound | SeekStatus::End => {
                let v = (-1 - self.terms_enum.ord()?).try_into()?;
                Ok(v)
            },
        }
    }
    type TermsEnum = DummyTermsEnum;
}
pub struct DenseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    max_doc: i32,
    doc: i32,
    curr: i64,
    count: i32,
    value: DirectPackedEnum<I::RandomAccessSlice>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
}
impl<I> DenseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn new(
        max_doc: i32,
        value: DirectPackedEnum<I::RandomAccessSlice>,
        addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    ) -> Self {
        Self {
            max_doc,
            doc: -1,
            curr: 0,
            count: 0,
            value,
            addresses,
        }
    }
}

impl<I> DocValuesIterator for DenseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.curr = self.addresses.get(target as i64)?;
        let end = self.addresses.get((target as i64) + 1)?;
        self.count = (end - self.curr) as i32;
        self.doc = target;
        Ok(true)
    }
}

impl<I> DocIdSetIterator for DenseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if target >= self.max_doc {
            self.doc = NO_MORE_DOCS;
            return Ok(NO_MORE_DOCS);
        }

        self.curr = self.addresses.get(target as i64)?;
        let end = self.addresses.get((target as i64) + 1)?;
        self.count = (end - self.curr) as i32;
        self.doc = target;

        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

impl<I> SortedSetDocValues for DenseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        let ord = self.value.get(self.curr)?;
        self.count += 1;
        Ok(ord)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(self.count)
    }

    type AV = Vec<u8>;

    type TermsEnum = TermsDict<I, Vec<u8>>;
}

pub struct SparseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    disi: IndexedDISI<I>,
    set: bool,
    curr: i64,
    count: i32,
    value: DirectPackedEnum<I::RandomAccessSlice>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
}
impl<I> SparseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn new(
        disi: IndexedDISI<I>,
        value: DirectPackedEnum<I::RandomAccessSlice>,
        addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    ) -> Self {
        Self {
            disi,
            set: false,
            curr: 0,
            count: 0,
            value,
            addresses,
        }
    }
    fn set(&mut self) -> Result<()> {
        if !self.set {
            let index = self.disi.index();
            self.curr = self.addresses.get(index as i64)?;
            let end = self.addresses.get((index as i64) + 1)?;
            self.count = (end - self.curr) as i32;
            self.set = true;
        }
        Ok(())
    }
}

impl<I> DocValuesIterator for SparseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.set = false;
        self.disi.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for SparseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.set = false;
        self.disi.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.set = false;
        self.disi.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

impl<I> SortedSetDocValues for SparseBaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        self.set()?;
        let ord = self.value.get(self.curr)?;
        self.curr += 1;
        Ok(ord)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        self.set()?;
        Ok(self.count)
    }

    type AV = Vec<u8>;

    type TermsEnum = DummyTermsEnum;
}

pub struct BaseSortedSetDocValuesImpl<I>
where
    I: IndexInput,
{
    ords: Lucene90SortedNumericDocValuesEnums<I>,
}
impl<I> BaseSortedSetDocValuesImpl<I>
where
    I: IndexInput,
{
    fn new(ords: Lucene90SortedNumericDocValuesEnums<I>) -> Self {
        Self { ords }
    }
}

impl<I> DocValuesIterator for BaseSortedSetDocValuesImpl<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.ords.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for BaseSortedSetDocValuesImpl<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.ords.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.ords.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.ords.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.ords.cost()
    }
}

impl<I> SortedSetDocValues for BaseSortedSetDocValuesImpl<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        self.ords.next_value()
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        self.ords.doc_value_count()
    }

    type AV = Vec<u8>;

    type TermsEnum = TermsDict<I, Vec<u8>>;
}

pub struct BaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    entry: Rc<SortedSetEntry>,
    // if copy is heavy, we could change `Vec<u8>` as `Rc<RefCell<Vec<u8>>>`
    terms_enum: TermsDict<I, Vec<u8>>,
    sub: BaseSortedSetDocValuesEnum<I>,
}

impl<I> BaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn new(
        entry: Rc<SortedSetEntry>,
        data: &mut I,
        sub: BaseSortedSetDocValuesEnum<I>,
        merging: bool,
    ) -> Result<Self> {
        let terms_dict_entry = match entry.terms_dict_entry {
            Some(ref entry) => entry.clone(),
            None => {
                return Err(LuceneError::illegal_state(
                    "TermsDictEntry's terms_dict_entry is None".to_string(),
                ));
            },
        };
        let terms_enum = TermsDict::new(terms_dict_entry, data, merging)?;
        Ok(Self {
            entry,
            terms_enum,
            sub,
        })
    }
}

impl<I> DocValuesIterator for BaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.sub.advance_exact(target)
    }
}

impl<I> DocIdSetIterator for BaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.sub.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.sub.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.sub.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.sub.cost()
    }
}

impl<I> SortedSetDocValues for BaseSortedSetDocValues<I>
where
    I: IndexInput,
{
    fn next_ord(&mut self) -> Result<i64> {
        self.sub.next_ord()
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        self.sub.doc_value_count()
    }

    type AV = Vec<u8>;

    fn lookup_ord(&mut self, ord: i64) -> Result<Cow<BytesRef<Vec<u8>>>> {
        self.terms_enum.seek_exact_with_ord(ord)?;
        self.terms_enum.term()
    }

    fn get_value_count(&mut self) -> Result<i64> {
        match self.entry.terms_dict_entry {
            Some(ref entry) => Ok(entry.terms_dict_size),
            None => Err(LuceneError::illegal_state(
                "TermsDictEntry's terms_dict_entry is None".to_string(),
            )),
        }
    }

    fn lookup_term(&mut self, key: &BytesRef<Vec<u8>>) -> Result<i64> {
        match self.terms_enum.seek_ceil(key)? {
            SeekStatus::Found => Ok(self.terms_enum.ord()?),
            SeekStatus::NotFound | SeekStatus::End => {
                let ord = self.terms_enum.ord()?;
                Ok(-1 - ord)
            },
        }
    }

    type TermsEnum = TermsDict<I, Vec<u8>>;
}

pub struct TermsDict<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    pub entry: Rc<TermsDictEntry>,
    pub block_addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    pub bytes: I::Slice,
    pub block_mask: u64,
    pub index_addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    pub index_bytes: I::RandomAccessSlice,
    pub term: BytesRef<AV>,
    pub ord: i64,
    pub block_buffer: BytesRef<AV>,
    pub block_input: ByteArrayDataInput<AV>,
    pub current_compressed_block_start: i64,
    pub current_compressed_block_end: i64,
}

impl<I, AV> TermsDict<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    const LZ4_DECOMPRESSOR_PADDING: i32 = 7;

    pub fn new(entry: Rc<TermsDictEntry>, data: &mut I, merging: bool) -> Result<Self> {
        let addresses_slice = Rc::new(RefCell::new(
            data.random_access_slice(entry.terms_addresses_offset, entry.terms_addresses_length)?,
        ));
        let block_addresses = match entry.terms_addresses_meta {
            Some(ref meta) => DirectMonotonicReader::get_instance_with_merging(
                meta,
                addresses_slice.clone(),
                merging,
            )?,
            None => {
                return Err(LuceneError::illegal_state(
                    "TermsDictEntry's terms_addresses_meta is None".to_string(),
                ));
            },
        };

        let bytes = data.slice("terms", entry.terms_data_offset, entry.terms_data_length)?;

        let block_mask = (1u64 << Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT) - 1;

        let index_addresses_slice = Rc::new(RefCell::new(data.random_access_slice(
            entry.terms_index_addresses_offset,
            entry.terms_index_addresses_length,
        )?));

        let index_addresses = match entry.terms_index_addresses_meta {
            Some(ref meta) => DirectMonotonicReader::get_instance_with_merging(
                meta,
                index_addresses_slice.clone(),
                merging,
            )?,
            None => {
                return Err(LuceneError::illegal_state(
                    "TermsDictEntry's terms_index_addresses_meta is None".to_string(),
                ));
            },
        };

        let index_bytes =
            data.random_access_slice(entry.terms_index_offset, entry.terms_index_length)?;

        let term = BytesRef::with_capacity(entry.max_term_length as usize);
        // add the max term length for the dictionary
        // add 7 padding bytes can help decompression run faster.
        let buffer_size =
            entry.max_block_length + entry.max_term_length + Self::LZ4_DECOMPRESSOR_PADDING;

        let block_buffer = BytesRef::from_slice(
            AV::from_vec(vec![0u8; buffer_size as usize]),
            0,
            buffer_size as usize,
        );
        let block_input = ByteArrayDataInput::new(); // assuming default constructor

        Ok(Self {
            entry,
            block_addresses,
            bytes,
            block_mask,
            index_addresses,
            index_bytes,
            term,
            ord: -1,
            block_buffer,
            block_input,
            current_compressed_block_start: -1,
            current_compressed_block_end: -1,
        })
    }

    fn get_term_from_index(&mut self, index: i64) -> Result<&BytesRef<AV>> {
        debug_assert!(
            index >= 0
                && index
                    <= ((self.entry.terms_dict_size - 1) as u64
                        >> self.entry.terms_dict_index_shift) as i64,
            "index {} out of range",
            index
        );

        let start = self.index_addresses.get(index)?;
        let end = self.index_addresses.get(index + 1)?;
        let len = (end - start) as i32;
        self.term.length = len as usize;

        self.term.bytes.access_mut(|bytes| {
            self.index_bytes.read_bytes(start, bytes, 0, len)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;

        Ok(&self.term)
    }
    fn seek_terms_index(&mut self, text: &BytesRef<AV>) -> Result<i64> {
        let mut lo: i64 = 0;
        let mut hi: i64 = (self.entry.terms_dict_size - 1) >> self.entry.terms_dict_index_shift;

        while lo <= hi {
            let mid = (lo + hi) >> 1;
            let term = self.get_term_from_index(mid)?;
            let cmp = term.cmp(text).to_int();
            if cmp <= 0 {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        debug_assert!(
            hi < 0 || self.get_term_from_index(hi)?.cmp(text).to_int() <= 0,
            "hi check failed"
        );
        debug_assert!(
            hi == ((self.entry.terms_dict_size - 1) >> self.entry.terms_dict_index_shift)
                || self.get_term_from_index(hi + 1)?.cmp(text).to_int() > 0,
            "hi+1 check failed"
        );
        // return -1 iff empty term dict
        debug_assert!(
            (hi < 0) ^ (self.entry.terms_dict_size > 0),
            "empty term dict assertion failed"
        );
        Ok(hi)
    }
    fn get_first_term_from_block(&mut self, block: i64) -> Result<&BytesRef<AV>> {
        debug_assert!(
            block >= 0
                && block
                    <= (((self.entry.terms_dict_size - 1) as u64)
                        >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT)
                        as i64
        );

        let block_address = self.block_addresses.get(block)?;
        self.bytes.seek(block_address)?;

        let len = self.bytes.read_vint()?;
        self.term.length = len as usize;
        self.term.bytes.access_mut(|bytes| {
            self.bytes.read_bytes(bytes, 0, len)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;

        Ok(&self.term)
    }
    fn seek_block(&mut self, text: &BytesRef<AV>) -> Result<i64> {
        let index = self.seek_terms_index(text)?;

        if index == -1 {
            // Empty term dictionary
            self.ord = 0;
            return Ok(-2);
        }

        let ord_lo = index << self.entry.terms_dict_index_shift;
        let ord_hi = std::cmp::min(
            self.entry.terms_dict_size,
            ord_lo + (1 << self.entry.terms_dict_index_shift),
        ) - 1;

        let mut block_lo =
            (ord_lo as u64 >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT) as i64;
        let mut block_hi =
            (ord_hi as u64 >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT) as i64;

        while block_lo <= block_hi {
            let block_mid = ((block_lo + block_hi) as u64 >> 1) as i64;
            let term = self.get_first_term_from_block(block_mid)?;
            let cmp = term.cmp(text).to_int();
            if cmp <= 0 {
                block_lo = block_mid + 1;
            } else {
                block_hi = block_mid - 1;
            }
        }

        debug_assert!(
            block_hi < 0 || self.get_first_term_from_block(block_hi)?.cmp(text).to_int() <= 0
        );
        debug_assert!(
            block_hi
                == ((self.entry.terms_dict_size - 1) as u64
                    >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT)
                    as i64
                || self
                    .get_first_term_from_block(block_hi + 1)?
                    .cmp(text)
                    .to_int()
                    > 0
        );
        // read the block only if term dict is not empty
        debug_assert!(self.entry.terms_dict_size > 0);
        // reset ord and bytes to the ceiling block even if
        // text is before the first term (blockHi == -1)
        let block = std::cmp::max(block_hi, 0);
        let block_address = self.block_addresses.get(block)?;
        self.ord = block << Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT;
        self.bytes.seek(block_address)?;
        self.decompress_block()?;

        Ok(block_hi)
    }
    fn decompress_block(&mut self) -> Result<()> {
        // The first term is kept uncompressed, so no need to decompress block
        // if only look up the first term when doing seek block.
        self.term.length = self.bytes.read_vint()? as usize;
        self.term.bytes.access_mut(|bytes| {
            self.bytes.read_bytes(bytes, 0, self.term.length as i32)?;
            // Help the compiler infer types.
            Ok::<(), LuceneError>(())
        })?;
        let offset = self.bytes.get_file_pointer();
        if offset < self.entry.terms_data_length - 1 {
            // Avoid decompressing again if reading the same block
            if self.current_compressed_block_start != offset {
                let block_buffer_offset = self.term.length;
                let block_buffer_len = self.bytes.read_vint()?;

                self.block_buffer.offset = block_buffer_offset;
                self.block_buffer.length = block_buffer_len as usize;
                // Decompress the remaining of current block, using the first
                // term as a dictionary
                self.block_buffer.bytes.access_mut(|buffer_bytes| {
                    self.term.bytes.access(|term_bytes| {
                        buffer_bytes.copy_from(&term_bytes[..block_buffer_offset], 0);
                    })
                });
                self.block_buffer.bytes.access_mut(|buffer_bytes| {
                    LZ4::decompress(
                        &mut self.bytes,
                        block_buffer_len,
                        buffer_bytes,
                        block_buffer_offset as i32,
                    )?;
                    // Help the compiler infer types.
                    Ok::<(), LuceneError>(())
                })?;

                self.current_compressed_block_start = offset;
                self.current_compressed_block_end = self.bytes.get_file_pointer();
            } else {
                // Seek to block end if already decompressed
                self.bytes.seek(self.current_compressed_block_end)?;
            }

            // Reset buffer reader
            self.block_input = ByteArrayDataInput::with_range(
                std::mem::take(&mut self.block_buffer.bytes),
                self.block_buffer.offset,
                self.block_buffer.length,
            );
        }

        Ok(())
    }
}

impl<I, AV> BytesRefIterator for TermsDict<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    type AV = AV;

    fn next(&mut self) -> Result<Option<Cow<BytesRef<Self::AV>>>> {
        self.ord += 1;
        if self.ord >= self.entry.terms_dict_size {
            return Ok(None);
        }

        if (self.ord & self.block_mask as i64) == 0 {
            self.decompress_block()?;
        } else {
            let input = &mut self.block_input;
            let token = input.read_byte()? as i32;
            let mut prefix_length = token & 0x0F;
            let mut suffix_length = 1 + (token as usize >> 4) as i32;

            if prefix_length == 15 {
                prefix_length += input.read_vint()?;
            }
            if suffix_length == 16 {
                suffix_length += input.read_vint()?;
            }

            self.term.length = (prefix_length + suffix_length) as usize;
            self.term.bytes.access_mut(|bytes| {
                input.read_bytes(bytes, prefix_length, suffix_length)?;
                // Help the compiler infer types.
                Ok::<(), LuceneError>(())
            })?;
        }
        Ok(Some(Cow::Borrowed(&self.term)))
    }
}

impl<I, AV> TermsEnum for TermsDict<I, AV>
where
    I: IndexInput,
    AV: AccessVec<u8>,
{
    fn attributes(&self) -> Result<&AttributeSource> {
        Err(LuceneError::not_implemented(""))
    }

    fn seek_ceil(&mut self, text: &BytesRef<Self::AV>) -> Result<SeekStatus> {
        let block = self.seek_block(text)?;
        if block == -2 {
            // empty terms dict
            debug_assert!(self.entry.terms_dict_size == 0);
            return Ok(SeekStatus::End);
        } else if block == -1 {
            // before the first term
            return Ok(SeekStatus::NotFound);
        }

        loop {
            let cmp = self.term.cmp(text).to_int();
            if cmp == 0 {
                return Ok(SeekStatus::Found);
            } else if cmp > 0 {
                return Ok(SeekStatus::NotFound);
            }

            if self.next()?.is_none() {
                return Ok(SeekStatus::End);
            }
        }
    }

    fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
        if ord < 0 || ord >= self.entry.terms_dict_size {
            return Err(LuceneError::illegal_state(format!(
                "Invalid ordinal {} (max {})",
                ord, self.entry.terms_dict_size
            )));
        }

        // Signed shift since ord is -1 when unpositioned
        let current_block_index = self.ord >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT;
        let block_index = ord >> Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT;

        if ord < self.ord || block_index != current_block_index {
            // The looked up ord is before the current ord or belongs to a
            // different block, seek again
            let block_address = self.block_addresses.get(block_index)?;
            self.bytes.seek(block_address)?;
            self.ord = (block_index << Lucene90DocValuesFormat::TERMS_DICT_BLOCK_LZ4_SHIFT) - 1;
        }
        // Scan forward to the desired ordinal
        while self.ord < ord {
            self.next()?;
        }
        Ok(())
    }

    fn seek_exact_with_state(
        &mut self,
        _term: &BytesRef<Self::AV>,
        _state: &TermStateEnum,
    ) -> Result<()> {
        Err(LuceneError::not_implemented(""))
    }

    fn term(&self) -> Result<Cow<BytesRef<Self::AV>>> {
        Ok(Cow::Borrowed(&self.term))
    }

    fn ord(&self) -> Result<i64> {
        Ok(self.ord)
    }

    fn doc_freq(&mut self) -> Result<i32> {
        Err(LuceneError::unsupported_operation(""))
    }

    fn total_term_freq(&mut self) -> Result<i64> {
        Ok(-1)
    }

    type PostingsEnum = DummyPostingsEnum;

    fn postings_with_flags(
        &mut self,
        _reuse: Option<Self::PostingsEnum>,
        _flags: i32,
    ) -> Result<Self::PostingsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type ImpactsEnum = DummyImpactsEnum;

    fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
        Err(LuceneError::unsupported_operation(""))
    }

    type TermState = DummyTermState;

    fn term_state(&mut self) -> Result<Self::TermState> {
        Err(LuceneError::not_implemented(""))
    }
}

pub struct DenseSortedNumericDocValues<I>
where
    I: IndexInput,
{
    max_doc: i32,
    start: i64,
    end: i64,
    doc: i32,
    count: i32,
    values: LongValuesEnum<I>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
}
impl<I> DenseSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn new(
        max_doc: i32,
        start: i64,
        end: i64,
        values: LongValuesEnum<I>,
        addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    ) -> Self {
        Self {
            max_doc,
            start,
            end,
            doc: -1,
            count: 0,
            values,
            addresses,
        }
    }
}

impl<I> DocValuesIterator for DenseSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.start = self.addresses.get(target as i64)?;
        self.end = self.addresses.get((target as i64) + 1)?;
        self.count = (self.end - self.start) as i32;
        self.doc = target;
        Ok(true)
    }
}

impl<I> DocIdSetIterator for DenseSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.doc
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.advance(self.doc + 1)
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        if self.doc >= self.max_doc {
            self.doc = NO_MORE_DOCS;
            return Ok(NO_MORE_DOCS);
        }

        self.start = self.addresses.get(target as i64)?;
        self.end = self.addresses.get((target + 1) as i64)?;
        self.count = (self.end - self.start) as i32;
        self.doc = target;

        Ok(self.doc)
    }

    fn cost(&self) -> Result<i64> {
        Ok(self.max_doc as i64)
    }
}

impl<I> SortedNumericDocValues for DenseSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn next_value(&mut self) -> Result<i64> {
        let value = self.values.get(self.start)?;
        self.start += 1;
        Ok(value)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        Ok(self.count)
    }
}
pub struct SpareSortedNumericDocValues<I>
where
    I: IndexInput,
{
    disi: IndexedDISI<I>,
    values: LongValuesEnum<I>,
    addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    set: bool,
    start: i64,
    end: i64,
    count: i32,
}
impl<I> SpareSortedNumericDocValues<I>
where
    I: IndexInput,
{
    pub fn new(
        disi: IndexedDISI<I>,
        values: LongValuesEnum<I>,
        addresses: DirectMonotonicReader<I::RandomAccessSlice>,
    ) -> Self {
        Self {
            disi,
            values,
            addresses,
            set: false,
            start: 0,
            end: 0,
            count: 0,
        }
    }

    fn set(&mut self) -> Result<()> {
        if !self.set {
            let index = self.disi.index();
            self.start = self.addresses.get(index as i64)?;
            self.end = self.addresses.get((index as i64) + 1)?;
            self.count = (self.end - self.start) as i32;
            self.set = true;
        }
        Ok(())
    }
}
impl<I> DocIdSetIterator for SpareSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn doc_id(&self) -> i32 {
        self.disi.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.set = false;
        self.disi.next_doc()
    }

    fn advance(&mut self, target: i32) -> Result<i32> {
        self.set = false;
        self.disi.advance(target)
    }

    fn cost(&self) -> Result<i64> {
        self.disi.cost()
    }
}

impl<I> DocValuesIterator for SpareSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn advance_exact(&mut self, target: i32) -> Result<bool> {
        self.set = false;
        self.disi.advance_exact(target)
    }
}
impl<I> SortedNumericDocValues for SpareSortedNumericDocValues<I>
where
    I: IndexInput,
{
    fn next_value(&mut self) -> Result<i64> {
        self.set()?;
        let value = self.values.get(self.start)?;
        self.start += 1;
        Ok(value)
    }

    fn doc_value_count(&mut self) -> Result<i32> {
        self.set()?;
        Ok(self.count)
    }
}
