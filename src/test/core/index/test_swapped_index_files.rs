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
use crate::core::document::document::Document;
use crate::core::index::check_index::Level;
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::WRITE_LOCK_NAME;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::merge_policy::MergePolicy;
use crate::core::store::IO_CONTEXT_DEFAULT;
use crate::core::store::directory::{DirEnum, Directory};

use crate::core::util::close::CloseableRef;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::io_utils::IOUtils;
use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test_framework::core::index::random_index_writer::RandomIndexWriter;
use crate::test_framework::core::util::line_file_docs::LineFileDocs;
use crate::test_framework::core::util::lucene_test_case::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use crate::test_framework::core::util::test_util::TestUtil;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rand_chacha::rand_core::Rng;
use std::sync::Arc;
#[allow(dead_code)] // for quick search
struct TestSwappedIndexFiles;

/** Test that the same file name, but from a different index, is detected as foreign. */
#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir1 = new_directory_shared(&mut random)?;
  let dir2 = new_directory_shared(&mut random)?;

  // Disable CFS 80% of the time so we can truncate individual files, but the other 20% of the
  // time we test truncation of .cfs/.cfe too:
  let use_cfs = random.random_range(0..5) == 1;

  // Use LineFileDocs so we (hopefully) get most Lucene features tested.
  let mut docs = LineFileDocs::new(&mut random)?;
  let doc = docs.next_doc()?;
  let seed = random.random::<u64>();

  index_one_doc(seed, dir1.clone(), doc.clone(), use_cfs)?;
  index_one_doc(seed, dir2.clone(), doc, use_cfs)?;

  swap_files(&mut random, dir1.clone(), dir2.clone())?;
  dir1.as_ref().close()?;
  dir2.as_ref().close()
}

fn index_one_doc(seed: u64, dir: Arc<DirEnum>, doc: Document, use_cfs: bool) -> Result<()> {
  let mut random = StdRng::seed_from_u64(seed);
  let analyzer = MockAnalyzer::new(&mut random);
  let mut conf = new_index_writer_config_with_analyzer(&mut random, analyzer)?;
  conf.set_codec(TestUtil::get_default_codec());

  if !use_cfs {
    conf.set_use_compound_file(false);
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(0.0)?;
  } else {
    conf.set_use_compound_file(true);
    conf
      .get_merge_policy_mut()
      .get_base_mut()
      .set_no_cfs_ratio(1.0)?;
  }

  let w = RandomIndexWriter::with_config(&mut random, dir, conf);
  w.add_document(&mut random, doc)?;
  w.close(&mut random)
}

fn swap_files<R>(random: &mut R, dir1: Arc<DirEnum>, dir2: Arc<DirEnum>) -> Result<()>
where
  R: Rng + ?Sized,
{
  for name in dir1.list_all()? {
    if name == WRITE_LOCK_NAME {
      continue;
    }
    swap_one_file(random, dir1.clone(), dir2.clone(), &name)?;
  }
  Ok(())
}

fn swap_one_file<R>(
  random: &mut R,
  dir1: Arc<DirEnum>,
  dir2: Arc<DirEnum>,
  victim: &str,
) -> Result<()>
where
  R: Rng + ?Sized,
{
  let dir_copy = new_directory_shared(random)?;
  dir_copy.set_check_index_on_close(false);
  let context = IO_CONTEXT_DEFAULT.as_ref().map_err(Clone::clone)?;

  // Copy all files from dir1 to dirCopy, except victim which we copy from dir2:
  for name in dir1.list_all()? {
    if name != victim {
      dir_copy.copy_from(dir1.as_ref(), &name, &name, context)?;
    } else {
      dir_copy.copy_from(dir2.as_ref(), &name, &name, context)?;
    }
    dir_copy.sync(&[name])?;
  }

  match directory_reader::open(dir_copy.clone()) {
    Ok(reader) => {
      reader.close()?;
      Err(LuceneError::illegal_state(format!(
        "swapped index file {} was not detected",
        victim
      )))
    },
    Err(err) if is_expected_swapped_file_error(&err) => Ok(()),
    Err(err) => Err(err),
  }?;

  // CheckIndex should also fail:
  match TestUtil::check_index_with_options(
    random,
    dir_copy.clone(),
    Level::MIN_LEVEL_FOR_SLOW_CHECKS,
    true,
    true,
    None,
  ) {
    Err(err) if is_expected_check_index_error(&err) => {},
    Err(err) => return Err(err),
    Ok(_) => {
      return Err(LuceneError::illegal_state(format!(
        "swapped index file {victim} was not detected by CheckIndex"
      )));
    },
  }

  dir_copy.as_ref().close()
}

fn is_expected_swapped_file_error(err: &LuceneError) -> bool {
  matches!(
    err,
    LuceneError::CorruptIndex(_)
      | LuceneError::Eof(_)
      | LuceneError::IndexFormatTooOld(_)
      | LuceneError::Io { .. }
      | LuceneError::IoWithPath { .. }
  )
}

fn is_expected_check_index_error(err: &LuceneError) -> bool {
  is_expected_swapped_file_error(err) || matches!(err, LuceneError::IllegalState(_))
}
