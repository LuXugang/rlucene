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
use crate::core::index::merge_policy::OneMergeSR;
use crate::core::index::merge_scheduler::{MergeScheduler, MergeSource};
use crate::core::index::merge_trigger::MergeTrigger;
use crate::core::store::directory::Directory;
use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::Result;
#[allow(dead_code)] // for quick search
struct TestIndexWriterMerging;
pub struct MyMergeScheduler;

impl CloseableRef for MyMergeScheduler {
  fn close(&self) -> Result<()> {
    Ok(())
  }
}

impl MergeScheduler for MyMergeScheduler {
  fn merge<MS, D>(&self, merge_source: MS, _trigger: MergeTrigger) -> Result<()>
  where
    MS: MergeSource<D> + Clone + 'static,
    D: Directory + 'static,
    OneMergeSR<D>: Send + 'static,
  {
    loop {
      let merge = match merge_source.get_next_merge()? {
        Some(merge) => merge,
        None => break,
      };
      let mut num_docs = 0;
      for segment in &merge.segments {
        let max_doc = segment.max_doc;
        num_docs += max_doc;
        assert!(max_doc < 20);
      }
      let merge_stat = merge.stat.clone();
      merge_source.merge(merge)?;
      assert_eq!(Some(num_docs), merge_stat.max_doc());
    }
    Ok(())
  }

  type Directory<D>
    = D
  where
    D: Directory;

  fn wrap_for_merge<D>(&self, in_: D) -> Result<Self::Directory<D>>
  where
    D: Directory,
  {
    Ok(in_)
  }
}
