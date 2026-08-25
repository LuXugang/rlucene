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

use crate::core::codecs::fields_consumer::FieldsConsumer;
use crate::core::codecs::fields_producer::FieldsProducer;
use crate::core::codecs::norms_producer::NormsProducer;
use crate::core::codecs::perfield::per_field_merge_state::PerFieldMergeState;
use crate::core::codecs::postings_format::PostingsFormat;
use crate::core::index::fields::Fields;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_reader::Identity;
use crate::core::index::merge_state::MergeStateAccess;
use crate::core::index::segment_info::SegmentInfo;
use crate::core::index::segment_read_state::SegmentReadState;
use crate::core::index::segment_write_state::SegmentWriteState;
use crate::core::store::directory::Directory;
use crate::core::store::{IndexInput, IndexOutput};
use crate::core::util::close::{Closeable, CloseableRef};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::iterator::{IteratorExt, VecIter, VecIteratorExt};
use crate::core::util::merged_iterator::MergedIterator;
use crate::core::util::{HasIdentity, IOUtils};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Name of this [`PostingsFormat`].
pub const PER_FIELD_NAME: &str = "PerField40";

/// [`FieldInfo`](crate::core::index::field_info::FieldInfo) attribute name used to store the format
/// name for each field.
pub const PER_FIELD_FORMAT_KEY: &str = "PerFieldPostingsFormat.format";

/// [`FieldInfo`](crate::core::index::field_info::FieldInfo) attribute name used to store the segment
/// suffix name for each field.
pub const PER_FIELD_SUFFIX_KEY: &str = "PerFieldPostingsFormat.suffix";

/// Static-dispatch access to the format selection and name lookup needed by
/// [`PerFieldPostingsFormat`].
pub trait PerFieldPostingsFormatBase {
  type Format: PostingsFormat;

  /// Returns the postings format that should be used for writing new segments
  /// of `field`.
  ///
  /// The field-to-format mapping is written to the index, so this method is
  /// only invoked when writing, not when reading.
  fn get_postings_format_for_field(&self, field: &str) -> Result<&Self::Format>;
}

/// Enables per-field postings support.
///
/// The selected postings format's name is written into the index. In order for
/// a field to be read, that name must resolve to the same implementation
/// through [`PostingsFormat::for_name`].
///
/// Files written by each postings format have an additional suffix containing
/// the format name. For example, in a per-field configuration, a file named
/// `_1.prx` would instead look like `_1_Lucene40_0.prx`.
///
/// # Experimental
pub struct PerFieldPostingsFormat<B> {
  base: Arc<B>,
  identity: Identity,
}

impl<B> Clone for PerFieldPostingsFormat<B> {
  fn clone(&self) -> Self {
    Self {
      base: Arc::clone(&self.base),
      identity: self.identity.clone(),
    }
  }
}

impl<B> PerFieldPostingsFormat<B> {
  /// Sole constructor.
  pub fn new(base: B) -> Self {
    Self {
      base: Arc::new(base),
      identity: Identity::new(),
    }
  }
}

impl<B> HasIdentity for PerFieldPostingsFormat<B> {
  fn identity(&self) -> &Identity {
    &self.identity
  }
}

/// Group of fields written by one postings format.
///
/// `state` is the custom segment-write state for this group of fields, with a
/// segment suffix unique to this postings format.
struct FieldsGroup<'a, D> {
  fields: Vec<String>,
  #[allow(dead_code)]
  // Mirrors the Java record component; grouping uses the builder's suffix directly.
  suffix: i32,
  state: SegmentWriteState<'a, D>,
}

type FieldsGroupMapping<'a, 'b, F, D> = HashMap<Identity, (&'b F, FieldsGroup<'a, D>)>;

struct FieldsGroupBuilder<'a, D> {
  fields: HashSet<String>,
  suffix: i32,
  state: SegmentWriteState<'a, D>,
}

impl<'a, D> FieldsGroupBuilder<'a, D> {
  fn new(suffix: i32, state: SegmentWriteState<'a, D>) -> Self {
    Self {
      fields: HashSet::new(),
      suffix,
      state,
    }
  }

  fn add_field(&mut self, field: String) {
    self.fields.insert(field);
  }

  fn build(self) -> FieldsGroup<'a, D> {
    let mut fields: Vec<String> = self.fields.into_iter().collect();
    fields.sort();
    FieldsGroup {
      fields,
      suffix: self.suffix,
      state: self.state,
    }
  }
}

fn get_suffix(format_name: &str, suffix: impl Display) -> String {
  format!("{format_name}_{suffix}")
}

fn get_full_segment_suffix(
  field_name: &str,
  outer_segment_suffix: &str,
  segment_suffix: &str,
) -> Result<String> {
  if outer_segment_suffix.is_empty() {
    Ok(segment_suffix.to_string())
  } else {
    // return outerSegmentSuffix + "_" + segmentSuffix;
    Err(LuceneError::illegal_state(format!(
      "cannot embed PerFieldPostingsFormat inside itself (field \"{field_name}\" returned PerFieldPostingsFormat)"
    )))
  }
}

pub struct FieldsWriter<B> {
  base: Arc<B>,
  /// First delegate close error; later close errors are suppressed into it.
  /// It is returned by [`Closeable::close`] so one close failure does not prevent
  /// the remaining formats from being written.
  close_error: Option<LuceneError>,
}

impl<B> FieldsWriter<B> {
  fn new(base: Arc<B>) -> Self {
    Self {
      base,
      close_error: None,
    }
  }
}

impl<B> FieldsWriter<B>
where
  B: PerFieldPostingsFormatBase,
{
  fn build_fields_group_mapping<'a, 'b, 'c, D1, I>(
    base: &'b B,
    write_state: &SegmentWriteState<'a, D1>,
    indexed_field_names: &mut I,
  ) -> Result<FieldsGroupMapping<'a, 'b, B::Format, D1>>
  where
    D1: Directory,
    I: IteratorExt<Item = &'c String>,
  {
    // Maps a PostingsFormat instance to the suffix it should use.
    let mut format_to_group_builders: HashMap<
      Identity,
      (&'b B::Format, FieldsGroupBuilder<'a, D1>),
    > = HashMap::new();

    // Holds last suffix of each PostingsFormat name.
    let mut suffixes: HashMap<String, i32> = HashMap::new();

    // Assign field -> PostingsFormat.
    while indexed_field_names.has_next()? {
      let field = indexed_field_names
        .next()?
        .ok_or_else(|| LuceneError::illegal_state("indexedFieldNames.has_next returned true"))?;
      let field_info = write_state
        .field_infos
        .field_info_by_name(field)?
        .ok_or_else(|| {
          LuceneError::illegal_state(format!("missing FieldInfo for field {field}"))
        })?;
      let format = base.get_postings_format_for_field(field)?;
      let format_name = format.get_name();
      let identity = format.identity().clone();

      let group_builder = match format_to_group_builders.entry(identity) {
        Entry::Occupied(entry) => {
          let group_builder = &mut entry.into_mut().1;
          // We've already seen this format, so just grab its suffix.
          if !suffixes.contains_key(format_name) {
            return Err(LuceneError::illegal_state(format!(
              "no suffix for format name: {format_name}, expected: {}",
              group_builder.suffix
            )));
          }
          group_builder
        },
        Entry::Vacant(entry) => {
          // First time we are seeing this format; create a new instance.

          // Bump the suffix.
          let suffix = suffixes
            .entry(format_name.to_string())
            .and_modify(|suffix| *suffix += 1)
            .or_insert(0);
          let segment_suffix = get_full_segment_suffix(
            field,
            &write_state.segment_suffix,
            &get_suffix(format_name, *suffix),
          )?;
          &mut entry
            .insert((
              format,
              FieldsGroupBuilder::new(
                *suffix,
                SegmentWriteState::copy_with_suffix(write_state, segment_suffix),
              ),
            ))
            .1
        },
      };

      group_builder.add_field(field.clone());
      field_info.put_attribute(PER_FIELD_FORMAT_KEY.to_string(), format_name.to_string());
      field_info.put_attribute(
        PER_FIELD_SUFFIX_KEY.to_string(),
        group_builder.suffix.to_string(),
      );
    }

    let mut format_to_groups = HashMap::with_capacity(format_to_group_builders.len());
    for (identity, (format, builder)) in format_to_group_builders {
      format_to_groups.insert(identity, (format, builder.build()));
    }
    Ok(format_to_groups)
  }
}

impl<B> FieldsConsumer for FieldsWriter<B>
where
  B: PerFieldPostingsFormatBase,
{
  fn write<D1, D2, F, N>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    fields: &mut F,
    norms: Option<&N>,
  ) -> Result<()>
  where
    D1: Directory,
    F: Fields,
    N: NormsProducer,
  {
    let base = Arc::clone(&self.base);
    let groups = {
      let mut indexed_field_names = fields.iterator()?;
      Self::build_fields_group_mapping(base.as_ref(), write_state, &mut indexed_field_names)
    }?;

    // Write postings.
    for (format, group) in groups.into_values() {
      // Exposes only the fields from this group.
      let mut masked_fields = FilterFields::new(fields, &group.fields);
      let mut consumer = format.fields_consumer(&group.state, segment_info)?;
      let write_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        consumer.write(&group.state, segment_info, &mut masked_fields, norms)
      }));
      let close_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| consumer.close()));

      match close_result {
        Ok(Ok(())) => {},
        Ok(Err(error)) => {
          self.close_error = Some(IOUtils::use_or_suppress(self.close_error.take(), error));
        },
        Err(payload) => {
          return IOUtils::use_or_suppress_caught_result(write_result, Err(payload));
        },
      }

      unwrap_caught_result!(write_result)?;
    }
    Ok(())
  }

  fn merge<D1, D2, N, MS>(
    &mut self,
    write_state: &SegmentWriteState<D1>,
    segment_info: &SegmentInfo<D2>,
    merge_state: &MS,
    norms: Option<&N>,
  ) -> Result<()>
  where
    D1: Directory,
    N: NormsProducer,
    MS: MergeStateAccess,
  {
    let mut iterators = Vec::new();
    for fields_producer in merge_state.fields_producers().iter().flatten() {
      iterators.push(fields_producer.iterator()?);
    }
    let mut indexed_field_names = MergedIterator::new(iterators)?;
    let base = Arc::clone(&self.base);
    let groups =
      Self::build_fields_group_mapping(base.as_ref(), write_state, &mut indexed_field_names)?;

    // Merge postings.
    for (format, group) in groups.into_values() {
      let mut consumer = format.fields_consumer(&group.state, segment_info)?;
      let merge_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let restricted = PerFieldMergeState::restrict_fields(merge_state, &group.fields)?;
        consumer.merge(&group.state, segment_info, &restricted, norms)
      }));
      let close_result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| consumer.close()));

      match close_result {
        Ok(Ok(())) => {},
        Ok(Err(error)) => {
          self.close_error = Some(IOUtils::use_or_suppress(self.close_error.take(), error));
        },
        Err(payload) => {
          return IOUtils::use_or_suppress_caught_result(merge_result, Err(payload));
        },
      }

      unwrap_caught_result!(merge_result)?;
    }
    Ok(())
  }
}

impl<B> Closeable for FieldsWriter<B>
where
  B: PerFieldPostingsFormatBase,
{
  fn close(&mut self) -> Result<()> {
    match self.close_error.take() {
      Some(error) => Err(error),
      None => Ok(()),
    }
  }
}

pub struct FieldsReader<FP> {
  fields: HashMap<String, Arc<FP>>,
  field_names: Vec<String>,
  formats: HashMap<String, Arc<FP>>,
  segment: String,
}

impl<FP> FieldsReader<FP>
where
  FP: FieldsProducer,
{
  #[cfg(test)]
  pub(crate) fn map_producers<T, F>(self, mut mapper: F) -> Result<FieldsReader<T>>
  where
    T: FieldsProducer,
    F: FnMut(FP) -> T,
  {
    let field_formats: HashMap<String, *const FP> = self
      .fields
      .into_iter()
      .map(|(field, producer)| (field, Arc::as_ptr(&producer)))
      .collect();
    let mut old_to_new = HashMap::with_capacity(self.formats.len());
    let mut formats = HashMap::with_capacity(self.formats.len());
    for (segment_suffix, producer) in self.formats {
      let old_ptr = Arc::as_ptr(&producer);
      let producer = Arc::try_unwrap(producer).map_err(|_| {
        LuceneError::illegal_state(format!(
          "postings producer for segment suffix {segment_suffix} is still shared"
        ))
      })?;
      let producer = Arc::new(mapper(producer));
      old_to_new.insert(old_ptr, Arc::clone(&producer));
      formats.insert(segment_suffix, producer);
    }
    let mut fields = HashMap::with_capacity(field_formats.len());
    for (field, old_ptr) in field_formats {
      let producer = old_to_new.get(&old_ptr).ok_or_else(|| {
        LuceneError::illegal_state(format!("missing postings producer for field: {field}"))
      })?;
      fields.insert(field, Arc::clone(producer));
    }
    Ok(FieldsReader {
      fields,
      field_names: self.field_names,
      formats,
      segment: self.segment,
    })
  }

  // Clone for merge.
  fn from_other(other: &Self) -> Result<Self> {
    let mut fields = HashMap::with_capacity(other.fields.len());
    let mut formats = HashMap::with_capacity(other.formats.len());
    let mut old_to_new = HashMap::with_capacity(other.formats.len());
    // First clone all formats.
    for (segment_suffix, format) in &other.formats {
      let values = match format.as_ref().get_merge_instance()? {
        Some(format) => Arc::new(format),
        None => Arc::clone(format),
      };
      formats.insert(segment_suffix.clone(), Arc::clone(&values));
      old_to_new.insert(Arc::as_ptr(format), values);
    }

    // Then rebuild fields.
    for (field, format) in &other.fields {
      let producer = old_to_new.get(&Arc::as_ptr(format)).ok_or_else(|| {
        LuceneError::illegal_state(format!("missing merge instance for field: {field}"))
      })?;
      fields.insert(field.clone(), Arc::clone(producer));
    }

    Ok(Self {
      fields,
      field_names: other.field_names.clone(),
      formats,
      segment: other.segment.clone(),
    })
  }

  fn new<PF, D1, D2>(
    read_state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self>
  where
    PF: PostingsFormat<FieldsProducer<D1::IndexInput> = FP>,
    D1: Directory,
  {
    let mut fields = HashMap::new();
    let mut formats: HashMap<String, Arc<FP>> = HashMap::new();

    let mut success = false;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
      for field_info in read_state.field_infos.iter() {
        if field_info.get_index_options() == &IndexOptions::None {
          continue;
        }
        let field_name = &field_info.name;
        let Some(format_name) = field_info.get_attribute(PER_FIELD_FORMAT_KEY) else {
          continue;
        };
        let suffix = field_info
          .get_attribute(PER_FIELD_SUFFIX_KEY)
          .ok_or_else(|| {
            LuceneError::illegal_state(format!(
              "missing attribute: {PER_FIELD_SUFFIX_KEY} for field: {field_name}"
            ))
          })?;
        let segment_suffix = get_suffix(&format_name, suffix);
        if !formats.contains_key(&segment_suffix) {
          let format = PF::for_name(&format_name)?;
          let state = SegmentReadState::copy_with_suffix(read_state, &segment_suffix);
          let producer = Arc::new(format.fields_producer(&state, segment_info)?);
          formats.insert(segment_suffix.clone(), producer);
        }
        let producer = formats.get(&segment_suffix).ok_or_else(|| {
          LuceneError::illegal_state(format!("missing postings producer for field: {field_name}"))
        })?;
        fields.insert(field_name.clone(), Arc::clone(producer));
      }
      success = true;
      Ok(())
    }));

    if !success {
      IOUtils::close_while_handling_exception_with(formats.values(), |format| format.close());
    }
    unwrap_caught_result!(result)?;

    let mut field_names: Vec<String> = fields.keys().cloned().collect();
    field_names.sort();
    Ok(Self {
      fields,
      field_names,
      formats,
      segment: segment_info.name.clone(),
    })
  }
}

impl<FP> Display for FieldsReader<FP> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "PerFieldPostings(segment={} formats={})",
      self.segment,
      self.formats.len()
    )
  }
}

impl<FP> Fields for FieldsReader<FP>
where
  FP: FieldsProducer,
{
  type FieldIter<'a>
    = VecIter<'a, String>
  where
    Self: 'a;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    Ok(self.field_names.iter_ext())
  }

  type Terms = FP::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    match self.fields.get(field) {
      Some(fields_producer) => fields_producer.terms(field),
      None => Ok(None),
    }
  }

  fn size(&self) -> Result<i32> {
    Ok(self.fields.len() as i32)
  }
}

impl<FP> CloseableRef for FieldsReader<FP>
where
  FP: CloseableRef,
{
  fn close(&self) -> Result<()> {
    IOUtils::close_with(self.formats.values(), |format| format.close())
  }
}

impl<FP> FieldsProducer for FieldsReader<FP>
where
  FP: FieldsProducer,
{
  fn check_integrity(&self) -> Result<()> {
    for format in self.formats.values() {
      format.check_integrity()?;
    }
    Ok(())
  }

  fn get_merge_instance(&self) -> Result<Option<Self>>
  where
    Self: Sized,
  {
    Ok(Some(Self::from_other(self)?))
  }
}

struct FilterFields<'a, F> {
  in_: &'a F,
  fields: &'a Vec<String>,
}

impl<'a, F> FilterFields<'a, F> {
  fn new(in_: &'a F, fields: &'a Vec<String>) -> Self {
    Self { in_, fields }
  }
}

impl<F> Fields for FilterFields<'_, F>
where
  F: Fields,
{
  type FieldIter<'a>
    = VecIter<'a, String>
  where
    Self: 'a;

  fn iterator(&self) -> Result<Self::FieldIter<'_>> {
    Ok(self.fields.iter_ext())
  }

  type Terms = F::Terms;

  fn terms(&self, field: &str) -> Result<Option<Self::Terms>> {
    self.in_.terms(field)
  }

  fn size(&self) -> Result<i32> {
    self.in_.size()
  }
}

impl<B> PostingsFormat for PerFieldPostingsFormat<B>
where
  B: PerFieldPostingsFormatBase,
{
  fn get_name(&self) -> &str {
    PER_FIELD_NAME
  }

  type FieldsConsumer<O: IndexOutput> = FieldsWriter<B>;

  fn fields_consumer<D1, D2>(
    &self,
    _state: &SegmentWriteState<D1>,
    _segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsConsumer<D1::IndexOutput>>
  where
    D1: Directory,
  {
    Ok(FieldsWriter::new(self.base.clone()))
  }

  type FieldsProducer<I: IndexInput> =
    FieldsReader<<B::Format as PostingsFormat>::FieldsProducer<I>>;

  fn fields_producer<D1, D2>(
    &self,
    state: &SegmentReadState<D1>,
    segment_info: &SegmentInfo<D2>,
  ) -> Result<Self::FieldsProducer<D1::IndexInput>>
  where
    D1: Directory,
  {
    FieldsReader::new::<B::Format, D1, D2>(state, segment_info)
  }

  fn for_name(name: &str) -> Result<Arc<Self>> {
    Err(LuceneError::illegal_argument(format!(
      "Could not load postings format named \"{name}\""
    )))
  }
}
