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
use crate::index::BytesRef;
use crate::util::error::lucene_error::LuceneError;
use std::fmt;
use std::sync::Arc;

/// Abstraction around a stored value.
///
/// See: [`IndexableField`]
#[derive(Debug, Clone)]
pub enum StoredValue {
    /// Type of integer values.
    Integer(i32),
    /// Type of long values.
    Long(i64),
    /// Type of float values.
    Float(f32),
    /// Type of double values.
    Double(f64),
    /// Type of binary values.
    Binary(Arc<BytesRef>),
    /// Type of string values.
    String(Arc<String>),
}

/// Type of a [`StoredValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredValueType {
    /// Type of integer values.
    INTEGER,
    /// Type of long values.
    LONG,
    /// Type of float values.
    FLOAT,
    /// Type of double values.
    DOUBLE,
    /// Type of binary values.
    BINARY,
    /// Type of string values.
    STRING,
}

impl fmt::Display for StoredValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self {
            StoredValueType::INTEGER => "INTEGER",
            StoredValueType::LONG => "LONG",
            StoredValueType::FLOAT => "FLOAT",
            StoredValueType::DOUBLE => "DOUBLE",
            StoredValueType::BINARY => "BINARY",
            StoredValueType::STRING => "STRING",
        };
        write!(f, "{}", type_str)
    }
}

impl StoredValue {
    /// Ctor for integer values.
    pub fn new_integer(value: i32) -> Self {
        StoredValue::Integer(value)
    }

    /// Ctor for long values.
    pub fn new_long(value: i64) -> Self {
        StoredValue::Long(value)
    }

    /// Ctor for float values.
    pub fn new_float(value: f32) -> Self {
        StoredValue::Float(value)
    }

    /// Ctor for double values.
    pub fn new_double(value: f64) -> Self {
        StoredValue::Double(value)
    }

    /// Ctor for binary values.
    pub fn new_binary(value: Arc<BytesRef>) -> Self {
        StoredValue::Binary(value)
    }

    /// Ctor for string values.
    pub fn new_string(value: Arc<String>) -> Self {
        StoredValue::String(value)
    }

    /// Retrieve the type of the stored value.
    pub fn get_type(&self) -> StoredValueType {
        match self {
            StoredValue::Integer(_) => StoredValueType::INTEGER,
            StoredValue::Long(_) => StoredValueType::LONG,
            StoredValue::Float(_) => StoredValueType::FLOAT,
            StoredValue::Double(_) => StoredValueType::DOUBLE,
            StoredValue::Binary(_) => StoredValueType::BINARY,
            StoredValue::String(_) => StoredValueType::STRING,
        }
    }

    /// Set an integer value.
    pub fn set_int_value(&mut self, value: i32) -> Result<(), LuceneError> {
        if let StoredValue::Integer(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set an integer on a {} value",
                self.get_type()
            )))
        }
    }

    /// Set a long value.
    pub fn set_long_value(&mut self, value: i64) -> Result<(), LuceneError> {
        if let StoredValue::Long(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set a long on a {} value",
                self.get_type()
            )))
        }
    }

    /// Set a float value.
    pub fn set_float_value(&mut self, value: f32) -> Result<(), LuceneError> {
        if let StoredValue::Float(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set a float on a {} value",
                self.get_type()
            )))
        }
    }

    /// Set a double value.
    pub fn set_double_value(&mut self, value: f64) -> Result<(), LuceneError> {
        if let StoredValue::Double(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set a double on a {} value",
                self.get_type()
            )))
        }
    }

    /// Set a binary value.
    pub fn set_binary_value(&mut self, value: Arc<BytesRef>) -> Result<(), LuceneError> {
        if let StoredValue::Binary(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set a binary value on a {} value",
                self.get_type()
            )))
        }
    }

    /// Set a string value.
    pub fn set_string_value(&mut self, value: Arc<String>) -> Result<(), LuceneError> {
        if let StoredValue::String(ref mut v) = self {
            *v = value;
            Ok(())
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot set a string value on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve an integer value.
    pub fn get_int_value(&self) -> Result<i32, LuceneError> {
        if let StoredValue::Integer(v) = self {
            Ok(*v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get an integer on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve a long value.
    pub fn get_long_value(&self) -> Result<i64, LuceneError> {
        if let StoredValue::Long(v) = self {
            Ok(*v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get a long on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve a float value.
    pub fn get_float_value(&self) -> Result<f32, LuceneError> {
        if let StoredValue::Float(v) = self {
            Ok(*v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get a float on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve a double value.
    pub fn get_double_value(&self) -> Result<f64, LuceneError> {
        if let StoredValue::Double(v) = self {
            Ok(*v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get a double on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve a binary value.
    pub fn get_binary_value(&self) -> Result<&BytesRef, LuceneError> {
        if let StoredValue::Binary(ref v) = self {
            Ok(v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get a binary value on a {} value",
                self.get_type()
            )))
        }
    }

    /// Retrieve a string value.
    pub fn get_string_value(&self) -> Result<&String, LuceneError> {
        if let StoredValue::String(ref v) = self {
            Ok(v)
        } else {
            Err(LuceneError::illegal_argument(format!(
                "Cannot get a string value on a {} value",
                self.get_type()
            )))
        }
    }
}
