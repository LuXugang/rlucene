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
use crate::index::index_sorter::{
    DoubleSorter, FloatSorter, IndexSortEnum, IntSorter, LongSorter, StringSorter,
};
use crate::index::sort_field_provider::SortFieldProvider;
use crate::search::sort_field::{
    MissingValueEnum, SortField, SortFieldEnum, SortFieldType, SortFiledBase,
};
use crate::search::sorted_numeric_selector::SortedNumericSelectorType;
use crate::store::{DataInput, DataOutput};
use crate::util::error::lucene_error::LuceneError;
use crate::util::numeric_utils::NumericUtils;
use std::fmt::Display;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct SortedNumericSortField {
    sort_field_type: SortFieldType,
    selector: SortedNumericSelectorType,
    sort_field: SortField,
}
impl SortedNumericSortField {
    /// Creates a sort by the minimum value in the set for the document.
    ///
    /// # Arguments
    ///
    /// * `field` - Name of the field to sort by. Must not be empty.
    /// * `sort_field_type` - Type of values.
    pub fn new(field: String, sort_field_type: SortFieldType) -> Result<Self, LuceneError> {
        Self::with_reverse(field, sort_field_type, false)
    }

    /// Creates a sort, possibly in reverse, by the minimum value in the set for the document.
    ///
    /// # Arguments
    ///
    /// * `field` - Name of the field to sort by. Must not be empty.
    /// * `sort_field_type` - Type of values.
    /// * `reverse` - `true` if natural order should be reversed.
    pub fn with_reverse(
        field: String,
        sort_field_type: SortFieldType,
        reverse: bool,
    ) -> Result<Self, LuceneError> {
        Self::with_selector(
            field,
            sort_field_type,
            reverse,
            SortedNumericSelectorType::Min,
        )
    }
    /// Creates a sort, possibly in reverse, specifying how the sort value from the document's set is selected.
    ///
    /// # Arguments
    ///
    /// * `field` - Name of the field to sort by.
    /// * `sort_field_type` - Type of values.
    /// * `reverse` - `true` if natural order should be reversed.
    /// * `selector` - Custom selector type for choosing the sort value from the set.
    pub fn with_selector(
        field: String,
        sort_field_type: SortFieldType,
        reverse: bool,
        selector: SortedNumericSelectorType,
    ) -> Result<Self, LuceneError> {
        let sort_field =
            SortField::with_reverse(Some(field.clone()), SortFieldType::Custom, reverse)?;
        Ok(SortedNumericSortField {
            sort_field_type,
            selector,
            sort_field,
        })
    }
    pub fn read_selector_type<T: DataInput>(
        data_input: &mut T,
    ) -> Result<SortedNumericSelectorType, LuceneError> {
        let selector_type = data_input.read_int()?;

        match selector_type {
            0 => Ok(SortedNumericSelectorType::Min),
            1 => Ok(SortedNumericSelectorType::Max),
            _ => Err(LuceneError::illegal_argument(format!(
                "Cannot deserialize SortedNumericSortField - unknown selector type: {}",
                selector_type
            ))),
        }
    }
}

impl SortFiledBase for SortedNumericSortField {
    fn set_missing_value(
        &mut self,
        missing_value: Option<MissingValueEnum>,
    ) -> Result<(), LuceneError> {
        self.sort_field.missing_value = missing_value;
        Ok(())
    }

    fn get_index_sorter(&self) -> Option<IndexSortEnum> {
        match self.sort_field_type {
            SortFieldType::Int => Some(IndexSortEnum::IntSorter(IntSorter {
                provider_name: NumericProvider::NAME.to_string(),
            })),
            SortFieldType::Float => Some(IndexSortEnum::FloatSorter(FloatSorter {
                provider_name: NumericProvider::NAME.to_string(),
            })),
            SortFieldType::Long => Some(IndexSortEnum::LongSorter(LongSorter {
                provider_name: NumericProvider::NAME.to_string(),
            })),
            SortFieldType::Double => Some(IndexSortEnum::DoubleSorter(DoubleSorter {
                provider_name: NumericProvider::NAME.to_string(),
            })),
            SortFieldType::String => Some(IndexSortEnum::StringSorter(StringSorter {
                provider_name: NumericProvider::NAME.to_string(),
            })),
            _ => None,
        }
    }

    fn serialize<T: DataOutput>(&self, out: &mut T) -> Result<(), LuceneError> {
        debug_assert!(self.sort_field.get_field().is_some());
        out.write_string(self.sort_field.get_field().unwrap())?;
        out.write_string(&self.sort_field_type.to_string())?;
        out.write_int(if self.sort_field.reverse { 1 } else { 0 })?;
        out.write_int(self.selector as i32)?;
        if let Some(missing_value) = &self.sort_field.missing_value {
            out.write_int(1)?;
            match self.sort_field_type {
                SortFieldType::Int => {
                    if let MissingValueEnum::Int(value) = missing_value {
                        out.write_int(*value)?;
                    } else {
                        return Err(LuceneError::illegal_state(
                            "Missing value type mismatch for INT.".to_string(),
                        ));
                    }
                }
                SortFieldType::Long => {
                    if let MissingValueEnum::Long(value) = missing_value {
                        out.write_long(*value)?;
                    } else {
                        return Err(LuceneError::illegal_state(
                            "Missing value type mismatch for LONG.".to_string(),
                        ));
                    }
                }
                SortFieldType::Float => {
                    if let MissingValueEnum::Float(value) = missing_value {
                        out.write_int(NumericUtils::float_to_sortable_int(*value))?;
                    } else {
                        return Err(LuceneError::illegal_state(
                            "Missing value type mismatch for FLOAT.".to_string(),
                        ));
                    }
                }
                SortFieldType::Double => {
                    if let MissingValueEnum::Double(value) = missing_value {
                        out.write_long(NumericUtils::double_to_sortable_long(*value))?;
                    } else {
                        return Err(LuceneError::illegal_state(
                            "Missing value type mismatch for DOUBLE.".to_string(),
                        ));
                    }
                }
                SortFieldType::Custom
                | SortFieldType::Doc
                | SortFieldType::Rewritable
                | SortFieldType::StringVal
                | SortFieldType::Score
                | SortFieldType::String => {
                    return Err(LuceneError::illegal_state(format!(
                        "Cannot serialize field of type {:?}.",
                        self.sort_field_type
                    )));
                }
            }
        } else {
            out.write_int(0)?;
        }

        Ok(())
    }
}
impl Display for SortedNumericSortField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = String::new();
        debug_assert!(self.sort_field.get_field().is_some());
        buffer.push_str(&format!(
            "<sortednumeric: \"{}\">",
            self.sort_field.get_field().unwrap()
        ));
        if self.sort_field.reverse {
            buffer.push('!');
        }
        if let Some(missing_value) = &self.sort_field.missing_value {
            buffer.push_str(&format!(" missingValue={}", missing_value));
        }
        buffer.push_str(&format!(" selector={:?}", self.selector));
        buffer.push_str(&format!(" type={:?}", self.sort_field_type));
        write!(f, "{}", buffer)
    }
}
impl Hash for SortedNumericSortField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sort_field_type.hash(state);
        self.selector.hash(state);
        self.sort_field.hash(state);
    }
}

pub struct NumericProvider;
impl NumericProvider {
    /// The name this Provider is registered under.
    pub const NAME: &'static str = "SortedNumericSortField";
}
impl SortFieldProvider for NumericProvider {
    fn read_sort_field<D>(&self, data_input: &mut D) -> Result<SortFieldEnum, LuceneError>
    where
        D: DataInput,
    {
        let field_name = data_input.read_string()?;
        let field_type = SortFieldType::read_type(data_input)?;
        let reverse = data_input.read_int()? == 1;
        let selector = SortedNumericSortField::read_selector_type(data_input)?;
        let mut sorted_numeric_sort_field =
            SortedNumericSortField::with_selector(field_name, field_type, reverse, selector)?;
        let value = data_input.read_int()?;
        if value == 1 {
            match field_type {
                SortFieldType::Int => {
                    let missing_value = data_input.read_int()?;
                    sorted_numeric_sort_field
                        .set_missing_value(Some(MissingValueEnum::Int(missing_value)))?;
                }
                SortFieldType::Long => {
                    let missing_value = data_input.read_long()?;
                    sorted_numeric_sort_field
                        .set_missing_value(Some(MissingValueEnum::Long(missing_value)))?;
                }
                SortFieldType::Float => {
                    let missing_value = NumericUtils::sortable_int_to_float(data_input.read_int()?);
                    sorted_numeric_sort_field
                        .set_missing_value(Some(MissingValueEnum::Float(missing_value)))?;
                }
                SortFieldType::Double => {
                    let missing_value =
                        NumericUtils::sortable_long_to_double(data_input.read_long()?);
                    sorted_numeric_sort_field
                        .set_missing_value(Some(MissingValueEnum::Double(missing_value)))?;
                }
                SortFieldType::Custom
                | SortFieldType::Doc
                | SortFieldType::Rewritable
                | SortFieldType::StringVal
                | SortFieldType::Score
                | SortFieldType::String => {
                    return Err(LuceneError::illegal_state(format!(
                        "Cannot deserialize sort of type {:?}",
                        field_type
                    )));
                }
            }
        } else {
            debug_assert!(value == 0);
        }
        Ok(SortFieldEnum::SortedNumeric(sorted_numeric_sort_field))
    }

    fn write_sort_field<D>(&self, sf: &SortFieldEnum, output: &mut D) -> Result<(), LuceneError>
    where
        D: DataOutput,
    {
        sf.serialize(output)
    }
}
impl PartialEq for SortedNumericSortField {
    fn eq(&self, other: &Self) -> bool {
        if self.sort_field != other.sort_field {
            return false;
        }
        self.selector == other.selector && self.sort_field_type == other.sort_field_type
    }
}
impl Eq for SortedNumericSortField {}
