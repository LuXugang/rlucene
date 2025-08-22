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
use crate::codecs::compound_directory::CompoundDirectory;
use crate::codecs::doc_values_producer::DocValuesProducer;
use crate::codecs::lucene90::lucene90_doc_values_producer::{
    Lucene90BinaryDocValuesEnum, Lucene90NumericDocValuesEnum, Lucene90SortedNumericDocValuesEnum,
};
use crate::codecs::lucene90_compound_reader::Lucene90CompoundReader;
use crate::codecs::lucene90_doc_values_producer::{
    BaseSortedDocValues, DocValuesSkipperImpl, Lucene90DocValuesProducer,
    Lucene90SortedSetDocValuesEnum,
};
use crate::index::doc_values_type::DocValuesType;
use crate::index::field_info::FieldInfo;
use crate::index::field_infos::FieldInfos;
use crate::index::segment_commit_info::SegmentCommitInfo;
use crate::index::segment_core_readers::CfsOrBaseInput;
use crate::index::segment_doc_values::SegmentDocValues;
use crate::store::directory::Directory;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::{CoreHelper, IdentityRc};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::Arc;

/// Encapsulates multiple producers when there are docvalues updates as one producer
pub(crate) struct SegmentDocValuesProducer<D>
where
    D: Directory,
{
    pub dv_producers_by_field: HashMap<i32, Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>>,
    pub dv_producers: HashSet<IdentityRc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>>,
    pub dv_gens: Vec<i64>,
}
impl<D> SegmentDocValuesProducer<D>
where
    D: Directory,
{
    pub(crate) fn new(
        si: &SegmentCommitInfo<D>,
        dir: &mut CompoundDirectory<Lucene90CompoundReader<D>>,
        core_infos: Rc<FieldInfos>,
        all_infos: &FieldInfos,
        seg_doc_values: &SegmentDocValues<D>,
    ) -> Result<Self> {
        let mut dv_producers_by_field = HashMap::new();
        let mut dv_producers = HashSet::new();
        let mut dv_gens = Vec::new();

        let mut base_producer: Option<Rc<Lucene90DocValuesProducer<CfsOrBaseInput<D>>>> = None;

        let result: Result<()> = (|| {
            for fi in all_infos {
                if *fi.get_doc_values_type() == DocValuesType::None {
                    continue;
                }
                let doc_values_gen = fi.get_doc_values_gen();

                if doc_values_gen == -1 {
                    if base_producer.is_none() {
                        // the base producer gets the original fieldinfos it wrote
                        let producer = seg_doc_values.get_doc_values_producer(
                            doc_values_gen,
                            si,
                            dir,
                            core_infos.clone(),
                        )?;
                        dv_gens.push(doc_values_gen);
                        dv_producers.insert(IdentityRc::new(producer.clone()));
                        base_producer = Some(producer);
                    }
                    dv_producers_by_field
                        .insert(fi.number, base_producer.as_ref().unwrap().clone());
                } else {
                    assert!(!dv_gens.contains(&doc_values_gen));
                    // otherwise, producer sees only the one fieldinfo it wrote
                    let field_infos = Rc::new(FieldInfos::new(vec![fi.clone()])?);
                    let dvp = seg_doc_values.get_doc_values_producer(
                        doc_values_gen,
                        si,
                        dir,
                        field_infos,
                    )?;
                    dv_gens.push(doc_values_gen);
                    dv_producers.insert(IdentityRc::new(dvp.clone()));
                    dv_producers_by_field.insert(fi.number, dvp);
                }
            }
            Ok(())
        })();

        // TODO: IMPORT 这里需要实现LuceneError的嵌套返回
        if let Err(e) = result {
            if let Err(dec_err) = seg_doc_values.dec_ref(&dv_gens) {
                return Err(LuceneError::illegal_state(format!(
                    "{}, caused by {}",
                    dec_err, e
                )));
            }
            return Err(e);
        }

        Ok(Self {
            dv_producers_by_field,
            dv_producers,
            dv_gens,
        })
    }
}

impl<D> Clone for SegmentDocValuesProducer<D>
where
    D: Directory,
{
    fn clone(&self) -> Self {
        unreachable!(
            "{} {}",
            std::any::type_name::<Self>(),
            CoreHelper::CLONE_WARRING
        )
    }
}

impl<D> DocValuesProducer for SegmentDocValuesProducer<D>
where
    D: Directory,
{
    type NumericDocValues = Lucene90NumericDocValuesEnum<CfsOrBaseInput<D>>;

    fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_numeric(field)
    }

    type BinaryDocValues = Lucene90BinaryDocValuesEnum<CfsOrBaseInput<D>>;

    fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_binary(field)
    }

    type SortedDocValues = BaseSortedDocValues<CfsOrBaseInput<D>>;

    fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_sorted(field)
    }

    type SortedNumericDocValues = Lucene90SortedNumericDocValuesEnum<CfsOrBaseInput<D>>;

    fn get_sorted_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_sorted_numeric(field)
    }

    type SortedSetDocValues = Lucene90SortedSetDocValuesEnum<CfsOrBaseInput<D>>;

    fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_sorted_set(field)
    }

    type DocValuesSkipper = DocValuesSkipperImpl<CfsOrBaseInput<D>>;

    fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Self::DocValuesSkipper> {
        let dv_producer = self.dv_producers_by_field.get(&field.number);
        debug_assert!(dv_producer.is_some());
        dv_producer.as_ref().unwrap().get_skipper(field)
    }

    fn check_integrity(&self) -> Result<()> {
        for dv_producer in self.dv_producers.iter() {
            dv_producer.object.check_integrity()?;
        }
        Ok(())
    }
}

impl<D> Display for SegmentDocValuesProducer<D>
where
    D: Directory,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SegmentDocValuesProducer (producers={})",
            self.dv_producers.len()
        )
    }
}
