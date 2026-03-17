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
use crate::core::search::scorable::{ChildScorable, Scorable};
use crate::core::util::error::lucene_error::{LuceneError, Result};
/// Filter a [`Scorable`], intercepting methods and optionally changing their return values.
///
/// The default implementation simply passes all calls to its delegate,
/// except for [`set_min_competitive_score`](Scorable::set_min_competitive_score),
/// which defaults to a no-op.
pub struct FilterScorable<'a, S>
where
  S: Scorable + ?Sized,
{
  pub(crate) in_: &'a mut S,
}
impl<'a, S> FilterScorable<'a, S>
where
  S: Scorable + ?Sized,
{
  pub fn new(in_: &'a mut S) -> Self {
    Self { in_ }
  }
}
impl<S> Scorable for FilterScorable<'_, S>
where
  S: Scorable + ?Sized,
{
  fn score(&mut self) -> Result<f32> {
    self.in_.score()
  }

  fn get_children(&self) -> Result<Vec<ChildScorable<Box<dyn Scorable>>>> {
    todo!()
  }

  fn cost(&self) -> Result<i64> {
    Err(LuceneError::unsupported_operation(""))
  }
}
