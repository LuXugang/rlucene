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

pub trait Comparator<T> {
    fn compare(&self, a: &T, b: &T) -> i32;
}

pub struct NaturalOrder<T>
where
    T: Ord,
{
    _t: std::marker::PhantomData<T>,
}

impl<T> Default for NaturalOrder<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NaturalOrder<T>
where
    T: Ord,
{
    pub fn new() -> NaturalOrder<T> {
        NaturalOrder {
            _t: std::marker::PhantomData,
        }
    }
}
impl<T> Comparator<T> for NaturalOrder<T>
where
    T: Ord,
{
    fn compare(&self, a: &T, b: &T) -> i32 {
        let result = a.cmp(b);
        match result {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }
}
