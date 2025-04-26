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
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::util::automation::IntSet::IntSet;

pub(crate) struct FrozenIntSet {
    values: Rc<Vec<i32>>,
    state: i32,
    hash_code: i64,
}

impl FrozenIntSet {
    pub(crate) fn new(values: Rc<Vec<i32>>, hash_code: i64, state: i32) -> Self {
        FrozenIntSet {
            values,
            hash_code,
            state,
        }
    }
}

impl PartialEq for FrozenIntSet {
    fn eq(&self, other: &Self) -> bool {
        self.hash_code == other.hash_code && *self.values == *other.values
    }
}
impl IntSet for FrozenIntSet {
    fn get_array(&mut self) -> &Rc<Vec<i32>> {
        &self.values
    }

    fn size(&self) -> usize {
        self.values.len()
    }

    fn long_hash_code(&mut self) -> i64 {
        self.hash_code
    }
}

impl fmt::Display for FrozenIntSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.values)
    }
}
impl Hash for FrozenIntSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_code.hash(state);
    }
}
