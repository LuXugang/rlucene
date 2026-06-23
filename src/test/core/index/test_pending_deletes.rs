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
use crate::core::codecs::field_infos_format::FieldInfosFormat;
use crate::core::codecs::live_docs_format::LiveDocsFormat;
use crate::core::codecs::{Codec, LATEST_CODEC};
use crate::core::index::field_infos::FieldInfos;
use crate::core::index::pending_deletes::{PendingDeletes, PendingDeletesBase, PendingDeletesEnum};
use crate::core::index::segment_commit_info::{SegmentCommitInfo, SegmentCommitInfoMeta};
use crate::core::index::segment_info::SegmentInfo;
use crate::test::core::util::lucene_test_case::random;

use crate::core::store::ByteBuffersDirectory;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::Result;
use crate::core::util::{LATEST, StringHelper};

use crate::core::index::pending_soft_deletes;
use rand::RngExt;
use rand::prelude::StdRng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestPendingDeletes;

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestPendingDeletes, &mut StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestPendingDeletes;
  f(&case, &mut random)
}
mod test_pending_deletes_base_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::index::test_pending_deletes::{TestPendingDeletesBase, run_case};

  #[test]
  fn test_delete_doc() -> Result<()> {
    run_case(|case, random| case.test_delete_doc(random))
  }
  #[test]
  fn test_write_live_docs() -> Result<()> {
    run_case(|case, random| case.test_write_live_docs(random))
  }
  #[test]
  fn test_is_fully_deleted() -> Result<()> {
    run_case(|case, random| case.test_is_fully_deleted(random))
  }
}

impl TestPendingDeletesBase for TestPendingDeletes {
  fn new_pending_deletes<D>(
    &self,
    commit_info: &SegmentCommitInfoMeta<D>,
  ) -> Result<PendingDeletesEnum>
  where
    D: Directory,
  {
    Ok(PendingDeletesEnum::PD(PendingDeletes::new(commit_info)?))
  }
}

pub(crate) trait TestPendingDeletesBase {
  fn new_pending_deletes<D>(
    &self,
    commit_info: &SegmentCommitInfoMeta<D>,
  ) -> Result<PendingDeletesEnum>
  where
    D: Directory;

  fn test_delete_doc<R>(&self, random: &mut R) -> Result<()>
  where
    R: rand::Rng,
  {
    let dir = Arc::new(ByteBuffersDirectory::new());
    let si = SegmentInfo::new(
      dir.clone(),
      Some((*LATEST).clone()),
      Some((*LATEST).clone()),
      "test",
      10,
      false,
      false,
      HashMap::new(),
      StringHelper::random_id(),
      HashMap::new(),
      None,
    )?;
    let commit_info = SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));
    let meta = commit_info.to_meta()?;
    let mut deletes = self.new_pending_deletes(&meta)?;
    assert!(deletes.get_live_docs().is_none());

    let doc_to_delete = random.random_range(0..=7);
    assert!(deletes.delete(doc_to_delete, &commit_info)?);
    let mut live_docs = deletes.get_live_docs().unwrap();
    assert_eq!(deletes.num_pending_deletes(), 1);

    assert!(!live_docs.get(doc_to_delete as usize)?);
    assert!(!deletes.delete(doc_to_delete, &commit_info)?);

    assert!(live_docs.get(8)?);
    assert!(deletes.delete(8, &commit_info)?);
    assert!(live_docs.get(8)?);
    assert_eq!(deletes.num_pending_deletes(), 2);

    assert!(live_docs.get(9)?);
    assert!(deletes.delete(9, &commit_info)?);
    assert!(live_docs.get(9)?);

    live_docs = deletes.get_live_docs().unwrap();
    assert!(!live_docs.get(8)?);
    assert!(!live_docs.get(9)?);
    assert!(!live_docs.get(doc_to_delete as usize)?);
    assert_eq!(deletes.num_pending_deletes(), 3);
    Ok(())
  }
  fn test_write_live_docs<R>(&self, random: &mut R) -> Result<()>
  where
    R: rand::Rng,
  {
    let dir = Arc::new(ByteBuffersDirectory::new());
    let si = SegmentInfo::new(
      dir.clone(),
      Some((*LATEST).clone()),
      Some((*LATEST).clone()),
      "test",
      6,
      false,
      false,
      HashMap::new(),
      StringHelper::random_id(),
      HashMap::new(),
      None,
    )?;
    let mut commit_info =
      SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));

    let meta = commit_info.to_meta()?;
    let mut deletes = self.new_pending_deletes(&meta)?;
    assert!(!deletes.write_live_docs(dir.clone(), &mut commit_info)?);
    assert_eq!(dir.list_all()?.len(), 0);

    let second_doc_deletes: bool = random.random_bool(0.5);
    deletes.delete(5, &commit_info)?;
    if second_doc_deletes {
      let _ = deletes.get_live_docs();
      deletes.delete(2, &commit_info)?;
    }

    assert_eq!(commit_info.get_del_gen(), -1);
    assert_eq!(commit_info.get_del_count(), 0);

    let expected_pending = if second_doc_deletes { 2 } else { 1 };
    assert_eq!(deletes.num_pending_deletes(), expected_pending);

    assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
    assert_eq!(dir.list_all()?.len(), 1);

    let codec = &*LATEST_CODEC;
    let live_docs = codec.live_docs_format().read_live_docs(
      dir.as_ref(),
      &commit_info,
      &IOContext::default_io_context()?,
    )?;
    assert!(!live_docs.get(5)?);
    if second_doc_deletes {
      assert!(!live_docs.get(2)?);
    } else {
      assert!(live_docs.get(2)?);
    }
    for doc in &[0, 1, 3, 4] {
      assert!(live_docs.get(*doc)?);
    }

    assert_eq!(deletes.num_pending_deletes(), 0);
    assert_eq!(commit_info.get_del_count(), expected_pending);
    assert_eq!(commit_info.get_del_gen(), 1);

    deletes.delete(0, &commit_info)?;
    assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
    assert_eq!(dir.list_all()?.len(), 2);

    let live_docs = codec.live_docs_format().read_live_docs(
      dir.as_ref(),
      &commit_info,
      &IOContext::default_io_context()?,
    )?;
    assert!(!live_docs.get(5)?);
    if second_doc_deletes {
      assert!(!live_docs.get(2)?);
    } else {
      assert!(live_docs.get(2)?);
    }
    assert!(!live_docs.get(0)?);
    for doc in &[1, 3, 4] {
      assert!(live_docs.get(*doc)?);
    }

    assert_eq!(deletes.num_pending_deletes(), 0);
    let expected_total = expected_pending + 1;
    assert_eq!(commit_info.get_del_count(), expected_total);
    assert_eq!(commit_info.get_del_gen(), 2);

    Ok(())
  }
  fn test_is_fully_deleted<R>(&self, random: &mut R) -> Result<()>
  where
    R: rand::Rng,
  {
    let dir = Arc::new(ByteBuffersDirectory::new());
    let si = SegmentInfo::new(
      dir.clone(),
      Some((*LATEST).clone()),
      Some((*LATEST).clone()),
      "test",
      3,
      false,
      false,
      HashMap::new(),
      StringHelper::random_id(),
      HashMap::new(),
      None,
    )?;
    let mut commit_info =
      SegmentCommitInfo::new(si, 0, 0, -1, -1, -1, Some(StringHelper::random_id()));

    let codec = &*LATEST_CODEC;
    let field_infos = FieldInfos::new(Vec::new())?;
    codec.field_infos_format().write(
      dir.as_ref(),
      &commit_info.info,
      "",
      &field_infos,
      &IOContext::default_io_context()?,
    )?;
    let meta = commit_info.to_meta()?;
    let mut deletes = self.new_pending_deletes(&meta)?;

    for i in 0..3 {
      assert!(deletes.delete(i, &commit_info)?);
      if random.random_bool(0.5) {
        assert!(deletes.write_live_docs(dir.clone(), &mut commit_info)?);
      }

      let (dv_gen, field) = match deletes {
        PendingDeletesEnum::Soft(ref v) => (v.dv_generation, v.field.to_string()),
        // not -2
        PendingDeletesEnum::PD(_) => (-1, "".to_string()),
      };
      let (reader, field_infos, on_new_reader) = if dv_gen == -2 {
        let field_infos = pending_soft_deletes::read_field_infos(&commit_info)?;
        let field_info = field_infos.field_info_by_name(field.as_ref());
        let on_new_reader = pending_soft_deletes::do_on_new_reader(field_info.as_ref());
        (None, Some(field_infos), on_new_reader)
      } else {
        (None, None, false)
      };
      assert_eq!(
        i == 2,
        deletes.is_fully_deleted(&commit_info, reader, field_infos, on_new_reader)?
      );
    }

    Ok(())
  }
}
