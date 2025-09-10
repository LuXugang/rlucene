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
use std::borrow::Cow;
use std::collections::HashMap;
/// A simple class that stores key Strings as char[]'s in a hash table
pub struct CharArrayMap<T> {
    ignore: bool,
    map: HashMap<Vec<char>, T>,
}
impl<T> CharArrayMap<T> {
    pub fn new(ignore: bool) -> Self {
        CharArrayMap {
            ignore,
            map: HashMap::new(),
        }
    }
    pub fn extend(other: HashMap<Vec<char>, T>, ignore: bool) -> Self {
        let mut v = Self::new(ignore);
        v.map.extend(other);
        v
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }
    pub fn contains_key(&self, key: &[char], off: i32, len: i32) -> bool {
        let slice = &key[off as usize..(off + len) as usize];
        let key = Self::norm(self.ignore, slice);
        self.map.contains_key(&*key)
    }
    pub fn contains_key_str(&self, key: &str) -> bool {
        let chars: Vec<char> = key.chars().collect();
        let norm = Self::norm(self.ignore, &chars);
        self.map.contains_key(&*norm)
    }

    pub fn put(&mut self, key: &[char], val: T) {
        let k = if self.ignore {
            key.iter().flat_map(|c| c.to_lowercase()).collect()
        } else {
            key.to_vec()
        };
        self.map.insert(k, val);
    }
    pub fn put_str(&mut self, key: &str, val: T) {
        let chars: Vec<char> = key.chars().collect();
        let k = match Self::norm(self.ignore, &chars) {
            Cow::Borrowed(b) => b.to_vec(),
            Cow::Owned(o) => o,
        };
        self.map.insert(k, val);
    }
    pub fn get(&self, key: &[char]) -> Option<&T> {
        let key = Self::norm(self.ignore, key);
        self.map.get(&*key)
    }
    pub fn get_str(&self, key: &str) -> Option<&T> {
        let chars: Vec<char> = key.chars().collect();
        let key = Self::norm(self.ignore, &chars);
        self.map.get(&*key)
    }

    fn norm(ignore: bool, s: &[char]) -> Cow<'_, [char]> {
        if ignore {
            Cow::Owned(s.iter().flat_map(|c| c.to_lowercase()).collect())
        } else {
            Cow::Borrowed(s)
        }
    }
}
