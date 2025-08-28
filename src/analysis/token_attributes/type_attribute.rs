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
use crate::util::attribute::Attribute;

/// A Token’s lexical type. The default value is `"word"`.
pub trait TypeAttribute: Attribute {
    /// Returns this Token’s lexical type. Defaults to `"word"`.
    ///
    /// # See
    ///
    /// [`set_type`](TypeAttribute::set_type)
    fn type_value(&self) -> &str;

    /// Set the lexical type.
    ///
    /// # See
    ///
    /// [`type`](TypeAttribute::type_value)
    fn set_type(&mut self, type_: &str);
}

/// The default type.
pub const DEFAULT_TYPE: &str = "word";
