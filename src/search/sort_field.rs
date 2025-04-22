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
#![allow(deprecated)]
use crate::index::index_sorter::{
    DoubleSorter, FloatSorter, IndexSortEnum, IntSorter, LongSorter,
    StringSorter,
};
use crate::index::sort_field_provider::SortFieldProvider;
use crate::search::field_comparator_source::FieldComparatorSourceEnum;
use crate::search::sort_field_enum::SortFieldEnum;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::numeric_utils::NumericUtils;
use std::fmt;
use std::fmt::Display;
use std::hash::Hash;

/// Stores information about how to sort documents by terms in an individual field.
/// Fields must be indexed to sort by them.
///
/// Sorting on a numeric field that is indexed with both doc values and points may
/// use an optimization to skip non-competitive documents. This optimization relies
/// on the assumption that the same data is stored in these points and doc values.
///
/// Sorting on a SORTED(_SET) field that is indexed with both doc values and term
/// index may use an optimization to skip non-competitive documents. This optimization
/// relies on the assumption that the same data is stored in these term index and
/// doc values.
#[derive(Clone)]
pub struct SortField {
    fields: Option<String>,
    field_type: SortFieldType,
    comparator_source: Option<FieldComparatorSourceEnum>,
    /// defaults to natural order
    pub(crate) reverse: bool,
    /// Used for 'sortMissingFirst/Last'
    pub(crate) missing_value: Option<MissingValueEnum>,
    /// Indicates if sort should be optimized with indexed data. Set to true by default.
    #[deprecated(since = "10.0.0")]
    #[allow(unused)]
    optimize_sort_with_indexed_data: bool,
}

impl SortField {
    /// Creates a sort by terms in the given field with the type of term values explicitly given.
    ///
    /// # Arguments
    ///
    /// - `Field`: Name of the field to sort by. Can be `None` if `field_type` is `SCORE` or `DOC`.
    /// - `field_type`: Type of values in the terms.
    /// - `sub_sort_field`: Provides additional (or customized) sorting functionality.
    ///   This could be a trait or type that encapsulates more advanced logic.
    ///
    /// # Errors
    ///
    /// Returns an error if the field is `None` and the type is not `SCORE` or `DOC`.
    pub fn new(
        field: Option<String>,
        field_type: SortFieldType,
    ) -> Result<Self> {
        SortField::init_field_type(field, field_type)
    }
    /// Creates a sort, possibly in reverse, by terms in the given field with the type of term values
    /// explicitly given.
    ///
    /// # Arguments
    ///
    /// - `Field`: Name of the field to sort by. Can be `None` if `field_type` is `SCORE` or `DOC`.
    /// - `field_type`: Type of values in the terms.
    /// - `reverse`: `true` if natural order should be reversed.
    /// - `Sub_sort_field`: An additional sorting criterion or a custom implementation that provides
    ///   extended sorting logic. It can be used to define advanced or secondary sorting behavior.
    ///
    /// # Errors
    ///
    /// Returns an error if the `field` is `None` and the `field_type` is not `SCORE` or `DOC`.
    pub fn with_reverse(
        field: Option<String>,
        field_type: SortFieldType,
        reverse: bool,
    ) -> Result<Self> {
        let mut result = Self::new(field, field_type)?;
        result.reverse = reverse;
        Ok(result)
    }
    /// Creates a sort with a custom comparison function and an optional sub-sort field.
    ///
    /// # Arguments
    ///
    /// - `Field`: Name of the field to sort by.
    /// - `comparator`: A source that returns a comparator for sorting hits; cannot be `None`
    /// - `sub_sort_field`: An additional sorting criterion or a custom implementation that provides
    ///   extended sorting logic. It can be used to define advanced or secondary sorting behavior.
    /// # Errors
    ///
    /// Returns an error if the `field` is `None` and the `field_type` is not `SCORE` or `DOC`.
    pub fn with_comparator(
        field: Option<String>,
        comparator: Option<FieldComparatorSourceEnum>,
    ) -> Result<Self> {
        let mut result =
            SortField::init_field_type(field, SortFieldType::Custom)?;
        debug_assert!(comparator.is_some());
        result.comparator_source = comparator;
        Ok(result)
    }
    /// Creates a sort, possibly in reverse, with a custom comparison function and an optional sub-sort field.
    ///
    /// # Arguments
    ///
    /// - `Field`: Name of the field to sort by.
    /// - `comparator`: A source that returns a comparator for sorting hits. cannot be `None`
    /// - `reverse`: `true` if natural order should be reversed.
    /// - `Sub_sort_field`: An additional sorting criterion or a custom implementation that provides
    ///   extended sorting logic. It can be used to define advanced or secondary sorting behavior.
    /// # Errors
    ///
    /// Returns an error if the `field` is `None` and the `field_type` is not `SCORE` or `DOC`.
    pub fn with_comparator_reverse(
        field: Option<String>,
        comparator: Option<FieldComparatorSourceEnum>,
        reverse: bool,
    ) -> Result<Self> {
        let mut result = Self::with_comparator(field, comparator)?;
        result.reverse = reverse;
        Ok(result)
    }
    /// Represents sorting by document score (relevance)
    /// # Note
    /// Replace Java's `SortField.FIELD_SCORE` with this method.
    pub fn get_field_score() -> Result<Self> {
        SortField::new(None, SortFieldType::Score)
    }
    /// Represents sorting by document number (index order).
    /// # Note
    /// Replace Java's `SortField.FIELD_DOC` with this method.
    pub fn get_field_doc() -> Result<Self> {
        SortField::new(None, SortFieldType::Doc)
    }
    // Sets field & type, and ensures field is not NULL unless
    // type is SCORE or DOC
    fn init_field_type(
        field: Option<String>,
        field_type: SortFieldType,
    ) -> Result<Self> {
        if field.is_none()
            && field_type != SortFieldType::Score
            && field_type != SortFieldType::Doc
        {
            return Err(LuceneError::illegal_argument(
                "field can only be None when type is SCORE or DOC".to_string(),
            ));
        }
        Ok(Self {
            fields: field,
            field_type,
            comparator_source: None,
            reverse: false,
            missing_value: None,
            optimize_sort_with_indexed_data: true,
        })
    }
    /// Returns the value to use for documents that don't have a value.
    ///
    /// A value of `None` indicates that the default value should be used.
    pub fn get_missing_value(&self) -> Option<&MissingValueEnum> {
        self.missing_value.as_ref()
    }
    /// Returns the name of the field.
    ///
    /// This could return `None` if the sort is by `SCORE` or `DOC`.
    ///
    /// # Returns
    /// The name of the field, or `None` if the sort is by `SCORE` or `DOC`.
    pub fn get_field(&self) -> Option<&String> {
        self.fields.as_ref()
    }
    /// Returns the type of contents in the field.
    ///
    /// # Returns
    /// One of the constants: `SCORE`, `DOC`, `STRING`, `INT`, or `FLOAT`.
    pub fn get_type(&self) -> &SortFieldType {
        &self.field_type
    }
    /// Returns whether the sort should be reversed.
    ///
    /// # Returns
    /// `true` if natural order should be reversed.
    pub fn get_reverse(&self) -> bool {
        self.reverse
    }
}
impl SortFiledBase for SortField {
    /// Set the value to use for documents that don't have a value.
    fn set_missing_value(
        &mut self,
        missing_value: Option<MissingValueEnum>,
    ) -> Result<()> {
        match self.field_type {
            SortFieldType::String | SortFieldType::StringVal => {
                if let Some(
                    MissingValueEnum::StringFirst
                    | MissingValueEnum::StringLast,
                ) = missing_value
                {
                    self.missing_value = missing_value;
                } else {
                    return Err(LuceneError::illegal_argument(
                        "For STRING type, missing value must be either STRING_FIRST or STRING_LAST"
                            .to_string(),
                    ));
                }
            },
            SortFieldType::Int => {
                if let Some(MissingValueEnum::Int(_)) = missing_value {
                    self.missing_value = missing_value;
                } else {
                    return Err(LuceneError::illegal_argument(
                        "Missing values for Type.INT can only be of type MissingValueEnum::Int"
                            .to_string(),
                    ));
                }
            },
            SortFieldType::Long => {
                if let Some(MissingValueEnum::Long(_)) = missing_value {
                    self.missing_value = missing_value;
                } else {
                    return Err(LuceneError::illegal_argument(
                        "Missing values for Type.LONG can only be of type MissingValueEnum::Long"
                            .to_string(),
                    ));
                }
            },
            SortFieldType::Float => {
                if let Some(MissingValueEnum::Float(_)) = missing_value {
                    self.missing_value = missing_value;
                } else {
                    return Err(LuceneError::illegal_argument(
                        "Missing values for Type.FLOAT can only be of type MissingValueEnum::Float"
                            .to_string(),
                    ));
                }
            },
            SortFieldType::Double => {
                if let Some(MissingValueEnum::Double(_)) = missing_value {
                    self.missing_value = missing_value;
                } else {
                    return Err(LuceneError::illegal_argument("Missing values for Type.DOUBLE can only be of type MissingValueEnum::Double".to_string()));
                }
            },
            _ => {
                return Err(LuceneError::illegal_argument(
                    "Missing value only works for numeric or STRING types"
                        .to_string(),
                ));
            },
        }

        Ok(())
    }
    fn get_index_sorter(&self) -> Option<IndexSortEnum> {
        match self.field_type {
            SortFieldType::Int => Some(IndexSortEnum::IntSorter(IntSorter {
                provider_name: Provider::NAME.to_string(),
            })),
            SortFieldType::Float => {
                Some(IndexSortEnum::FloatSorter(FloatSorter {
                    provider_name: Provider::NAME.to_string(),
                }))
            },
            SortFieldType::Long => {
                Some(IndexSortEnum::LongSorter(LongSorter {
                    provider_name: Provider::NAME.to_string(),
                }))
            },
            SortFieldType::Double => {
                Some(IndexSortEnum::DoubleSorter(DoubleSorter {
                    provider_name: Provider::NAME.to_string(),
                }))
            },
            SortFieldType::String => {
                Some(IndexSortEnum::StringSorter(StringSorter {
                    provider_name: Provider::NAME.to_string(),
                }))
            },
            _ => None,
        }
    }
    fn serialize(&self, out: &mut impl DataOutput) -> Result<()> {
        debug_assert!(self.fields.is_some());
        out.write_string(self.fields.as_ref().unwrap())?;
        out.write_string(&self.field_type.to_string())?;
        out.write_int(if self.reverse { 1 } else { 0 })?;
        if let Some(missing_value) = &self.missing_value {
            out.write_int(1)?;
            match &self.field_type {
                SortFieldType::String => match missing_value {
                    MissingValueEnum::StringLast => out.write_int(0)?,
                    MissingValueEnum::StringFirst => out.write_int(1)?,
                    _ => {
                        return Err(LuceneError::illegal_argument(format!(
                            "Cannot serialize missing value {} for type STRING",
                            missing_value
                        )));
                    },
                },
                SortFieldType::Int => {
                    if let MissingValueEnum::Int(value) = missing_value {
                        out.write_int(*value)?;
                    } else {
                        return Err(LuceneError::illegal_argument(format!(
                            "Invalid missing value {} for type INT",
                            missing_value
                        )));
                    }
                },
                SortFieldType::Long => {
                    if let MissingValueEnum::Long(value) = missing_value {
                        out.write_long(*value)?;
                    } else {
                        return Err(LuceneError::illegal_argument(format!(
                            "Invalid missing value {} for type LONG",
                            missing_value
                        )));
                    }
                },
                SortFieldType::Float => {
                    if let MissingValueEnum::Float(value) = missing_value {
                        out.write_int(NumericUtils::float_to_sortable_int(
                            *value,
                        ))?;
                    } else {
                        return Err(LuceneError::illegal_argument(format!(
                            "Invalid missing value {} for type FLOAT",
                            missing_value
                        )));
                    }
                },
                SortFieldType::Double => {
                    if let MissingValueEnum::Double(value) = missing_value {
                        out.write_long(NumericUtils::double_to_sortable_long(
                            *value,
                        ))?;
                    } else {
                        return Err(LuceneError::illegal_argument(format!(
                            "Invalid missing value {} for type DOUBLE",
                            missing_value
                        )));
                    }
                },
                SortFieldType::Custom
                | SortFieldType::Doc
                | SortFieldType::Rewritable
                | SortFieldType::StringVal
                | SortFieldType::Score => {
                    return Err(LuceneError::illegal_argument(format!(
                        "Cannot serialize SortField of type {:?}",
                        self.field_type
                    )));
                },
            }
        } else {
            out.write_int(0)?;
        }

        Ok(())
    }
}
impl Display for SortField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = String::new();
        match self.field_type {
            SortFieldType::Score => buffer.push_str("<score>"),
            SortFieldType::Doc => buffer.push_str("<doc>"),
            SortFieldType::String => {
                buffer.push_str("<string: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Int => {
                buffer.push_str("<int: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Long => {
                buffer.push_str("<long: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Float => {
                buffer.push_str("<float: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Double => {
                buffer.push_str("<double: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Custom => {
                buffer.push_str("<custom: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\": ");
                if let Some(ref comparator) = self.comparator_source {
                    buffer.push_str(&format!("{}", comparator));
                }
                buffer.push('>');
            },
            SortFieldType::StringVal => {
                buffer.push_str("<string_val: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
            SortFieldType::Rewritable => {
                buffer.push_str("<rewriteable: \"");
                if let Some(ref field) = self.fields {
                    buffer.push_str(field);
                }
                buffer.push_str("\">");
            },
        }
        if self.reverse {
            buffer.push('!');
        }
        if let Some(ref missing_value) = self.missing_value {
            buffer.push_str(" missingValue=");
            buffer.push_str(&format!("{}", missing_value));
        }
        write!(f, "{}", buffer)
    }
}
impl PartialEq for SortField {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
            && self.field_type == other.field_type
            && self.comparator_source == other.comparator_source
            && self.reverse == other.reverse
            && self.missing_value == other.missing_value
    }
}
impl Eq for SortField {}
impl Hash for SortField {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fields.hash(state);
        self.field_type.hash(state);
        self.reverse.hash(state);
        self.comparator_source.hash(state);
        self.missing_value.hash(state);
    }
}

pub struct Provider;
impl Provider {
    /// The name this Provider is registered under.
    pub const NAME: &'static str = "SortField";
}
impl SortFieldProvider for Provider {
    fn read_sort_field(
        &self,
        data_input: &mut impl DataInput,
    ) -> Result<SortFieldEnum> {
        let field_name = data_input.read_string()?;
        let field_type = SortFieldType::read_type(data_input)?;
        let reverse = data_input.read_int()? == 1;
        let mut sort_field =
            SortField::with_reverse(Some(field_name), field_type, reverse)?;
        if data_input.read_int()? == 1 {
            match sort_field.field_type {
                SortFieldType::String => {
                    let missing_string = data_input.read_int()?;
                    match missing_string {
                        1 => sort_field.set_missing_value(Some(
                            MissingValueEnum::StringFirst,
                        ))?,
                        _ => sort_field.set_missing_value(Some(
                            MissingValueEnum::StringLast,
                        ))?,
                    }
                },
                SortFieldType::Int => {
                    let value = data_input.read_int()?;
                    sort_field.set_missing_value(Some(
                        MissingValueEnum::Int(value),
                    ))?;
                },
                SortFieldType::Long => {
                    let value = data_input.read_long()?;
                    sort_field.set_missing_value(Some(
                        MissingValueEnum::Long(value),
                    ))?;
                },
                SortFieldType::Float => {
                    let value = NumericUtils::sortable_int_to_float(
                        data_input.read_int()?,
                    );
                    sort_field.set_missing_value(Some(
                        MissingValueEnum::Float(value),
                    ))?;
                },
                SortFieldType::Double => {
                    let value = NumericUtils::sortable_long_to_double(
                        data_input.read_long()?,
                    );
                    sort_field.set_missing_value(Some(
                        MissingValueEnum::Double(value),
                    ))?;
                },
                SortFieldType::Custom
                | SortFieldType::Doc
                | SortFieldType::Rewritable
                | SortFieldType::StringVal
                | SortFieldType::Score => {
                    return Err(LuceneError::illegal_argument(format!(
                        "Cannot deserialize sort of type {:?}",
                        sort_field.field_type
                    )));
                },
            }
        }

        Ok(SortFieldEnum::Sorter(sort_field))
    }

    fn write_sort_field(
        &self,
        sf: &SortFieldEnum,
        output: &mut impl DataOutput,
    ) -> Result<()> {
        sf.serialize(output)
    }
}

/// Specifies the type of the terms to be sorted, or special types such as `CUSTOM`.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum SortFieldType {
    /// Sort by document score (relevance). Sort values are `f32` and higher values are at the front.
    Score,

    /// Sort by document number (index order). Sort values are `i32` and lower values are at the front.
    Doc,

    /// Sort using term values as `String`. Sort values are `String` and lower values are at the front.
    String,

    /// Sort using term values as encoded `i32`. Sort values are `i32` and lower values are at the front.
    /// Fields must either be not indexed or indexed with `IntPoint`.
    Int,

    /// Sort using term values as encoded `f32`. Sort values are `f32` and lower values are at the front.
    /// Fields must either be not indexed or indexed with `FloatPoint`.
    Float,

    /// Sort using term values as encoded `i64`. Sort values are `i64` and lower values are at the front.
    /// Fields must either be not indexed or indexed with `LongPoint`.
    Long,

    /// Sort using term values as encoded `f64`. Sort values are `f64` and lower values are at the front.
    /// Fields must either be not indexed or indexed with `DoublePoint`.
    Double,

    /// Sort using a custom comparator. Sort values are any `Comparable` and sorting is done according
    /// to natural order.
    Custom,

    /// Sort using term values as `String`, but comparing by value (using `String::cmp`) for all comparisons.
    /// This is typically slower than `STRING`, which uses ordinals to do the sorting.
    StringVal,

    /// Force rewriting of `SortField` using `SortField::rewrite` before it can be used for sorting.
    Rewritable,
}
impl SortFieldType {
    pub fn value_of(type_str: &str) -> Result<Self> {
        match type_str {
            "Score" => Ok(SortFieldType::Score),
            "Doc" => Ok(SortFieldType::Doc),
            "String" => Ok(SortFieldType::String),
            "Int" => Ok(SortFieldType::Int),
            "Float" => Ok(SortFieldType::Float),
            "Long" => Ok(SortFieldType::Long),
            "Double" => Ok(SortFieldType::Double),
            "Custom" => Ok(SortFieldType::Custom),
            "StringVal" => Ok(SortFieldType::StringVal),
            "Rewritable" => Ok(SortFieldType::Rewritable),
            _ => Err(LuceneError::illegal_argument(format!(
                "Can't deserialize SortField - unknown type {}",
                type_str
            ))),
        }
    }
    pub fn read_type<D>(input: &mut D) -> Result<Self>
    where
        D: DataInput,
    {
        let type_str = input.read_string()?;
        SortFieldType::value_of(&type_str)
    }
}
impl Display for SortFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortFieldType::Score => write!(f, "Score"),
            SortFieldType::Doc => write!(f, "Doc"),
            SortFieldType::String => write!(f, "String"),
            SortFieldType::Int => write!(f, "Int"),
            SortFieldType::Float => write!(f, "Float"),
            SortFieldType::Long => write!(f, "Long"),
            SortFieldType::Double => write!(f, "Double"),
            SortFieldType::Custom => write!(f, "Custom"),
            SortFieldType::StringVal => write!(f, "StringVal"),
            SortFieldType::Rewritable => write!(f, "Rewritable"),
        }
    }
}

#[derive(Clone)]
pub enum MissingValueEnum {
    /// Pass this to `setMissingValue` to have missing string values sort first. */
    StringFirst,
    /// Pass this to `setMissingValue` to have missing string values sort last. */
    StringLast,
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
}

impl PartialEq<Self> for MissingValueEnum {
    fn eq(&self, other: &Self) -> bool {
        match self {
            MissingValueEnum::StringFirst => {
                matches!(other, MissingValueEnum::StringFirst)
            },
            MissingValueEnum::StringLast => {
                matches!(other, MissingValueEnum::StringLast)
            },
            MissingValueEnum::Int(val) => {
                if let MissingValueEnum::Int(other_val) = other {
                    *val == *other_val
                } else {
                    false
                }
            },
            MissingValueEnum::Long(val) => {
                if let MissingValueEnum::Long(other_val) = other {
                    *val == *other_val
                } else {
                    false
                }
            },
            MissingValueEnum::Float(val) => {
                if let MissingValueEnum::Float(other_val) = other {
                    // In Rust Lucene,
                    // negative Float::NAN and positive Float::NAN are considered the smallest and largest floating-point values,
                    // respectively.
                    // However, we need to stay consistent with Java Lucene,
                    // where Float::NAN, regardless of its sign,
                    // is always treated as the largest floating-point value.
                    NumericUtils::float_to_sortable_int(*val)
                        == NumericUtils::float_to_sortable_int(*other_val)
                } else {
                    false
                }
            },
            MissingValueEnum::Double(val) => {
                if let MissingValueEnum::Double(other_val) = other {
                    // In Rust Lucene,
                    // negative Double::NAN and positive Double::NAN are considered the smallest and largest floating-point values,
                    // respectively.
                    // However, we need to stay consistent with Java Lucene,
                    // where Double::NAN, regardless of its sign,
                    // is always treated as the largest floating-point value.
                    NumericUtils::double_to_sortable_long(*val)
                        == NumericUtils::double_to_sortable_long(*other_val)
                } else {
                    false
                }
            },
        }
    }
}

impl Eq for MissingValueEnum {}

impl Display for MissingValueEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MissingValueEnum::StringFirst => {
                write!(f, "SortField.STRING_FIRST")
            },
            MissingValueEnum::StringLast => write!(f, "SortField.STRING_LAST"),
            MissingValueEnum::Int(val) => write!(f, "SortField.INT({})", val),
            MissingValueEnum::Long(val) => write!(f, "SortField.LONG({})", val),
            MissingValueEnum::Float(val) => {
                write!(f, "SortField.FLOAT({})", val)
            },
            MissingValueEnum::Double(val) => {
                write!(f, "SortField.DOUBLE({})", val)
            },
        }
    }
}
impl Hash for MissingValueEnum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            MissingValueEnum::StringFirst => {
                "SortField.STRING_FIRST".hash(state)
            },
            MissingValueEnum::StringLast => "SortField.STRING_LAST".hash(state),
            MissingValueEnum::Int(val) => {
                "SortField.INT".hash(state);
                val.hash(state);
            },
            MissingValueEnum::Long(val) => {
                "SortField.LONG".hash(state);
                val.hash(state);
            },
            MissingValueEnum::Float(val) => {
                "SortField.FLOAT".hash(state);
                NumericUtils::float_to_sortable_int(*val).hash(state);
            },
            MissingValueEnum::Double(val) => {
                "SortField.DOUBLE".hash(state);
                NumericUtils::double_to_sortable_long(*val).hash(state);
            },
        }
    }
}

pub trait SortFiledBase {
    /// Set the value to use for documents that don't have a value.
    fn set_missing_value(
        &mut self,
        missing_value: Option<MissingValueEnum>,
    ) -> Result<()>;
    fn get_index_sorter(&self) -> Option<IndexSortEnum>;
    fn serialize(&self, out: &mut impl DataOutput) -> Result<()>;
}
