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
use crate::core::document::field::Store::No;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::IndexWriterConfig;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, create_temp_dir, new_fs_directory, new_text_field, random,
};
use crate::test::util::test_util::TestUtil;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;

#[allow(dead_code)] // for quick search
struct TestIndexManyDocuments;
// TODO 测试未通过
fn test_threaded_indexing() -> Result<()> {
    let mut random = random();

    let dir = new_fs_directory(&mut random, create_temp_dir()?)?;

    let mut iwc = IndexWriterConfig::new();
    let max_buffered_docs = TestUtil::next_int(&mut random, 100, 2000);
    iwc.set_max_buffered_docs(max_buffered_docs);

    let num_docs = at_least(&mut random, 10000);

    let writer = Arc::new(IndexWriter::new(dir.clone(), iwc)?);

    let counter = Arc::new(AtomicI32::new(0));
    let mut threads = Vec::new();

    // TODO IMPORTANT 这里使用多线程的测试未通过
    let shared_field_types = Arc::new(Mutex::new(HashMap::new()));
    for _ in 0..1 {
        let writer = writer.clone();
        let counter_cloned = counter.clone();
        let field_types = shared_field_types.clone();

        threads.push(thread::spawn(move || {
            loop {
                let curr = counter_cloned.fetch_add(1, Ordering::SeqCst);
                if curr >= num_docs {
                    break;
                }

                let mut doc = Document::new();
                doc.add(new_text_field("field", "text", No, &mut field_types.lock()).unwrap());

                if let Err(e) = writer.add_document(doc) {
                    panic!("thread indexing failed: {:?}", e);
                }
            }
        }));
    }

    for t in threads {
        t.join().expect("thread panicked");
    }

    let stats = writer.get_doc_stats()?;
    assert_eq!(
        num_docs,
        stats.max_doc,
        "lost {} documents; maxBufferedDocs={}",
        num_docs - stats.max_doc,
        max_buffered_docs
    );

    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    assert_eq!(num_docs, reader.max_doc()?);

    Ok(())
}
