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
use crate::core::analysis::char_array_map::{CharArrayMap, empty_map};
use std::fmt::Display;

/// A set that stores strings as character arrays in a hash table.
pub struct CharArraySet {
  map: CharArrayMap<()>,
}
impl CharArraySet {
  pub fn new(ignore: bool) -> Self {
    CharArraySet {
      map: CharArrayMap::new(ignore),
    }
  }
  pub fn from_map(map: CharArrayMap<()>) -> CharArraySet {
    CharArraySet { map }
  }
  pub fn empty_set() -> CharArraySet {
    Self::from_map(empty_map())
  }
  pub fn add_all<I, S>(&mut self, iter: I)
  where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
  {
    for s in iter {
      self.add_str(s.as_ref());
    }
  }
  pub fn clear(&mut self) {
    self.map.clear();
  }
  pub fn contains_key(&self, key: &[char], off: i32, len: i32) -> bool {
    self.map.contains_key(key, off, len)
  }
  pub fn contains_key_str(&self, key: &str) -> bool {
    self.map.contains_key_str(key)
  }
  pub fn contains_key_any<T>(&self, key: &T) -> bool
  where
    T: Display,
  {
    self.map.contains_key_any(key)
  }

  pub fn add(&mut self, key: &[char]) -> Option<()> {
    self.map.put(key, ())
  }
  pub fn add_str(&mut self, key: &str) -> Option<()> {
    self.map.put_str(key, ())
  }
  pub fn add_any<T>(&mut self, key: &T) -> Option<()>
  where
    T: Display,
  {
    self.map.put_any(key, ())
  }
  pub fn get(&self, key: &[char]) -> Option<&()> {
    debug_assert!(key.len() <= i32::MAX as usize);
    self.map.get(key, 0, key.len() as i32)
  }
  pub fn get_str(&self, key: &str) -> Option<&()> {
    self.map.get_str(key)
  }
  pub fn get_any<T>(&self, key: &T) -> Option<&()>
  where
    T: Display,
  {
    self.map.get_any(key)
  }
  pub fn size(&self) -> usize {
    self.map.size()
  }
}
