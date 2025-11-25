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
use crate::core::document::field::Field;
use crate::core::document::field_type::FieldType;
use crate::core::index::index_options::IndexOptions;
use crate::core::index::index_writer::{IndexWriter, MAX_TERM_LENGTH};
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    DirType, new_directory, new_index_writer_config, random,
};
use crate::test::util::test_util::TestUtil;
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestExceedMaxTermLength;

const MIN_TEST_TERM_LENGTH: i32 = MAX_TERM_LENGTH + 1;
const MAX_TEST_TERM_LENGTH: i32 = MAX_TERM_LENGTH * 2;
fn create_dir<R: Rng + ?Sized>(random: &mut R) -> DirType {
    new_directory(random).unwrap()
}

// TODO IMPORTANT MockAnalyzer 未实现
fn test_token_stream() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(create_dir(&mut random));

    // TODO: MockAnalyzer 未实现
    let iwc = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut ft = FieldType::new();
    ft.set_index_options(IndexOptions::DocsAndFreqsAndPositions)?;
    ft.set_stored(random.random_bool(0.5))?;
    ft.freeze();

    let mut doc = Document::new();

    if random.random_bool(0.5) {
        doc.add(Field::new(
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            ft.clone(),
        ));
    }

    let name = TestUtil::random_simple_string_range(&mut random, 1, 50);
    let value = TestUtil::random_simple_string_range(
        &mut random,
        MIN_TEST_TERM_LENGTH as usize,
        MAX_TEST_TERM_LENGTH as usize,
    );
    let f = Field::new(name.clone(), value, ft.clone());

    if random.random_bool(0.5) {
        doc.add(Field::new(
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            ft.clone(),
        ));
    }

    doc.add(f);

    let res = writer.add_document(doc);

    match res {
        Err(LuceneError::IllegalArgument(msg)) => {
            let error_msg = &msg.message;

            let max_len = MAX_TERM_LENGTH.to_string();

            assert!(
                error_msg.contains("immense term"),
                "IllegalArgumentException didn't mention 'immense term': {}",
                error_msg
            );

            assert!(
                error_msg.contains(&max_len),
                "IllegalArgumentException didn't mention max length ({}): {}",
                max_len,
                error_msg
            );

            assert!(
                error_msg.contains(&name),
                "IllegalArgumentException didn't mention field name ({}): {}",
                name,
                error_msg
            );

            assert!(
                error_msg.contains("bytes can be at most") && error_msg.contains("in length; got"),
                "IllegalArgumentException didn't mention original message: {}",
                error_msg
            );
        },
        other => {
            assert!(false, "expected IllegalArgument but got {:?}", other);
        },
    }

    writer.close()?;
    Ok(())
}
// TODO IMPORTANT MockAnalyzer 未实现
fn test_binary_value() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(create_dir(&mut random));
    let iwc = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut ft = FieldType::new();
    let opts = if random.random_bool(0.5) {
        IndexOptions::Docs
    } else {
        IndexOptions::DocsAndFreqs
    };
    ft.set_index_options(opts)?;
    ft.set_stored(random.random_bool(0.5))?;
    ft.set_tokenized(false)?;
    ft.freeze();

    let mut doc = Document::new();

    if random.random_bool(0.5) {
        doc.add(Field::with_bytes_ref(
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            TestUtil::random_binary_term_with_len(&mut random, 10),
            ft.clone(),
        )?);
    }

    // problematic field
    let name = TestUtil::random_simple_string_range(&mut random, 1, 50);
    let len = TestUtil::next_int(&mut random, MIN_TEST_TERM_LENGTH, MAX_TEST_TERM_LENGTH) as usize;
    let value = TestUtil::random_binary_term_with_len(&mut random, len);

    let f = Field::with_bytes_ref(name.clone(), value, ft.clone())?;

    if random.random_bool(0.5) {
        doc.add(Field::with_bytes_ref(
            TestUtil::random_simple_string_range(&mut random, 1, 10),
            TestUtil::random_binary_term_with_len(&mut random, 10),
            ft.clone(),
        )?);
    }

    doc.add(f);

    // expect error
    let res = writer.add_document(doc);

    match res {
        Err(LuceneError::IllegalArgument(msg)) => {
            let error_msg = &msg.message;
            let max_len = MAX_TERM_LENGTH.to_string();

            assert!(
                error_msg.contains("immense term"),
                "IllegalArgumentException didn't mention 'immense term': {}",
                error_msg
            );

            assert!(
                error_msg.contains(&max_len),
                "IllegalArgumentException didn't mention max length ({}): {}",
                max_len,
                error_msg
            );

            assert!(
                error_msg.contains(&name),
                "IllegalArgumentException didn't mention field name ({}): {}",
                name,
                error_msg
            );

            assert!(
                error_msg.contains("bytes can be at most") && error_msg.contains("in length; got"),
                "IllegalArgumentException didn't mention original message: {}",
                error_msg
            );
        },
        other => {
            unreachable!("expected IllegalArgument but got {:?}", other);
        },
    }

    writer.close()?;
    Ok(())
}
