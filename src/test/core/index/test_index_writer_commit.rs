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
use crate::core::index::directory_reader::{DirectoryReader, directory_reader_util};
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::term::Term;
use crate::core::search::term_query::TermQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::core::analysis::mock_analyzer::MockAnalyzer;
use crate::test::core::index::test_index_writer::add_doc;
use crate::test::core::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config_with_analyzer, new_searcher_with_reader, random,
};
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
struct TestIndexWriterCommit;
/*
 * Simple test for "commit on close": open writer then
 * add a bunch of docs, making sure reader does not see
 * these docs until writer is closed.
 */
#[test]
fn test_commit_on_close() -> Result<()> {
    let mut random = random();

    let dir = new_directory_shared(&mut random)?;
    let mut field_types = HashMap::new();

    let mock = MockAnalyzer::new(&mut random);
    let iwc1 = new_index_writer_config_with_analyzer(&mut random, mock);
    {
        let writer = IndexWriter::new(dir.clone(), iwc1)?;

        for _ in 0..14 {
            add_doc(&mut random, &writer, &mut field_types)?;
        }

        writer.close()?;
    }

    let search_term = Term::from_text("content", "aaa");

    {
        let reader = directory_reader_util::open(dir.clone())?;
        let searcher = new_searcher_with_reader(reader)?;
        let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
        assert_eq!(14, hits.score_docs.len(), "first number of hits");
    }

    let reader = directory_reader_util::open(dir.clone())?;

    let mock = MockAnalyzer::new(&mut random);
    let iwc2 = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc2)?;

    for _ in 0..3 {
        for _ in 0..11 {
            add_doc(&mut random, &writer, &mut field_types)?;
        }

        let r = directory_reader_util::open(dir.clone())?;
        let searcher = new_searcher_with_reader(r)?;
        let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
        assert_eq!(
            14,
            hits.score_docs.len(),
            "reader incorrectly sees changes from writer"
        );

        assert!(
            reader.is_current(&writer)?,
            "reader should have still been current"
        );
    }

    writer.close()?;

    assert!(
        !reader.is_current(&writer)?,
        "reader should not be current now"
    );

    {
        let r = directory_reader_util::open(dir.clone())?;
        let searcher = new_searcher_with_reader(r)?;
        let hits = searcher.search(TermQuery::new(search_term.clone()), 1000)?;
        assert_eq!(
            47,
            hits.score_docs.len(),
            "reader did not see changes after writer was closed"
        );
    }

    Ok(())
}
#[test]
fn test_commit_on_close_abort() -> Result<()> {
    // TODO: roll_back未实现
    Ok(())
}

#[test]
fn test_commit_on_close_disk_usage() -> Result<()> {
    Ok(())
}
#[test]
fn test_commit_on_close_force_merge() -> Result<()> {
    // TODO: roll_back未实现
    Ok(())
}
#[test]
fn test_commit_thread_safety() -> Result<()> {
    // TODO: 多线程未实现
    Ok(())
}
#[test]
fn test_force_commit() -> Result<()> {
    // TODO: open_if_change 未实现
    Ok(())
}
#[test]
fn test_future_commit() -> Result<()> {
    // TODO: ReaderCommit未实现
    Ok(())
}

#[test]
fn test_zero_commits() -> Result<()> {
    // TODO: ReaderCommit未实现
    Ok(())
}
#[test]
fn test_prepare_commit() -> Result<()> {
    // TODO: open_if_changed 未实现
    Ok(())
}

#[test]
fn test_prepare_commit_rollback() -> Result<()> {
    // TODO: open_if_changed 未实现
    Ok(())
}
#[test]
fn test_prepare_commit_no_changes() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    let mock = MockAnalyzer::new(&mut random);
    let iwc = new_index_writer_config_with_analyzer(&mut random, mock);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    writer.prepare_commit()?;
    writer.commit()?;
    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    assert_eq!(0, reader.num_docs()?);

    Ok(())
}
