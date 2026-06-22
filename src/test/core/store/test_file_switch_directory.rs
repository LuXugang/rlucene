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
use crate::test::core::util::lucene_test_case::{
  new_index_writer_config_with_analyzer, new_log_merge_policy_with_cfs, random,
};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use rand::{Rng, RngExt};
use tempfile::Builder;

use crate::core::codecs::lucene90::compressing::lucene90_compressing_stored_fields_writer::{
  FIELDS_EXTENSION, INDEX_EXTENSION, META_EXTENSION,
};
use crate::core::index::directory_reader;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::two_phase_commit::TwoPhaseCommit;
use crate::core::store::directory::Directory;
use crate::core::store::file_switch_directory::{FileSwitchDirectory, get_extension};
use crate::core::store::index_input::{IndexInput, IndexInputEnum2};
use crate::core::store::nio_fs_directory::{NIOFSDirectory, NIOFSIndexInput};
use crate::core::store::{
  BufferedIndexInput, ByteBuffersDirectory, DataOutput, FSDirectory, IOContext, IndexOutput,
  NativeFSLockFactory,
};
use crate::core::util::close::Closeable;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer_reader::create_index_no_close;
use crate::test::core::store::base_directory_test_case::BaseDirectoryTestCase;

type NioDirectory = FSDirectory<NativeFSLockFactory, NIOFSDirectory>;
type SwitchDirectory = FileSwitchDirectory<NioDirectory, NioDirectory>;
type SwitchIndexInput =
  IndexInputEnum2<BufferedIndexInput<NIOFSIndexInput>, BufferedIndexInput<NIOFSIndexInput>>;

#[allow(dead_code)] // for quick search
pub struct TestFileSwitchDirectory;

impl TestFileSwitchDirectory {
  /// Test if writing doc stores to disk and everything else to ram works.
  fn test_basic<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let mut file_extensions = HashSet::new();
    file_extensions.insert(FIELDS_EXTENSION.to_string());
    file_extensions.insert(INDEX_EXTENSION.to_string());
    file_extensions.insert(META_EXTENSION.to_string());

    // TODO MockDirectoryWrapper未实现
    let primary_dir = ByteBuffersDirectory::new();
    let secondary_dir = ByteBuffersDirectory::new();

    let fsd = Arc::new(FileSwitchDirectory::new(
      file_extensions.clone(),
      primary_dir,
      secondary_dir,
      true,
    )?);

    // For now we use the default codec because we rely upon its specific impl.
    // TODO setCodec未实现
    let analyzer = MockAnalyzer::new(random);
    let mut config = new_index_writer_config_with_analyzer(random, analyzer);
    config.set_merge_policy(new_log_merge_policy_with_cfs(random, false)?);
    config.set_use_compound_file(false);
    let writer = IndexWriter::new(fsd.clone(), config)?;
    create_index_no_close(true, "ram", &writer)?;
    let reader = directory_reader::open_from_writer(&writer)?;
    assert_eq!(100, reader.max_doc()?);
    writer.commit()?;

    // We should see only fdx,fdt files here.
    let mut files = fsd.get_primary_dir().list_all()?;
    assert!(!files.is_empty());
    for file in &files {
      let ext = get_extension(file);
      assert!(file_extensions.contains(ext));
    }
    files = fsd.get_secondary_dir().list_all()?;
    assert!(!files.is_empty());
    // We should not see fdx,fdt files here.
    for file in &files {
      let ext = get_extension(file);
      assert!(!file_extensions.contains(ext));
    }
    reader.close()?;
    writer.close()?;

    files = fsd.list_all()?;
    for file in files {
      assert!(!file.is_empty());
    }
    Ok(())
  }

  fn new_fs_switch_directory(primary_extensions: HashSet<String>) -> Result<SwitchDirectory> {
    let prim_dir = Builder::new().prefix("foo").tempdir()?.keep();
    let second_dir = Builder::new().prefix("bar").tempdir()?.keep();
    Self::new_fs_switch_directory_with_paths(prim_dir, second_dir, primary_extensions)
  }

  fn new_fs_switch_directory_with_paths(
    a_dir: PathBuf,
    b_dir: PathBuf,
    primary_extensions: HashSet<String>,
  ) -> Result<SwitchDirectory> {
    let a = NIOFSDirectory::new(a_dir)?;
    let b = NIOFSDirectory::new(b_dir)?;
    FileSwitchDirectory::new(primary_extensions, a, b, true)
  }

  // LUCENE-3380 -- make sure we get exception if the directory really does not exist.
  fn test_no_dir<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    let prim_dir = Builder::new().prefix("foo").tempdir()?;
    let second_dir = Builder::new().prefix("bar").tempdir()?;
    let dir = Arc::new(Self::new_fs_switch_directory_with_paths(
      prim_dir.path().to_path_buf(),
      second_dir.path().to_path_buf(),
      HashSet::new(),
    )?);
    match directory_reader::open(dir) {
      Ok(_) => panic!("expected IndexNotFound or NoSuchFile"),
      Err(LuceneError::IndexNotFound(_)) | Err(LuceneError::NoSuchFile(_)) => {},
      Err(LuceneError::IoWithPath { source, .. }) | Err(LuceneError::Io { source, .. })
        if source.kind() == ErrorKind::NotFound => {},
      Err(err) => return Err(err),
    }

    Ok(())
  }

  fn test_rename_tmp_file<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    {
      let mut directory = self.get_directory(
        Builder::new()
          .prefix("renameTmp")
          .tempdir()?
          .path()
          .to_path_buf(),
        random,
      )?;
      let name;
      {
        let mut out =
          directory.create_temp_output("foo.cfs", "", &IOContext::default_io_context()?)?;
        out.write_int(1)?;
        name = out.get_name().to_string();
        out.close()?;
      }
      assert_eq!(
        1,
        directory
          .list_all()?
          .into_iter()
          .filter(|f| f == &name)
          .count()
      );
      assert_eq!(
        0,
        directory
          .list_all()?
          .into_iter()
          .filter(|f| f == "foo.cfs")
          .count()
      );
      directory.rename(&name, "foo.cfs")?;
      assert_eq!(
        1,
        directory
          .list_all()?
          .into_iter()
          .filter(|f| f == "foo.cfs")
          .count()
      );
      assert_eq!(
        0,
        directory
          .list_all()?
          .into_iter()
          .filter(|f| f == &name)
          .count()
      );
      directory.close()?;
    }

    {
      let mut primary_extensions = HashSet::new();
      primary_extensions.insert("bar".to_string());
      let mut directory = Self::new_fs_switch_directory(primary_extensions)?;
      let broken_name;
      {
        let mut out =
          directory.create_temp_output("foo", "bar", &IOContext::default_io_context()?)?;
        out.write_int(1)?;
        broken_name = out.get_name().to_string();
        out.close()?;
      }
      let exception = directory
        .rename(&broken_name, "foo.bar")
        .expect_err("source and dest should be in different directories");
      match exception {
        LuceneError::Io { source, .. } | LuceneError::IoWithPath { source, .. } => {
          assert_eq!(
            "foo_bar_0.tmp -> foo.bar: source and dest are in different directories",
            source.to_string()
          );
        },
        other => panic!("unexpected error: {other}"),
      }
      directory.close()?;
    }

    Ok(())
  }

  fn get_directory<R>(&self, _path: PathBuf, random: &mut R) -> Result<SwitchDirectory>
  where
    R: Rng + ?Sized,
  {
    let mut extensions = HashSet::new();
    if random.random_bool(0.5) {
      extensions.insert("cfs".to_string());
    }
    if random.random_bool(0.5) {
      extensions.insert("prx".to_string());
    }
    if random.random_bool(0.5) {
      extensions.insert("frq".to_string());
    }
    if random.random_bool(0.5) {
      extensions.insert("tip".to_string());
    }
    if random.random_bool(0.5) {
      extensions.insert("tim".to_string());
    }
    if random.random_bool(0.5) {
      extensions.insert("del".to_string());
    }
    Self::new_fs_switch_directory(extensions)
  }

  fn test_delete_and_list<R>(&self, _random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    // Relies on Windows semantics.
    let path = Builder::new().prefix("deleteAndList").tempdir()?;
    let index_path = path.path().to_path_buf();
    let mut primary_extensions = HashSet::new();
    primary_extensions.insert("tim".to_string());
    let mut dir = FileSwitchDirectory::new(
      primary_extensions,
      NIOFSDirectory::new(index_path.clone())?,
      NIOFSDirectory::new(index_path)?,
      true,
    )?;
    {
      let mut out = dir.create_output("foo.tim", &IOContext::default_io_context()?)?;
      out.write_int(1)?;
      out.close()?;
    }
    let strip_extra = |array: Vec<String>| -> usize {
      array
        .into_iter()
        .filter(|f| !f.starts_with("extra"))
        .count()
    };
    {
      let index_input = dir.open_input("foo.tim", &IOContext::default_io_context()?)?;
      assert!(index_input.length()? > 0);
      dir.delete_file("foo.tim")?;
      // TODO IMPORTANT WindowsFS is not implemented in Rust yet. The Java test keeps the
      // file pending while an input is open; native Rust paths delete it
      // immediately, but the same-path primary/secondary listing behavior is
      // still covered here.
      assert_eq!(0, dir.get_primary_dir().get_pending_deletions()?.len());
      assert_eq!(0, dir.get_pending_deletions()?.len());
      assert_eq!(0, strip_extra(dir.list_all()?));
      assert_eq!(0, strip_extra(dir.get_primary_dir().list_all()?));
      assert_eq!(0, strip_extra(dir.get_secondary_dir().list_all()?));
      drop(index_input);
    }
    assert_eq!(0, dir.get_primary_dir().get_pending_deletions()?.len());
    assert_eq!(0, dir.get_pending_deletions()?.len());
    assert_eq!(0, strip_extra(dir.list_all()?));
    assert_eq!(0, strip_extra(dir.get_primary_dir().list_all()?));
    assert_eq!(0, strip_extra(dir.get_secondary_dir().list_all()?));
    dir.close()
  }
}

impl BaseDirectoryTestCase for TestFileSwitchDirectory {
  type Directory = SwitchDirectory;
  type Output = SwitchIndexInput;

  fn test_no_dir<R>(&self, random: &mut R) -> Result<()>
  where
    R: Rng + ?Sized,
  {
    TestFileSwitchDirectory::test_no_dir(self, random)
  }

  fn get_directory<R>(&self, path: PathBuf, random: &mut R) -> Result<Self::Directory>
  where
    R: Rng + ?Sized,
  {
    TestFileSwitchDirectory::get_directory(self, path, random)
  }
}

#[test]
fn test_basic() -> Result<()> {
  run_case(|case, random| case.test_basic(random))
}

#[test]
fn test_rename_tmp_file() -> Result<()> {
  run_case(|case, random| case.test_rename_tmp_file(random))
}

#[test]
fn test_delete_and_list() -> Result<()> {
  run_case(|case, random| case.test_delete_and_list(random))
}

mod base_directory_test_case_tests {
  use crate::core::util::error::lucene_error::Result;
  use crate::test::core::store::base_directory_test_case::BaseDirectoryTestCase;
  use crate::test::core::store::test_file_switch_directory::run_case;

  #[test]
  fn test_copy_from() -> Result<()> {
    run_case(|case, random| case.test_copy_from(random))
  }

  #[test]
  fn test_rename() -> Result<()> {
    run_case(|case, random| case.test_rename(random))
  }

  #[test]
  fn test_delete_file() -> Result<()> {
    run_case(|case, random| case.test_delete_file(random))
  }

  #[test]
  fn test_byte() -> Result<()> {
    run_case(|case, random| case.test_byte(random))
  }

  #[test]
  fn test_short() -> Result<()> {
    run_case(|case, random| case.test_short(random))
  }

  #[test]
  fn test_int() -> Result<()> {
    run_case(|case, random| case.test_int(random))
  }

  #[test]
  fn test_long() -> Result<()> {
    run_case(|case, random| case.test_long(random))
  }

  #[test]
  fn test_aligned_little_endian_longs() -> Result<()> {
    run_case(|case, random| case.test_aligned_little_endian_longs(random))
  }

  #[test]
  fn test_unaligned_little_endian_longs() -> Result<()> {
    run_case(|case, random| case.test_unaligned_little_endian_longs(random))
  }

  #[test]
  fn test_little_endian_longs_underflow() -> Result<()> {
    run_case(|case, random| case.test_little_endian_longs_underflow(random))
  }

  #[test]
  fn test_aligned_ints() -> Result<()> {
    run_case(|case, random| case.test_aligned_ints(random))
  }

  #[test]
  fn test_unaligned_ints() -> Result<()> {
    run_case(|case, random| case.test_unaligned_ints(random))
  }

  #[test]
  fn test_ints_underflow() -> Result<()> {
    run_case(|case, random| case.test_ints_underflow(random))
  }

  #[test]
  fn test_aligned_floats() -> Result<()> {
    run_case(|case, random| case.test_aligned_floats(random))
  }

  #[test]
  fn test_unaligned_floats() -> Result<()> {
    run_case(|case, random| case.test_unaligned_floats(random))
  }

  #[test]
  fn test_floats_underflow() -> Result<()> {
    run_case(|case, random| case.test_floats_underflow(random))
  }

  #[test]
  fn test_string() -> Result<()> {
    run_case(|case, random| case.test_string(random))
  }

  #[test]
  fn test_vint() -> Result<()> {
    run_case(|case, random| case.test_vint(random))
  }

  #[test]
  fn test_vlong() -> Result<()> {
    run_case(|case, random| case.test_vlong(random))
  }

  #[test]
  fn test_zint() -> Result<()> {
    run_case(|case, random| case.test_zint(random))
  }

  #[test]
  fn test_zlong() -> Result<()> {
    run_case(|case, random| case.test_zlong(random))
  }

  #[test]
  fn test_set_of_strings() -> Result<()> {
    run_case(|case, random| case.test_set_of_strings(random))
  }

  #[test]
  fn test_map_of_strings() -> Result<()> {
    run_case(|case, random| case.test_map_of_strings(random))
  }

  #[test]
  fn test_checksum() -> Result<()> {
    run_case(|case, random| case.test_checksum(random))
  }

  #[test]
  fn test_thread_safety_in_list_all() -> Result<()> {
    run_case(|case, random| case.test_thread_safety_in_list_all(random))
  }

  #[test]
  fn test_file_exists_in_list_after_created() -> Result<()> {
    run_case(|case, random| case.test_file_exists_in_list_after_created(random))
  }

  #[test]
  fn test_seek_to_eof_then_back() -> Result<()> {
    run_case(|case, random| case.test_seek_to_eof_then_back(random))
  }

  #[test]
  fn test_illegal_eof() -> Result<()> {
    run_case(|case, random| case.test_illegal_eof(random))
  }

  #[test]
  fn test_seek_past_eof() -> Result<()> {
    run_case(|case, random| case.test_seek_past_eof(random))
  }

  #[test]
  fn test_slice_out_of_bounds() -> Result<()> {
    run_case(|case, random| case.test_slice_out_of_bounds(random))
  }

  #[test]
  fn test_no_dir() -> Result<()> {
    run_case(|case, random| case.test_no_dir(random))
  }

  #[test]
  fn test_copy_bytes() -> Result<()> {
    run_case(|case, random| case.test_copy_bytes(random))
  }

  #[test]
  fn test_copy_bytes_with_threads() -> Result<()> {
    run_case(|case, random| case.test_copy_bytes_with_threads(random))
  }

  #[test]
  fn test_fsync_doesnt_create_new_files() -> Result<()> {
    run_case(|case, random| case.test_fsync_doesnt_create_new_files(random))
  }

  #[test]
  fn test_random_long() -> Result<()> {
    run_case(|case, random| case.test_random_long(random))
  }

  #[test]
  fn test_random_int() -> Result<()> {
    run_case(|case, random| case.test_random_int(random))
  }

  #[test]
  fn test_random_short() -> Result<()> {
    run_case(|case, random| case.test_random_short(random))
  }

  #[test]
  fn test_random_byte() -> Result<()> {
    run_case(|case, random| case.test_random_byte(random))
  }

  #[test]
  fn test_slice_of_slice() -> Result<()> {
    run_case(|case, random| case.test_slice_of_slice(random))
  }

  #[test]
  fn test_large_writes() -> Result<()> {
    run_case(|case, random| case.test_large_writes(random))
  }

  #[test]
  fn test_index_output_to_string() -> Result<()> {
    run_case(|case, random| case.test_index_output_to_string(random))
  }

  #[test]
  fn test_create_temp_output() -> Result<()> {
    run_case(|case, random| case.test_create_temp_output(random))
  }

  #[test]
  fn test_create_output_for_existing_file() -> Result<()> {
    run_case(|case, random| case.test_create_output_for_existing_file(random))
  }

  #[test]
  fn test_seek_to_end_of_file() -> Result<()> {
    run_case(|case, random| case.test_seek_to_end_of_file(random))
  }

  #[test]
  fn test_seek_beyond_end_of_file() -> Result<()> {
    run_case(|case, random| case.test_seek_beyond_end_of_file(random))
  }

  #[test]
  fn test_pending_deletions() -> Result<()> {
    run_case(|case, random| case.test_pending_deletions(random))
  }

  #[test]
  fn test_list_all_is_sorted() -> Result<()> {
    run_case(|case, random| case.test_list_all_is_sorted(random))
  }

  #[test]
  fn test_data_types() -> Result<()> {
    run_case(|case, random| case.test_data_types(random))
  }

  #[test]
  fn test_group_vint_overflow() -> Result<()> {
    run_case(|case, random| case.test_group_vint_overflow(random))
  }

  #[test]
  fn test_group_vint() -> Result<()> {
    run_case(|case, random| case.test_group_vint(random))
  }

  #[test]
  fn test_prefetch() -> Result<()> {
    run_case(|case, random| case.test_prefetch(random))
  }

  #[test]
  fn test_prefetch_on_slice() -> Result<()> {
    run_case(|case, random| case.test_prefetch_on_slice(random))
  }

  #[test]
  fn test_update_read_advice() -> Result<()> {
    run_case(|case, random| case.test_update_read_advice(random))
  }

  #[test]
  fn test_is_loaded() -> Result<()> {
    run_case(|case, random| case.test_is_loaded(random))
  }

  #[test]
  fn test_is_loaded_on_slice() -> Result<()> {
    run_case(|case, random| case.test_is_loaded_on_slice(random))
  }
}

fn run_case<F>(f: F) -> Result<()>
where
  F: FnOnce(&TestFileSwitchDirectory, &mut rand::prelude::StdRng) -> Result<()>,
{
  let mut random = random();
  let case = TestFileSwitchDirectory;
  f(&case, &mut random)
}
