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
use crate::core::codecs::doc_values_producer::{
  DefaultBinary, DefaultDocValuesProducer, DefaultNumeric, DefaultSkipper, DefaultSorted,
  DefaultSortedNumeric, DefaultSortedSet, DocValuesProducer,
};
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::field_info::FieldInfo;
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::segment_commit_info::SegmentCommitInfo;
use crate::core::index::segment_doc_values::SegmentDocValues;
use crate::core::store::directory::Directory;
use crate::core::util::IdentityArc;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Encapsulates multiple producers when there are docvalues updates as one producer
pub struct SegmentDocValuesProducer<D>
where
  D: Directory,
{
  pub(crate) dv_producers_by_field: HashMap<i32, Arc<DefaultDocValuesProducer<D::IndexInput>>>,
  pub(crate) dv_producers: HashSet<IdentityArc<DefaultDocValuesProducer<D::IndexInput>>>,
  pub(crate) dv_gens: Vec<i64>,
}
impl<D> SegmentDocValuesProducer<D>
where
  D: Directory,
{
  pub(crate) fn new<D1>(
    si: &SegmentCommitInfo<D>,
    dir: Option<&D1>,
    core_infos: Arc<FieldInfos>,
    all_infos: &FieldInfos,
    seg_doc_values: &SegmentDocValues<D>,
  ) -> Result<Self>
  where
    D1: Directory<IndexInput = D::IndexInput, IndexOutput = D::IndexOutput, Lock = D::Lock>,
  {
    let mut dv_producers_by_field = HashMap::new();
    let mut dv_producers = HashSet::new();
    let mut dv_gens = Vec::new();

    let mut base_producer = None;

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
            dv_producers.insert(IdentityArc::new(producer.clone()));
            base_producer = Some(producer);
          }
          dv_producers_by_field.insert(fi.number, base_producer.as_ref().unwrap().clone());
        } else {
          debug_assert!(!dv_gens.contains(&doc_values_gen));
          // otherwise, producer sees only the one fieldinfo it wrote
          let field_infos = Arc::new(FieldInfos::new(vec![fi.clone()])?);
          let dvp = seg_doc_values.get_doc_values_producer(doc_values_gen, si, dir, field_infos)?;
          dv_gens.push(doc_values_gen);
          dv_producers.insert(IdentityArc::new(dvp.clone()));
          dv_producers_by_field.insert(fi.number, dvp);
        }
      }
      Ok(())
    })();

    if let Err(mut e) = result {
      if let Err(dec_err) = seg_doc_values.dec_ref(&dv_gens) {
        e.add_suppressed(dec_err);
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

impl<D> CloseableRef for SegmentDocValuesProducer<D>
where
  D: Directory,
{
  fn close(&self) -> Result<()> {
    Err(LuceneError::unsupported_operation(""))
  }
}

impl<D> DocValuesProducer for SegmentDocValuesProducer<D>
where
  D: Directory,
{
  type NumericDocValues = DefaultNumeric<D::IndexInput>;

  fn get_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::NumericDocValues> {
    let dv_producer = self.dv_producers_by_field.get(&field.number);
    debug_assert!(dv_producer.is_some());
    dv_producer.as_ref().unwrap().get_numeric(field)
  }

  type BinaryDocValues = DefaultBinary<D::IndexInput>;

  fn get_binary(&self, field: &Arc<FieldInfo>) -> Result<Self::BinaryDocValues> {
    let dv_producer = self.dv_producers_by_field.get(&field.number);
    debug_assert!(dv_producer.is_some());
    dv_producer.as_ref().unwrap().get_binary(field)
  }

  type SortedDocValues = DefaultSorted<D::IndexInput>;

  fn get_sorted(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedDocValues> {
    let dv_producer = self.dv_producers_by_field.get(&field.number);
    debug_assert!(dv_producer.is_some());
    dv_producer.as_ref().unwrap().get_sorted(field)
  }

  type SortedNumericDocValues = DefaultSortedNumeric<D::IndexInput>;

  fn get_sorted_numeric(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedNumericDocValues> {
    let dv_producer = self.dv_producers_by_field.get(&field.number);
    debug_assert!(dv_producer.is_some());
    dv_producer.as_ref().unwrap().get_sorted_numeric(field)
  }

  type SortedSetDocValues = DefaultSortedSet<D::IndexInput>;

  fn get_sorted_set(&self, field: &Arc<FieldInfo>) -> Result<Self::SortedSetDocValues> {
    let dv_producer = self.dv_producers_by_field.get(&field.number);
    debug_assert!(dv_producer.is_some());
    dv_producer.as_ref().unwrap().get_sorted_set(field)
  }

  type DocValuesSkipper = DefaultSkipper<D::IndexInput>;

  fn get_skipper(&self, field: &Arc<FieldInfo>) -> Result<Option<Self::DocValuesSkipper>> {
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
