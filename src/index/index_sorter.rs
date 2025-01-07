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
pub trait IndexSorter {
    fn get_provider_name(&self) -> &str;
}

/// Sorts documents based on double values from a NumericDocValues instance.
pub struct DoubleSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for DoubleSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on integer values from a NumericDocValues instance */
pub struct IntSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for IntSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on long values from a NumericDocValues instance
pub struct LongSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for LongSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on float values from a NumericDocValues instance
pub struct FloatSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for FloatSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

/// Sorts documents based on short values from a NumericDocValues instance
pub struct StringSorter {
    pub(crate) provider_name: String,
}
impl IndexSorter for StringSorter {
    fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

pub enum IndexSortEnum {
    DSorter(DoubleSorter),
    ISorter(IntSorter),
    LSorter(LongSorter),
    FSorter(FloatSorter),
    SSorter(StringSorter),
}
impl IndexSorter for IndexSortEnum {
    fn get_provider_name(&self) -> &str {
        match self {
            IndexSortEnum::DSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::ISorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::LSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::FSorter(sorter) => sorter.get_provider_name(),
            IndexSortEnum::SSorter(sorter) => sorter.get_provider_name(),
        }
    }
}
