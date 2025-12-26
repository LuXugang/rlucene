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
use crate::core::document::int_point::IntPoint;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_writer::IndexWriter;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory_shared, new_index_writer_config, random,
};

#[allow(dead_code)] // for quick search
pub struct TestPointQueries;

#[test]
fn test_basic_ints() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;
    // TODO: 未实现MockAnalyzer
    let w = IndexWriter::new(dir.clone(), new_index_writer_config(&mut random))?;

    {
        let mut doc = Document::new();
        doc.add(IntPoint::new("point", [-7])?);
        w.add_document(doc)?;
    }

    {
        let mut doc = Document::new();
        doc.add(IntPoint::new("point", [0])?);
        w.add_document(doc)?;
    }

    {
        let mut doc = Document::new();
        doc.add(IntPoint::new("point", [3])?);
        w.add_document(doc)?;
    }

    let r = directory_reader_util::open_with_writer(&w)?;
    let searcher = IndexSearcher::new(get_context(&r)?)?;

    assert_eq!(
        2,
        searcher.count(IntPoint::new_point_range_query("point", [-8], [1])?)?
    );

    assert_eq!(
        3,
        searcher.count(IntPoint::new_point_range_query("point", [-7], [3])?)?
    );

    assert_eq!(
        1,
        searcher.count(IntPoint::new_exact_query("point", [-7])?)?
    );

    assert_eq!(
        0,
        searcher.count(IntPoint::new_exact_query("point", [-6])?)?
    );
    w.close()?;
    Ok(())
}
