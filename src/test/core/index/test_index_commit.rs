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
use crate::core::store::directory::DirEnum;
use crate::test::core::util::lucene_test_case::{new_directory_shared, random};

use crate::core::index::index_commit::{IndexCommit, cmp_commit, is_same_commit};
use crate::core::util::error::lucene_error::Result;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

struct TestIndexCommit {
  segments_file_name: String,
  directory: Arc<DirEnum>,
  file_names: Vec<String>,
  deleted: bool,
  generation: i64,
  user_data: HashMap<String, String>,
  segment_count: usize,
}

impl PartialEq for TestIndexCommit {
  fn eq(&self, other: &Self) -> bool {
    is_same_commit(self, other)
  }
}

impl Eq for TestIndexCommit {}

impl Debug for TestIndexCommit {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TestIndexCommit")
      .field("segments_file_name", &self.segments_file_name)
      .field("generation", &self.generation)
      .field("segment_count", &self.segment_count)
      .finish()
  }
}

impl Hash for TestIndexCommit {
  fn hash<H: Hasher>(&self, state: &mut H) {
    std::ptr::hash(Arc::as_ptr(&self.directory), state);
    self.generation.hash(state);
  }
}

impl PartialOrd for TestIndexCommit {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TestIndexCommit {
  fn cmp(&self, other: &Self) -> Ordering {
    cmp_commit(self, other)
  }
}

impl Display for TestIndexCommit {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}({})",
      std::any::type_name::<Self>(),
      self.segments_file_name
    )
  }
}

impl IndexCommit for TestIndexCommit {
  fn get_segments_file_name(&self) -> &str {
    &self.segments_file_name
  }

  fn get_file_names(&self) -> Result<&[String]> {
    Ok(self.file_names.as_slice())
  }

  type Directory = DirEnum;

  fn get_directory(&self) -> Arc<Self::Directory> {
    self.directory.clone()
  }

  fn delete(&mut self) -> Result<()> {
    Ok(())
  }

  fn is_deleted(&self) -> bool {
    self.deleted
  }

  fn get_segment_count(&self) -> usize {
    self.segment_count
  }

  fn get_generation(&self) -> i64 {
    self.generation
  }

  fn get_user_data(&self) -> &HashMap<String, String> {
    &self.user_data
  }
}

#[test]
fn test_equals_hash_code() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let ic1 = TestIndexCommit {
    segments_file_name: "a".to_string(),
    directory: dir.clone(),
    file_names: Vec::new(),
    deleted: false,
    generation: 0,
    user_data: HashMap::new(),
    segment_count: 2,
  };

  let ic2 = TestIndexCommit {
    segments_file_name: "b".to_string(),
    directory: dir,
    file_names: Vec::new(),
    deleted: false,
    generation: 0,
    user_data: HashMap::new(),
    segment_count: 2,
  };

  assert_eq!(ic1, ic2);

  let mut ic1_hasher = DefaultHasher::new();
  ic1.hash(&mut ic1_hasher);
  let mut ic2_hasher = DefaultHasher::new();
  ic2.hash(&mut ic2_hasher);
  assert_eq!(
    ic1_hasher.finish(),
    ic2_hasher.finish(),
    "hash codes are not equals"
  );

  Ok(())
}
