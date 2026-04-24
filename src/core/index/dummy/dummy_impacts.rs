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
use crate::core::index::impact::Impact;
use crate::core::index::impacts::Impacts;
use crate::core::util::error::lucene_error::Result;

#[derive(Clone)]
pub struct DummyImpacts;
impl Impacts for DummyImpacts {
  fn num_levels(&self) -> i32 {
    dummy_unreachable!()
  }

  fn get_doc_id_upto(&self, _level: i32) -> i32 {
    dummy_unreachable!()
  }

  fn get_impacts(&self, _level: i32) -> Result<Vec<Impact>> {
    dummy_unreachable!()
  }
}
