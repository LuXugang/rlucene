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
use crate::core::codecs::{Codec, CodecUtil, CompoundFormat, LATEST_CODEC};
use crate::core::index::segment_infos::SegmentInfos;
use crate::core::index::term::Term;
use crate::core::store::IOContext;
use crate::core::store::directory::Directory;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::random_index_writer::RandomIndexWriter;
use crate::test::core::util::line_file_docs::LineFileDocs;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
  new_directory_shared, new_index_writer_config_with_analyzer, random,
};
use rand::RngExt;
use std::sync::Arc;

/// Test that a plain default puts CRC32 footers in all files.
#[allow(dead_code)] // for quick search
struct TestAllFilesHaveChecksumFooter;

#[test]
fn test() -> Result<()> {
  let mut random = random();
  let dir = new_directory_shared(&mut random)?;

  let analyzer = MockAnalyzer::new(&mut random);
  let conf = new_index_writer_config_with_analyzer(&mut random, analyzer);

  let riw = RandomIndexWriter::with_config(&mut random, dir.clone(), conf);
  // Use LineFileDocs so we (hopefully) get most Lucene features
  // tested, e.g. IntPoint was recently added to it:
  let mut docs = LineFileDocs::new(&mut random)?;

  for i in 0..100 {
    riw.add_document(docs.next_doc()?)?;

    if random.random_range(0..7) == 0 {
      riw.commit()?;
    }

    if random.random_range(0..20) == 0 {
      riw.delete_documents_with_terms(vec![Term::from_text("docid", i.to_string())])?;
    }

    if random.random_range(0..15) == 0 {
      riw.w.update_numeric_doc_value(
        Term::from_text("docid", i.to_string()),
        "page_views",
        i as i64,
      )?;
    }
  }

  riw.close()?;
  check_footers(dir.clone())?;

  Ok(())
}

fn check_footers<D>(dir: Arc<D>) -> Result<()>
where
  D: Directory,
{
  let sis = SegmentInfos::read_latest_commit(dir.clone())?;
  check_footer(dir.as_ref(), sis.get_segments_file_name().unwrap().as_ref())?;

  for si in sis.iter() {
    for file in si.files()? {
      check_footer(dir.as_ref(), &file)?;
    }

    if si.info.get_use_compound_file() {
      let cfs_dir = LATEST_CODEC
        .compound_format()
        .get_compound_reader(dir.as_ref(), &si.info)?;

      for cfs_file in cfs_dir.list_all()? {
        check_footer(&cfs_dir, &cfs_file)?;
      }
    }
  }

  Ok(())
}

fn check_footer<D>(dir: &D, file: &str) -> Result<()>
where
  D: Directory,
{
  let input = dir.open_input(file, &IOContext::read_once_io_context()?)?;
  CodecUtil::checksum_entire_file(&input)?;
  Ok(())
}
