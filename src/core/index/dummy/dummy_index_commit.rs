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
use crate::core::index::index_commit::IndexCommit;
use crate::core::store::directory::Directory;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

pub struct DummyIndexCommit<D>
where
  D: Directory,
{
  _marker: PhantomData<D>,
}

impl<D> PartialEq for DummyIndexCommit<D>
where
  D: Directory,
{
  fn eq(&self, _other: &Self) -> bool {
    dummy_unreachable!()
  }
}

impl<D> Eq for DummyIndexCommit<D> where D: Directory {}

impl<D> PartialOrd for DummyIndexCommit<D>
where
  D: Directory,
{
  fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
    Some(self.cmp(_other))
  }
}

impl<D> Ord for DummyIndexCommit<D>
where
  D: Directory,
{
  fn cmp(&self, _other: &Self) -> Ordering {
    dummy_unreachable!()
  }
}

impl<D> Display for DummyIndexCommit<D>
where
  D: Directory,
{
  fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
    dummy_unreachable!()
  }
}

impl<D> IndexCommit for DummyIndexCommit<D>
where
  D: Directory,
{
  fn get_segments_file_name(&self) -> &str {
    dummy_unreachable!()
  }

  fn get_file_names(&self) -> crate::core::util::error::lucene_error::Result<&[String]> {
    dummy_unreachable!()
  }

  type Directory = Arc<D>;

  fn get_directory(&self) -> Self::Directory {
    dummy_unreachable!()
  }

  fn delete(&self) -> crate::core::util::error::lucene_error::Result<()> {
    dummy_unreachable!()
  }

  fn is_deleted(&self) -> bool {
    dummy_unreachable!()
  }

  fn get_segment_count(&self) -> usize {
    dummy_unreachable!()
  }

  fn get_generation(&self) -> i64 {
    dummy_unreachable!()
  }

  fn get_user_data(&self) -> &HashMap<String, String> {
    dummy_unreachable!()
  }
}
