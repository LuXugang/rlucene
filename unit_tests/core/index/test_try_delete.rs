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
#[allow(dead_code)] // for quick search
struct TestTryDelete;

#[cfg(test)]
mod tests {
  use crate::core::document::document::Document;
  use crate::core::document::field::Store;
  use crate::core::document::string_field::StringField;
  use crate::core::index::directory_reader;
  use crate::core::index::index_reader::IndexReader;
  use crate::core::index::index_writer::IndexWriter;
  use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
  use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
  use crate::core::index::log_byte_size_merge_policy::LogByteSizeMergePolicy;
  use crate::core::index::log_merge_policy::LogMergePolicy;
  use crate::core::index::term::Term;
  use crate::core::index::two_phase_commit::TwoPhaseCommit;
  use crate::core::search::query::Query;
  use crate::core::search::term_query::TermQuery;
  use crate::core::store::directory::DirEnum;
  use crate::core::util::error::lucene_error::Result;
  use crate::test_framework::core::analysis::mock_analyzer::MockAnalyzer;
  use crate::test_framework::core::util::lucene_test_case::{
    new_directory_shared, new_searcher_with_reader, random,
  };
  use rand::RngExt;
  use std::sync::Arc;

  fn get_writer(directory: Arc<DirEnum>) -> Result<Arc<IndexWriter<DirEnum>>> {
    let mut random = random();
    let mp = LogMergePolicy::<LogByteSizeMergePolicy>::log_bytes_size();
    let a = MockAnalyzer::new(&mut random);
    let mut conf = IndexWriterConfig::with_analyzer(a)?;
    conf.set_merge_policy(mp);
    conf.set_open_mode(OpenMode::CreateOrAppend);
    IndexWriter::new(directory, conf)
  }

  fn create_index() -> Result<Arc<DirEnum>> {
    let mut random = random();
    let directory = new_directory_shared(&mut random)?;

    let writer = get_writer(directory.clone())?;

    for i in 0..10 {
      let mut doc = Document::new();
      doc.add(StringField::from_string("foo", i.to_string(), Store::Yes)?);
      writer.add_document(doc)?;
    }

    writer.commit()?;
    writer.close()?;

    Ok(directory)
  }

  #[test]
  fn test_try_delete_document() -> Result<()> {
    // TODO IMPORTANT SearcherManager未实现
    let directory = create_index()?;

    let writer = get_writer(directory.clone())?;

    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;
    assert_eq!(1, top_docs.total_hits.value());

    let result;
    if random().random_bool(0.5) {
      let r = directory_reader::open_from_writer(&writer)?;
      result = writer.try_delete_document(&r, 0)?;
      r.close()?;
    } else {
      let reader = directory_reader::open_from_writer(&writer)?;
      result = writer.try_delete_document(&reader, 0)?;
    }

    // The tryDeleteDocument should have succeeded:
    assert_ne!(result, -1);

    assert!(writer.has_deletions()?);

    if random().random_bool(0.5) {
      writer.commit()?;
    }

    assert!(writer.has_deletions()?);

    // Re-open reader to see changes (replaces mgr.maybeRefresh())
    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;

    assert_eq!(0, top_docs.total_hits.value());

    writer.close()?;
    Ok(())
  }

  #[test]
  fn test_try_delete_document_close_and_reopen() -> Result<()> {
    // TODO IMPORTANT SearcherManager未实现
    let directory = create_index()?;

    let writer = get_writer(directory.clone())?;

    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;
    assert_eq!(1, top_docs.total_hits.value());

    let r = directory_reader::open_from_writer(&writer)?;
    let result = writer.try_delete_document(&r, 0)?;

    assert_ne!(result, -1);

    writer.commit()?;

    assert!(writer.has_deletions()?);

    // Re-open reader to see changes (replaces mgr.maybeRefresh())
    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;

    assert_eq!(0, top_docs.total_hits.value());

    writer.close()?;

    // Open from directory directly (no writer)
    let reader = crate::core::index::directory_reader::open(directory)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;

    assert_eq!(0, top_docs.total_hits.value());

    Ok(())
  }

  #[test]
  fn test_delete_documents() -> Result<()> {
    // TODO IMPORTANT SearcherManager未实现
    let directory = create_index()?;

    let writer = get_writer(directory.clone())?;

    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;
    assert_eq!(1, top_docs.total_hits.value());
    let result = writer.delete_documents_with_queries(vec![Query::from(TermQuery::new(
      Term::from_text("foo", "0"),
    ))])?;

    assert_ne!(result, -1);

    // writer.commit();

    assert!(writer.has_deletions()?);

    // Re-open reader to see changes (replaces mgr.maybeRefresh())
    let reader = directory_reader::open_from_writer(&writer)?;
    let searcher = new_searcher_with_reader(reader)?;

    let top_docs = searcher.search(TermQuery::new(Term::from_text("foo", "0")), 100)?;

    assert_eq!(0, top_docs.total_hits.value());

    writer.close()?;
    Ok(())
  }
}
