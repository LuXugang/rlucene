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
use crate::core::search::scorable::Scorable;
use crate::core::util::error::lucene_error::Result;
/// Consumer for [`DocIdStream`] items.
pub trait DocIdStreamConsumer {
  fn accept(&mut self, doc: i32) -> Result<()>;
  fn accept_with_score(&mut self, doc: i32, score: f32) -> Result<()>;
}

/// A stream of doc IDs. Most methods on [`DocIdStream`]s are terminal,
/// meaning that the [`DocIdStream`] may not be further used.
pub trait DocIdStream {
  /// Iterate over doc IDs contained in this stream in order,
  /// calling the given consumer on them.
  /// This is a terminal operation.
  fn for_each<DC: DocIdStreamConsumer + ?Sized>(&mut self, f: &mut DC) -> Result<()>;

  /// Count the number of entries in this stream.
  /// This is a terminal operation.
  fn count<SC: Scorable + ?Sized>(&mut self, scorer: &mut SC) -> Result<i32>;
  fn default_count<SC: Scorable + ?Sized>(&mut self, scorer: &mut SC) -> Result<i32> {
    let mut counter = CountConsumer { cnt: 0, scorer };
    self.for_each(&mut counter)?;
    Ok(counter.cnt)
  }
}
struct CountConsumer<'a, S>
where
  S: ?Sized,
{
  cnt: i32,
  scorer: &'a mut S,
}

impl<S> DocIdStreamConsumer for CountConsumer<'_, S>
where
  S: Scorable + ?Sized,
{
  fn accept(&mut self, _doc: i32) -> Result<()> {
    self.cnt += 1;
    Ok(())
  }

  fn accept_with_score(&mut self, _doc: i32, score: f32) -> Result<()> {
    self.cnt += 1;
    self.scorer.set_score(score)?;
    Ok(())
  }
}

impl<DC: DocIdStreamConsumer + ?Sized> DocIdStreamConsumer for &mut DC {
  fn accept(&mut self, doc: i32) -> Result<()> {
    (**self).accept(doc)
  }
  fn accept_with_score(&mut self, doc: i32, score: f32) -> Result<()> {
    (**self).accept_with_score(doc, score)
  }
}
impl<DS: DocIdStream + ?Sized> DocIdStream for &mut DS {
  fn for_each<DC: DocIdStreamConsumer + ?Sized>(&mut self, consumer: &mut DC) -> Result<()> {
    (**self).for_each(consumer)
  }
  fn count<SC: Scorable + ?Sized>(&mut self, scorer: &mut SC) -> Result<i32> {
    (**self).count(scorer)
  }
}
