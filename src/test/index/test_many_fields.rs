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
use crate::core::document::field_type::FieldType;
use crate::core::document::text_field::text_field_type;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::util::error::lucene_error::Result;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, create_temp_dir, new_directory, new_field, new_fs_directory, new_index_writer_config,
    new_searcher_with_reader, random,
};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestManyFields;
#[test]
fn test_many_fields() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现 MockAnalyzer
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_max_buffered_docs(10);

    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    let mut stored_text_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    stored_text_type.freeze();
    for j in 0..100 {
        let mut doc = Document::new();

        doc.add(new_field(
            format!("a{}", j),
            format!("aaa{}", j),
            &stored_text_type,
            &mut field_types,
        )?);
        doc.add(new_field(
            format!("b{}", j),
            format!("aaa{}", j),
            &stored_text_type,
            &mut field_types,
        )?);
        doc.add(new_field(
            format!("c{}", j),
            format!("aaa{}", j),
            &stored_text_type,
            &mut field_types,
        )?);
        doc.add(new_field(
            format!("d{}", j),
            "aaa",
            &stored_text_type,
            &mut field_types,
        )?);
        doc.add(new_field(
            format!("e{}", j),
            "aaa",
            &stored_text_type,
            &mut field_types,
        )?);
        doc.add(new_field(
            format!("f{}", j),
            "aaa",
            &stored_text_type,
            &mut field_types,
        )?);
        writer.add_document(doc)?;
    }

    writer.close()?;

    let reader = directory_reader_util::open(dir.clone())?;
    assert_eq!(100, reader.max_doc()?);
    assert_eq!(100, reader.num_docs()?);
    for j in 0..100 {
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("a{}", j), format!("aaa{}", j)))?
        );
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("b{}", j), format!("aaa{}", j)))?
        );
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("c{}", j), format!("aaa{}", j)))?
        );
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("d{}", j), "aaa"))?
        );
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("e{}", j), "aaa"))?
        );
        assert_eq!(
            1,
            reader.doc_freq(&Term::from_text(format!("f{}", j), "aaa"))?
        );
    }

    Ok(())
}
#[test]
fn test_diverse_docs() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_ram_buffer_size_mb(0.5);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut field_types = HashMap::new();
    let mut stored_text_type = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    stored_text_type.freeze();

    let n = at_least(&mut random, 1);

    for _ in 0..n {
        for _j in 0..100 {
            // First, docs where every term is unique (heavy on
            // Posting instances)
            let mut doc = Document::new();
            for _k in 0..100 {
                doc.add(new_field(
                    "field",
                    random.random::<i32>().to_string(),
                    &stored_text_type,
                    &mut field_types,
                )?);
            }
            writer.add_document(doc)?;
        }
        // Next, many single term docs where only one term
        // occurs (heavy on byte blocks)
        for _j in 0..100 {
            let mut doc = Document::new();
            doc.add(new_field(
                "field",
                "aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa",
                &stored_text_type,
                &mut field_types,
            )?);
            writer.add_document(doc)?;
        }
        // Next, many single term docs where only one term
        // occurs but the terms are very long (heavy on
        // char[] arrays)
        for j in 0..100 {
            let x = format!("{}.", j);
            let long_term = x.repeat(1000);
            let mut doc = Document::new();
            doc.add(new_field(
                "field",
                long_term,
                &stored_text_type,
                &mut field_types,
            )?);
            writer.add_document(doc)?;
        }
    }

    writer.close()?;

    let reader = Arc::new(directory_reader_util::open(dir.clone())?);
    let _searcher = new_searcher_with_reader(reader.clone())?;
    // TODO IndexSearcher#count() 未实现
    // let total_hits = searcher.count(&TermQuery::new(Term::from_text("field", "aaa")))?;
    // assert_eq!(n * 100, total_hits);

    Ok(())
}
// TODO memory calculation not implemented
fn test_rotating_field_names() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_fs_directory(&mut random, create_temp_dir()?)?);
    let mut iwc = new_index_writer_config(&mut random);
    iwc.set_ram_buffer_size_mb(0.2);
    iwc.set_max_buffered_docs(-1);

    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let mut field_types = HashMap::new();

    let mut upto: i32 = 0;

    let mut ft = FieldType::from_ref(&*text_field_type::TYPE_NOT_STORED)?;
    ft.set_omit_norms(true)?;

    let mut first_doc_count: i32 = -1;

    for iter in 0..10 {
        let start_flush_count = writer.get_flush_count();

        let mut doc_count = 0;

        while writer.get_flush_count() == start_flush_count {
            let mut doc = Document::new();
            for _ in 0..10 {
                let field_name = format!("field{}", upto);
                upto += 1;
                doc.add(new_field(field_name, "content", &ft, &mut field_types)?);
            }

            writer.add_document(doc)?;
            doc_count += 1;
        }

        if iter == 0 {
            first_doc_count = doc_count;
        }

        let ratio = (doc_count as f32) / (first_doc_count as f32);
        assert!(
            ratio > 0.9,
            "flushed after too few docs: first segment flushed at docCount={}, \
current segment flushed after docCount={}, iter={} (ratio={})",
            first_doc_count,
            doc_count,
            iter,
            ratio,
        );

        if upto > 5000 {
            upto = 0;
        }
    }

    writer.close()?;
    Ok(())
}
