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
use std::fmt;
use std::rc::Rc;

use crate::index::BytesRef;
use crate::util::error::lucene_error::{LuceneError, Result};

/// Abstraction around a stored value.
///
/// See: [`IndexableField`](crate::index::indexable_field::IndexableField)
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
    Binary(Rc<BytesRef<Vec<u8>>>),
    /// Type of string values.
    String(Rc<String>),
}

/// Type of a [`StoredValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredValueType {
    /// Type of integer values.
    Integer,
    /// Type of long values.
    Long,
    /// Type of float values.
    Float,
    /// Type of double values.
    Double,
    /// Type of binary values.
    Binary,
    /// Type of string values.
    String,
}

impl fmt::Display for StoredValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self {
            StoredValueType::Integer => "INTEGER",
            StoredValueType::Long => "LONG",
            StoredValueType::Float => "FLOAT",
            StoredValueType::Double => "DOUBLE",
            StoredValueType::Binary => "BINARY",
            StoredValueType::String => "STRING",
        };
        write!(f, "{type_str}")
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
    pub fn new_binary(value: Rc<BytesRef<Vec<u8>>>) -> Self {
        StoredValue::Binary(value)
    }

    /// Ctor for string values.
    pub fn new_string(value: Rc<String>) -> Self {
        StoredValue::String(value)
    }

    /// Retrieve the type of the stored value.
    pub fn get_type(&self) -> StoredValueType {
        match self {
            StoredValue::Integer(_) => StoredValueType::Integer,
            StoredValue::Long(_) => StoredValueType::Long,
            StoredValue::Float(_) => StoredValueType::Float,
            StoredValue::Double(_) => StoredValueType::Double,
            StoredValue::Binary(_) => StoredValueType::Binary,
            StoredValue::String(_) => StoredValueType::String,
        }
    }

    /// Set an integer value.
    pub fn set_int_value(&mut self, value: i32) -> Result<()> {
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
    pub fn set_long_value(&mut self, value: i64) -> Result<()> {
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
    pub fn set_float_value(&mut self, value: f32) -> Result<()> {
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
    pub fn set_double_value(&mut self, value: f64) -> Result<()> {
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
    pub fn set_binary_value(&mut self, value: Rc<BytesRef<Vec<u8>>>) -> Result<()> {
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
    pub fn set_string_value(&mut self, value: Rc<String>) -> Result<()> {
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
    pub fn get_int_value(&self) -> Result<i32> {
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
    pub fn get_long_value(&self) -> Result<i64> {
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
    pub fn get_float_value(&self) -> Result<f32> {
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
    pub fn get_double_value(&self) -> Result<f64> {
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
    pub fn get_binary_value(&self) -> Result<&Rc<BytesRef<Vec<u8>>>> {
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
    pub fn get_string_value(&self) -> Result<&String> {
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
