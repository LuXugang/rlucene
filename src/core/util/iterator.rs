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
use crate::core::util::error::lucene_error::Result;
pub trait IteratorExt {
  type Item;
  fn next(&mut self) -> Result<Option<Self::Item>>;
  fn has_next(&self) -> Result<bool> {
    Ok(true)
  }
}

pub struct VecIter<'a, T> {
  data: &'a Vec<T>,
  pos: usize,
}

impl<'a, T> IteratorExt for VecIter<'a, T> {
  type Item = &'a T;

  fn next(&mut self) -> Result<Option<Self::Item>> {
    if self.pos < self.data.len() {
      let v = &self.data[self.pos];
      self.pos += 1;
      Ok(Some(v))
    } else {
      Ok(None)
    }
  }

  fn has_next(&self) -> Result<bool> {
    Ok(self.pos < self.data.len())
  }
}
pub trait VecIteratorExt<T> {
  fn iter_ext(&self) -> VecIter<'_, T>;
}

impl VecIteratorExt<String> for Vec<String> {
  fn iter_ext(&self) -> VecIter<'_, String> {
    VecIter { data: self, pos: 0 }
  }
}
impl VecIteratorExt<usize> for Vec<usize> {
  fn iter_ext(&self) -> VecIter<'_, usize> {
    VecIter { data: self, pos: 0 }
  }
}
