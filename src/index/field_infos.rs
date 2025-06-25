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
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
use crate::index::doc_values_type::DocValuesType;
use crate::index::field_info::FieldInfo;
use crate::index::index_options::IndexOptions;
use crate::index::vector_encoding::VectorEncoding;
use crate::index::vector_similarity_function::VectorSimilarityFunction;
use crate::util::error::lucene_error::{LuceneError, Result};

/// Collection of FieldInfos (accessible by number or by name).
///
/// # Experimental
pub struct FieldInfos {
    pub has_freq: bool,
    pub has_postings: bool,
    pub has_prox: bool,
    pub has_payloads: bool,
    pub has_offsets: bool,
    pub has_term_vectors: bool,
    pub has_norms: bool,
    pub has_doc_values: bool,
    pub has_point_values: bool,
    pub has_vector_values: bool,
    pub soft_deletes_field: Option<String>,

    pub parent_field: Option<String>,
    pub by_number: Vec<Rc<FieldInfo>>,
    pub by_name: HashMap<String, Rc<FieldInfo>>,
    pub values: Vec<Rc<FieldInfo>>,
}

impl FieldInfos {
    /// Constructs a new FieldInfos from an array of FieldInfo objects. The
    /// array can be used directly as the backing structure.
    pub fn new(mut infos: Vec<Rc<FieldInfo>>) -> Result<Self> {
        let mut has_term_vectors = false;
        let mut has_postings = false;
        let mut has_prox = false;
        let mut has_payloads = false;
        let mut has_offsets = false;
        let mut has_freq = false;
        let mut has_norms = false;
        let mut has_doc_values = false;
        let mut has_point_values = false;
        let mut has_vector_values = false;
        let mut soft_deletes_field: Option<String> = None;
        let mut parent_field: Option<String> = None;

        let mut by_name = HashMap::new();
        let mut max_field_number = -1;
        let mut field_number_strictly_ascending = true;

        for info in &infos {
            let field_number = info.number;
            if field_number < 0 {
                return Err(LuceneError::illegal_argument(format!(
                    "illegal field number: {} for field {}",
                    info.number, info.name
                )));
            }
            if field_number > max_field_number {
                max_field_number = field_number;
            } else {
                field_number_strictly_ascending = false;
            }
            if let Some(previous) = by_name.insert(info.name.clone(), info.clone()) {
                return Err(LuceneError::illegal_argument(format!(
                    "duplicate field names: {} and {} have: {}",
                    previous.number, info.number, info.name
                )));
            }

            has_term_vectors |= info.has_term_vectors();
            has_postings |= info.get_index_options() != &IndexOptions::None;
            has_prox |= info.get_index_options() >= &IndexOptions::DocsAndFreqsAndPositions;
            has_freq |= info.get_index_options() != &IndexOptions::Docs;
            has_offsets |=
                info.get_index_options() >= &IndexOptions::DocsAndFreqsAndPositionsAndOffsets;
            has_norms |= info.has_norms();
            has_doc_values |= info.get_doc_values_type() != &DocValuesType::None;
            has_payloads |= info.has_payloads();
            has_point_values |= info.get_point_dimension_count() != 0;
            has_vector_values |= info.get_vector_dimension() != 0;
            if info.is_soft_deletes_field() {
                if let Some(ref s) = soft_deletes_field {
                    if s != &info.name {
                        return Err(LuceneError::illegal_argument(format!(
                            "multiple soft-deletes fields [{} , {}]",
                            info.name, s
                        )));
                    }
                } else {
                    soft_deletes_field = Some(info.name.clone());
                }
            }
            if info.is_parent_field() {
                if let Some(ref p) = parent_field {
                    if p != &info.name {
                        return Err(LuceneError::illegal_argument(format!(
                            "multiple parent fields [{} , {}]",
                            info.name, p
                        )));
                    }
                } else {
                    parent_field = Some(info.name.clone());
                }
            }
        }

        if field_number_strictly_ascending && (max_field_number as usize == infos.len() - 1) {
            // The input FieldInfo[] contains all fields numbered from 0 to
            // infos.length - 1, and they are sorted, use it
            // directly. This is an optimization when reading a segment with all
            // fields since the FieldInfo[] is sorted.
        } else {
            infos.sort_by(|a, b| a.number.cmp(&b.number));
            #[cfg(debug_assertions)]
            {
                let mut seen_numbers = HashSet::new();
                for field in infos.iter() {
                    debug_assert!(
                        seen_numbers.insert(field.number),
                        "Duplicate number found: {}",
                        field.number
                    );
                }
            }
        }
        let by_number: Vec<Rc<FieldInfo>> = infos.clone();

        Ok(FieldInfos {
            has_freq,
            has_postings,
            has_prox,
            has_payloads,
            has_offsets,
            has_term_vectors,
            has_norms,
            has_doc_values,
            has_point_values,
            has_vector_values,
            soft_deletes_field,
            parent_field,
            by_number,
            by_name,
            values: infos.clone(),
        })
    }
    /// Returns true if any fields have freqs.
    pub fn has_freq(&self) -> bool {
        self.has_freq
    }

    /// Returns true if any fields have postings.
    pub fn has_postings(&self) -> bool {
        self.has_postings
    }

    /// Returns true if any fields have positions.
    pub fn has_prox(&self) -> bool {
        self.has_prox
    }

    /// Returns true if any fields have payloads.
    pub fn has_payloads(&self) -> bool {
        self.has_payloads
    }

    /// Returns true if any fields have offsets.
    pub fn has_offsets(&self) -> bool {
        self.has_offsets
    }

    /// Returns true if any fields have term vectors.
    pub fn has_term_vectors(&self) -> bool {
        self.has_term_vectors
    }

    /// Returns true if any fields have norms.
    pub fn has_norms(&self) -> bool {
        self.has_norms
    }

    /// Returns true if any fields have DocValues.
    pub fn has_doc_values(&self) -> bool {
        self.has_doc_values
    }

    /// Returns true if any fields have PointValues.
    pub fn has_point_values(&self) -> bool {
        self.has_point_values
    }

    /// Returns true if any fields have vector values.
    pub fn has_vector_values(&self) -> bool {
        self.has_vector_values
    }

    /// Returns the soft-deletes field name if it exists; otherwise returns
    /// None.
    pub fn get_soft_deletes_field(&self) -> Option<&String> {
        self.soft_deletes_field.as_ref()
    }

    /// Returns the parent document field name if it exists; otherwise returns
    /// None.
    pub fn get_parent_field(&self) -> Option<&str> {
        self.parent_field.as_deref()
    }

    /// Returns the number of fields.
    pub fn size(&self) -> usize {
        self.by_name.len()
    }

    /// Returns an iterator over all the FieldInfo objects present, ordered by
    /// ascending field number.
    pub fn iter(&self) -> impl Iterator<Item = &Rc<FieldInfo>> {
        self.values.iter()
    }

    /// Return the FieldInfo object referenced by the field name.
    ///
    /// Returns None if the given field name doesn't exist.
    pub fn field_info_by_name(&self, field_name: &str) -> Option<Rc<FieldInfo>> {
        self.by_name.get(field_name).cloned()
    }

    /// Return the FieldInfo object referenced by the field number.
    ///
    /// Returns None if the given field number doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if field_number is negative.
    pub fn field_info_by_number(&self, field_number: i32) -> Result<Option<Rc<FieldInfo>>> {
        if field_number < 0 {
            return Err(LuceneError::illegal_argument(format!(
                "Illegal field number: {}",
                field_number
            )));
        }
        match self.by_number.get(field_number as usize) {
            Some(fi) => Ok(Some(fi.clone())),
            None => Ok(None),
        }
    }
}
impl<'a> IntoIterator for &'a FieldInfos {
    type Item = &'a Rc<FieldInfo>;
    type IntoIter = std::slice::Iter<'a, Rc<FieldInfo>>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

//TODO:
// /// Call this to get the (merged) FieldInfos for a composite reader.
// ///
// /// # NOTE
// /// the returned field numbers will likely not correspond to the actual field
// numbers in /// the underlying readers, and codec metadata
// (FieldInfo::get_attribute) will be unavailable.
// pub fn get_merged_field_infos(reader: &impl IndexReader) ->
// Result<FieldInfos> {     let leaves = reader.leaves();
//     if leaves.is_empty() {
//         return Ok(FieldInfos::empty());
//     } else if leaves.len() == 1 {
//         return Ok(leaves[0].reader().get_field_infos().clone());
//     } else {
//         let soft_deletes_field = leaves
//             .iter()
//             .filter_map(|l|
// l.reader().get_field_infos().get_soft_deletes_field().cloned())
// .next();         let parent_field = get_and_validate_parent_field(&leaves)?;
//         let mut builder = Builder::new(FieldNumbers::new(soft_deletes_field,
// Some(parent_field))?);         for ctx in leaves {
//             for fi in ctx.reader().get_field_infos().iter() {
//                 builder.add(fi.clone());
//             }
//         }
//         builder.finish()
//     }
// }
//
// /// Helper function to validate and retrieve the parent field name across
// leaves. fn get_and_validate_parent_field(leaves: &[LeafReaderContext]) ->
// Result<String> {     let mut set = false;
//     let mut the_field: Option<String> = None;
//     for ctx in leaves {
//         let field =
// ctx.reader().get_field_infos().get_parent_field().unwrap_or_default();
//         if set {
//             if the_field.as_ref() != Some(&field) {
//                 return Err(LuceneError::illegal_state(format!(
//                     "expected parent doc field to be \"{}\" across all
// segments but found a segment with different field \"{}\"",
// the_field.unwrap_or_default(),                     field
//                 )));
//             }
//         } else {
//             the_field = Some(field);
//             set = true;
//         }
//     }
//     Ok(the_field.unwrap_or_default())
// }
//
// /// Returns a set of names of fields that have a terms index. The order is
// undefined. pub fn get_indexed_fields(reader: &impl IndexReader) ->
// HashSet<String> {     reader
//         .leaves()
//         .iter()
//         .flat_map(|l| {
//             l.reader()
//                 .get_field_infos()
//                 .iter()
//                 .filter(|fi| fi.get_index_options() != IndexOptions::None)
//                 .map(|fi| fi.name.clone())
//         })
//         .collect()
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldDimensions {
    pub dimension_count: i32,
    pub index_dimension_count: i32,
    pub dimension_num_bytes: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldVectorProperties {
    pub num_dimensions: i32,
    pub vector_encoding: VectorEncoding,
    pub similarity_function: VectorSimilarityFunction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexOptionsProperties {
    pub store_term_vectors: bool,
    pub omit_norms: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldProperties {
    pub number: i32,
    pub index_options: IndexOptions,
    pub index_options_properties: Option<IndexOptionsProperties>,
    pub doc_values_type: DocValuesType,
    pub doc_values_skip_index: DocValuesSkipIndexType,
    pub field_dimensions: FieldDimensions,
    pub field_vector_properties: FieldVectorProperties,
}

pub(crate) struct FieldNumbers {
    pub field_properties: Rc<RefCell<Property>>,
    pub soft_deletes_field_name: Option<String>,
    // The parent document field from IWC to mark parent document when indexing
    pub parent_field_name: Option<String>,
    // The soft-deletes field from IWC to enforce a single soft-deletes field
}
pub(crate) struct Property {
    pub number_to_name: HashMap<i32, String>,
    pub properties: HashMap<String, FieldProperties>,
    // TODO: we should similarly catch an attempt to turn
    // norms back on after they were already committed; today
    // we silently discard the norm but this is badly trappy
    pub lowest_unassigned_field_number: i32,
}

#[allow(unused)]
impl FieldNumbers {
    pub(crate) fn new(
        soft_deletes_field_name: Option<String>,
        parent_field_name: Option<String>,
    ) -> Result<Self> {
        if let (Some(ref soft), Some(ref parent)) = (&soft_deletes_field_name, &parent_field_name) {
            if soft == parent {
                return Err(LuceneError::illegal_argument(format!(
                    "parent document and soft-deletes field can't be the same field \"{}\"",
                    parent
                )));
            }
        }
        let properties = Property {
            number_to_name: HashMap::new(),
            properties: HashMap::new(),
            lowest_unassigned_field_number: -1,
        };
        Ok(FieldNumbers {
            field_properties: Rc::new(RefCell::new(properties)),
            soft_deletes_field_name,
            parent_field_name,
        })
    }
    pub fn verify_field_info(&self, fi: &FieldInfo) -> Result<()> {
        let field_properties_guard = self.field_properties.borrow();
        let field_name = fi.get_name();
        self.verify_soft_deleted_field_name(field_name, fi.is_soft_deletes_field())?;
        self.verify_parent_field_name(field_name, fi.is_parent_field())?;
        if field_properties_guard.properties.contains_key(field_name) {
            self.verify_same_schema(fi, &field_properties_guard.properties)?;
        }
        Ok(())
    }

    pub(crate) fn add_or_get(&self, fi: Rc<FieldInfo>) -> Result<i32> {
        let mut field_properties_guard = self.field_properties.borrow_mut();
        self.add_or_get_impl(fi, &mut field_properties_guard)
    }
    /// Returns the global field number for the given field name. If the name
    /// does not exist yet it tries to add it with the given preferred field
    /// number assigned if possible otherwise the first unassigned field
    /// number is used as the field number.
    pub(crate) fn add_or_get_impl(
        &self,
        fi: Rc<FieldInfo>,
        field_properties: &mut Property,
    ) -> Result<i32> {
        let field_name = fi.get_name();
        self.verify_soft_deleted_field_name(field_name, fi.is_soft_deletes_field())?;
        self.verify_parent_field_name(field_name, fi.is_parent_field())?;
        match field_properties.properties.get(field_name) {
            Some(fp) => {
                self.verify_same_schema(&fi, &field_properties.properties)?;
                Ok(fp.number)
            },
            None => {
                // first time we see this field in this index
                let field_number = if fi.number != -1
                    && !field_properties.number_to_name.contains_key(&fi.number)
                {
                    fi.number
                } else {
                    loop {
                        field_properties.lowest_unassigned_field_number += 1;
                        if !field_properties
                            .number_to_name
                            .contains_key(&field_properties.lowest_unassigned_field_number)
                        {
                            break field_properties.lowest_unassigned_field_number;
                        }
                    }
                };
                debug_assert!(field_number >= 0);
                field_properties
                    .number_to_name
                    .insert(field_number, field_name.to_string());
                let index_options_props = if fi.get_index_options() != &IndexOptions::None {
                    Some(IndexOptionsProperties {
                        store_term_vectors: fi.has_term_vectors(),
                        omit_norms: fi.omits_norms(),
                    })
                } else {
                    None
                };
                let new_props = FieldProperties {
                    number: field_number,
                    index_options: *fi.get_index_options(),
                    index_options_properties: index_options_props,
                    doc_values_type: *fi.get_doc_values_type(),
                    doc_values_skip_index: *fi.doc_values_skip_index_type(),
                    field_dimensions: FieldDimensions {
                        dimension_count: fi.get_point_dimension_count(),
                        index_dimension_count: fi.get_point_index_dimension_count(),
                        dimension_num_bytes: fi.get_point_num_bytes(),
                    },
                    field_vector_properties: FieldVectorProperties {
                        num_dimensions: fi.get_vector_dimension(),
                        vector_encoding: *fi.get_vector_encoding(),
                        similarity_function: *fi.get_vector_similarity_function(),
                    },
                };
                let number = new_props.number;
                field_properties
                    .properties
                    .insert(field_name.to_string(), new_props);
                Ok(number)
            },
        }
    }

    fn verify_soft_deleted_field_name(
        &self,
        field_name: &str,
        is_soft_deletes_field: bool,
    ) -> Result<()> {
        if is_soft_deletes_field {
            if self.soft_deletes_field_name.is_none() {
                return Err(LuceneError::illegal_argument(format!(
                    "this index has [{}] as soft-deletes already but soft-deletes field is not configured in IWC",
                    field_name
                )));
            } else if self.soft_deletes_field_name.as_ref().unwrap() != field_name {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot configure [{}] as soft-deletes; this index uses [{}] as soft-deletes already",
                    self.soft_deletes_field_name.as_ref().unwrap(),
                    field_name
                )));
            }
        } else if let Some(ref soft_name) = self.soft_deletes_field_name {
            if soft_name == field_name {
                return Err(LuceneError::illegal_argument(format!(
                    "cannot configure [{}] as soft-deletes; this index uses [{}] as non-soft-deletes already",
                    soft_name, field_name
                )));
            }
        }
        Ok(())
    }

    fn verify_parent_field_name(&self, field_name: &str, is_parent_field: bool) -> Result<()> {
        if is_parent_field {
            if self.parent_field_name.is_none() {
                return Err(LuceneError::illegal_argument(format!(
                    "can't add field [{}] as parent document field; this IndexWriter has no parent document field configured",
                    field_name
                )));
            } else if self.parent_field_name.as_ref().unwrap() != field_name {
                return Err(LuceneError::illegal_argument(format!(
                    "can't add field [{}] as parent document field; this IndexWriter is configured with [{}] as parent document field",
                    field_name,
                    self.parent_field_name.as_ref().unwrap()
                )));
            }
        } else if let Some(ref parent) = self.parent_field_name {
            // this would be the case if the current index has a parent field
            // that is not a parent field in the incoming index
            // (think addIndices)
            if parent == field_name {
                return Err(LuceneError::illegal_argument(format!(
                    "can't add field [{}] as non parent document field; this IndexWriter is configured with [{}] as parent document field",
                    field_name, parent
                )));
            }
        }
        Ok(())
    }

    fn verify_same_schema(
        &self,
        fi: &FieldInfo,
        props: &HashMap<String, FieldProperties>,
    ) -> Result<()> {
        let field_name = fi.get_name();
        debug_assert!(props.contains_key(field_name));
        let field_properties = props.get(field_name).unwrap();
        FieldInfo::verify_same_index_options(
            field_name,
            &field_properties.index_options,
            fi.get_index_options(),
        )?;
        if field_properties.index_options != IndexOptions::None {
            debug_assert!(field_properties.index_options_properties.is_some());
            let current_term_vector = field_properties
                .index_options_properties
                .as_ref()
                .unwrap()
                .store_term_vectors;
            FieldInfo::verify_same_store_term_vectors(
                field_name,
                current_term_vector,
                fi.has_term_vectors(),
            )?;
            let current_omit_norms = field_properties
                .index_options_properties
                .as_ref()
                .unwrap()
                .omit_norms;
            FieldInfo::verify_same_omit_norms(field_name, current_omit_norms, fi.omits_norms())?;
        }
        FieldInfo::verify_same_doc_values_type(
            field_name,
            &field_properties.doc_values_type,
            fi.get_doc_values_type(),
        )?;
        FieldInfo::verify_same_doc_values_skip_index(
            field_name,
            &field_properties.doc_values_skip_index,
            fi.doc_values_skip_index_type(),
        )?;
        let dims = &field_properties.field_dimensions;
        FieldInfo::verify_same_points_options(
            field_name,
            dims.dimension_count,
            dims.index_dimension_count,
            dims.dimension_num_bytes,
            fi.get_point_dimension_count(),
            fi.get_point_index_dimension_count(),
            fi.get_point_num_bytes(),
        )?;
        let vec_props = &field_properties.field_vector_properties;
        FieldInfo::verify_same_vector_options(
            field_name,
            vec_props.num_dimensions,
            &vec_props.vector_encoding,
            &vec_props.similarity_function,
            fi.get_vector_dimension(),
            fi.get_vector_encoding(),
            fi.get_vector_similarity_function(),
        )?;
        Ok(())
    }
    /// This function is called from `IndexWriter` to verify if doc values of
    /// the field can be updated. If a field with this name already exists,
    /// it verifies that it is a doc-values-only field. If the field does
    /// not exist and `field_must_exist` is `false`, a new field is created in
    /// the global field numbers.
    ///
    /// # Parameters
    /// - `field_name`: Name of the field.
    /// - `dv_type`: Expected doc values type.
    /// - `field_must_exist`: Whether the field must already exist.
    ///
    /// # Errors
    /// - Returns an error if the field must exist but does not.
    /// - Returns an error if the field exists but is not a doc-values-only
    ///   field with the provided doc values type.
    pub fn verify_or_create_dv_only_field(
        &mut self,
        field_name: &str,
        dv_type: DocValuesType,
        field_must_exist: bool,
    ) -> Result<()> {
        let mut field_properties_guard = self.field_properties.borrow_mut();
        if !field_properties_guard.properties.contains_key(field_name) {
            if field_must_exist {
                return Err(LuceneError::illegal_argument(format!(
                    "Can't update [{:?}] doc values; the field [{}] doesn't exist.",
                    dv_type, field_name
                )));
            } else {
                let fi = FieldInfo::new(
                    field_name.to_string(),
                    -1,
                    false,
                    false,
                    false,
                    IndexOptions::None,
                    dv_type,
                    DocValuesSkipIndexType::None,
                    -1,
                    Rc::new(RefCell::new(HashMap::new())),
                    0,
                    0,
                    0,
                    0,
                    VectorEncoding::FLOAT32(4),
                    VectorSimilarityFunction::Euclidean,
                    self.soft_deletes_field_name
                        .as_ref()
                        .is_some_and(|s| s == field_name),
                    self.parent_field_name
                        .as_ref()
                        .is_some_and(|s| s == field_name),
                );
                self.add_or_get_impl(Rc::new(fi), &mut field_properties_guard)?;
            }
        } else {
            // verify that field is doc values only field with the give doc
            // values type
            let field_props = field_properties_guard.properties.get(field_name).unwrap();
            if dv_type != field_props.doc_values_type {
                return Err(LuceneError::illegal_argument(format!(
                    "Can't update [{:?}] doc values; the field [{}] has inconsistent doc values' type of [{:?}].",
                    dv_type, field_name, field_props.doc_values_type
                )));
            }
            if field_props.doc_values_skip_index != DocValuesSkipIndexType::None {
                return Err(LuceneError::illegal_argument(format!(
                    "Can't update [{:?}] doc values; the field [{}] must be doc values only field, but it has doc values skip index",
                    dv_type, field_name
                )));
            }
            if field_props.field_dimensions.dimension_count != 0 {
                return Err(LuceneError::illegal_argument(format!(
                        "Can't update [{:?}] doc values; the field [{}] must be doc values only field, but is also indexed with points.",
                        dv_type, field_name
                    )));
            }
            if field_props.index_options != IndexOptions::None {
                return Err(LuceneError::illegal_argument(format!(
                        "Can't update [{:?}] doc values; the field [{}] must be doc values only field, but is also indexed with postings.",
                        dv_type, field_name
                    )));
            }
            if field_props.field_vector_properties.num_dimensions != 0 {
                return Err(LuceneError::illegal_argument(format!(
                        "Can't update [{:?}] doc values; the field [{}] must be doc values only field, but is also indexed with vectors.",
                        dv_type, field_name
                    )));
            }
        }
        Ok(())
    }

    /// Constructs a new `FieldInfo` based on the options in global field
    /// numbers. This method is not synchronized as all the options it uses
    /// are not modifiable.
    ///
    /// # Parameters
    /// - `field_name`: Name of the field.
    /// - `dv_type`: Doc values type.
    /// - `new_field_number`: A new field number.
    ///
    /// # Returns
    /// - `None` if `field_name` does not exist in the map or is not of the same
    ///   `dv_type`.
    /// - Otherwise, returns a new `FieldInfo` based on the options in global
    ///   field numbers.
    pub fn construct_field_info(
        &self,
        field_name: &str,
        dv_type: DocValuesType,
        new_field_number: i32,
    ) -> Result<Option<FieldInfo>> {
        let field_properties_guard = self.field_properties.borrow();
        let field_props = field_properties_guard.properties.get(field_name);
        if let Some(fp) = field_props {
            if dv_type != fp.doc_values_type {
                return Ok(None);
            }
            let is_soft_deletes_field = self
                .soft_deletes_field_name
                .as_ref()
                .is_some_and(|s| s == field_name);
            let is_parent_field = self
                .parent_field_name
                .as_ref()
                .is_some_and(|s| s == field_name);
            Ok(Some(FieldInfo::new(
                field_name.to_string(),
                new_field_number,
                false,
                false,
                false,
                IndexOptions::None,
                dv_type,
                DocValuesSkipIndexType::None,
                -1,
                Rc::new(RefCell::new(HashMap::new())),
                0,
                0,
                0,
                0,
                VectorEncoding::FLOAT32(4),
                VectorSimilarityFunction::Euclidean,
                is_soft_deletes_field,
                is_parent_field,
            )))
        } else {
            Ok(None)
        }
    }

    /// Returns a set of field names.
    pub fn get_field_names(&self) -> Result<HashSet<String>> {
        let field_properties_guard = self.field_properties.borrow_mut();
        Ok(field_properties_guard.properties.keys().cloned().collect())
    }

    /// Clears the field numbers.
    pub fn clear(&mut self) -> Result<()> {
        let mut field_properties_guard = self.field_properties.borrow_mut();
        field_properties_guard.number_to_name.clear();
        field_properties_guard.properties.clear();
        field_properties_guard.lowest_unassigned_field_number = -1;
        Ok(())
    }
}
pub mod build {
    use crate::index::field_info::FieldInfo;
    use crate::index::field_infos::{FieldInfos, FieldNumbers};
    use crate::util::error::lucene_error::Result;
    use std::collections::HashMap;
    use std::rc::Rc;
    use std::sync::Arc;

    pub struct Builder {
        by_name: HashMap<String, Rc<FieldInfo>>,
        global_field_numbers: Arc<FieldNumbers>,
        finished: bool,
    }
    #[allow(unused)]
    impl Builder {
        pub(crate) fn new(global_field_numbers: Arc<FieldNumbers>) -> Self {
            Self {
                by_name: HashMap::new(),
                global_field_numbers,
                finished: false,
            }
        }

        pub fn get_soft_deletes_field_name(&self) -> &Option<String> {
            &self.global_field_numbers.soft_deletes_field_name
        }

        pub fn get_parent_field_name(&self) -> &Option<String> {
            &self.global_field_numbers.parent_field_name
        }

        pub fn add(&mut self, fi: Rc<FieldInfo>) -> Result<Rc<FieldInfo>> {
            self.add_with_dv_gen(fi, -1)
        }

        pub fn add_with_dv_gen(&mut self, fi: Rc<FieldInfo>, dv_gen: i64) -> Result<Rc<FieldInfo>> {
            if let Some(cur_fi) = self.field_info(&fi.name) {
                cur_fi.verify_same_schema(&fi)?;

                let properties = fi.properties.borrow();
                let attributes_guard = properties.attributes.borrow_mut();
                for (k, v) in attributes_guard.iter() {
                    cur_fi.put_attribute(k.clone(), v.clone());
                }
                if fi.has_payloads() {
                    cur_fi.set_store_payloads()?;
                }
                return Ok(cur_fi.clone());
            }

            self.assert_not_finished();

            let field_number = self.global_field_numbers.add_or_get(fi.clone())?;
            let attributes = fi.properties.borrow().attributes.clone();
            let fi_new = Rc::new(FieldInfo::new(
                fi.name.clone(),
                field_number,
                fi.has_term_vectors(),
                fi.omits_norms(),
                fi.has_payloads(),
                // copy
                *fi.get_index_options(),
                *fi.get_doc_values_type(),
                *fi.doc_values_skip_index_type(),
                dv_gen,
                attributes.clone(),
                fi.get_point_dimension_count(),
                fi.get_point_index_dimension_count(),
                fi.get_point_num_bytes(),
                fi.get_vector_dimension(),
                *fi.get_vector_encoding(),
                *fi.get_vector_similarity_function(),
                fi.is_soft_deletes_field(),
                fi.is_parent_field(),
            ));
            self.by_name.insert(fi_new.name.clone(), fi_new.clone());
            Ok(fi_new)
        }
        pub fn field_info(&self, field_name: &str) -> Option<Rc<FieldInfo>> {
            self.by_name.get(field_name).cloned()
        }
        fn assert_not_finished(&self) {
            if self.finished {
                panic!("FieldInfos.Builder was already finished; cannot add new fields");
            }
        }
        pub fn finish(&mut self) -> Result<FieldInfos> {
            self.finished = true;
            FieldInfos::new(self.by_name.values().cloned().collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use crate::index::doc_values_skip_index_type::DocValuesSkipIndexType;
    use crate::index::doc_values_type::DocValuesType;
    use crate::index::field_info::FieldInfo;
    use crate::index::field_infos::FieldNumbers;
    use crate::index::index_options::IndexOptions;
    use crate::index::vector_encoding::VectorEncoding;
    use crate::index::vector_similarity_function::VectorSimilarityFunction;
    use crate::util::error::lucene_error::Result;

    #[allow(dead_code)] // for quick search
    struct TestFieldInfos;
    #[test]
    fn test_field_infos() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_field_attributes() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_field_attributes_single_segment() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_merged_field_infos_empty() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_merged_field_infos_single_leaf() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_field_numbers_auto_increment() -> Result<()> {
        let mut field_numbers = FieldNumbers::new(
            Some("softDeletes".to_string()),
            Some("parentDoc".to_string()),
        )?;
        for i in 0..10 {
            let fi = FieldInfo::new(
                format!("field{}", i),
                -1,
                false,
                false,
                false,
                IndexOptions::None,
                DocValuesType::None,
                DocValuesSkipIndexType::None,
                -1,
                Rc::new(RefCell::new(HashMap::new())),
                0,
                0,
                0,
                0,
                VectorEncoding::FLOAT32(4),
                VectorSimilarityFunction::Euclidean,
                false,
                false,
            );
            field_numbers.add_or_get(Rc::new(fi))?;
        }
        let idx = field_numbers.add_or_get(Rc::new(FieldInfo::new(
            "EleventhField".to_string(),
            -1,
            false,
            false,
            false,
            IndexOptions::None,
            DocValuesType::None,
            DocValuesSkipIndexType::None,
            -1,
            Rc::new(RefCell::new(HashMap::new())),
            0,
            0,
            0,
            0,
            VectorEncoding::FLOAT32(4),
            VectorSimilarityFunction::Euclidean,
            false,
            false,
        )))?;
        assert_eq!(10, idx, "Field numbers 0 through 9 were allocated");

        field_numbers.clear()?;
        let idx = field_numbers.add_or_get(Rc::new(FieldInfo::new(
            "PostClearField".to_string(),
            -1,
            false,
            false,
            false,
            IndexOptions::None,
            DocValuesType::None,
            DocValuesSkipIndexType::None,
            -1,
            Rc::new(RefCell::new(HashMap::new())),
            0,
            0,
            0,
            0,
            VectorEncoding::FLOAT32(4),
            VectorSimilarityFunction::Euclidean,
            false,
            false,
        )))?;
        assert_eq!(0, idx, "Field numbers should reset after clear()");
        Ok(())
    }
}
