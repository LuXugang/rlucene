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
use crate::core::document::text_field::text_field_type;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::leaf_reader::{LRPosting, LeafReader};
use crate::core::index::postings_enum::{ALL, PostingsEnum};
use crate::core::index::terms::Terms;
use crate::core::index::terms_enum::TermsEnum;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::util::error::lucene_error::Result;
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_bytes_ref_from_string, new_directory, new_field, new_index_writer_config, random,
};
use rand::Rng;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestDocsAndPositions;

fn field_name<R: Rng + ?Sized>(random: &mut R) -> String {
    let v: i32 = random.random();
    format!("field{}", v)
}
/// Simple testcase for ``[`PostingsEnum`]
#[test]
fn test_positions_simple() -> Result<()> {
    let mut random = random();
    let directory = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let config = new_index_writer_config(&mut random);
    let writer = RandomIndexWriter::with_config(&mut random, directory, config);
    let field_name = field_name(&mut random);

    for _ in 0..39 {
        let mut doc = Document::new();
        let mut custom_type = FieldType::from_ref(&text_field_type::TYPE_NOT_STORED.clone())?;
        custom_type.set_omit_norms(true)?;

        let text = concat!(
            "1 2 3 4 5 6 7 8 9 10 ",
            "1 2 3 4 5 6 7 8 9 10 ",
            "1 2 3 4 5 6 7 8 9 10 ",
            "1 2 3 4 5 6 7 8 9 10"
        );

        doc.add(new_field(&field_name, text, &custom_type)?);
        writer.add_document(doc)?;
    }

    let reader = Arc::new(writer.get_reader()?);
    writer.close()?;

    let num = at_least(&mut random, 13);
    for _ in 0..num {
        let bytes = new_bytes_ref_from_string(&mut random, "1")?;
        let top_reader_context = get_context(reader.clone())?;

        for leaf_reader_context in top_reader_context.leaves()? {
            let leaf_reader = leaf_reader_context.reader();

            let mut docs_and_pos_enum = get_docs_and_positions(leaf_reader, &field_name, &bytes)?
                .expect("postings enum must exist");
            let max_doc = leaf_reader.max_doc()?;
            if max_doc == 0 {
                continue;
            }

            let target = random.random_range(0..max_doc);
            let advance_doc = docs_and_pos_enum.advance(target)?;

            loop {
                let msg = format!(
                    "Advanced to {} current doc {}",
                    advance_doc,
                    docs_and_pos_enum.doc_id()
                );

                assert_eq!(docs_and_pos_enum.freq()?, 4, "{msg}");
                assert_eq!(docs_and_pos_enum.next_position()?, 0, "{msg}");

                assert_eq!(docs_and_pos_enum.freq()?, 4, "{msg}");
                assert_eq!(docs_and_pos_enum.next_position()?, 10, "{msg}");

                assert_eq!(docs_and_pos_enum.freq()?, 4, "{msg}");
                assert_eq!(docs_and_pos_enum.next_position()?, 20, "{msg}");

                assert_eq!(docs_and_pos_enum.freq()?, 4, "{msg}");
                assert_eq!(docs_and_pos_enum.next_position()?, 30, "{msg}");

                if docs_and_pos_enum.next_doc()? == NO_MORE_DOCS {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// this test indexes random numbers within a range into a field and checks their occurrences by
/// searching for a number from that range selected at random. All positions for that number are
/// saved up front and compared to the enums positions.
fn test_random_positions() -> Result<()> {
    let mut random = random();

    // directory + writer (Arc + your config style)
    let directory = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    // TODO: newLogMergePolicy未实现
    let config = new_index_writer_config(&mut random);
    let writer = RandomIndexWriter::with_config(&mut random, directory.clone(), config);

    let field_name = field_name(&mut random);

    // let num_docs = at_least(&mut random, 47);
    let num_docs = 62;
    let max = 1051;
    // let term: i32 = random.random_range(0..max);
    let term: i32 = 500;

    let mut positions_in_doc: Vec<Vec<i32>> = vec![Vec::new(); num_docs as usize];

    let mut custom_type = FieldType::from_ref(&text_field_type::TYPE_NOT_STORED.clone())?;
    custom_type.set_omit_norms(true)?;

    for i in 0..num_docs {
        let mut doc = Document::new();
        let mut positions = Vec::new();

        // let num = at_least(&mut random, 131);
        let num = 189;

        let mut builder = String::new();
        for j in 0..num {
            // let next_int: i32 = random.random_range(0..max);
            let next_int: i32 = j + 100;
            builder.push_str(&format!("{} ", next_int));
            if next_int == term {
                positions.push(j);
            }
        }

        if positions.is_empty() {
            builder.push_str(&format!("{}", term));
            positions.push(num);
        }

        // doc.add(new_field(&field_name, builder, &custom_type)?);
        doc.add(Field::new(&field_name, custom_type.clone(), builder));
        positions_in_doc[i as usize] = positions;

        writer.add_document(doc)?;
    }

    let reader = Arc::new(writer.get_reader()?);
    writer.close()?;

    let num_outer = at_least(&mut random, 13);
    // let num_outer = 1;

    for i in 0..num_outer {
        let bytes = new_bytes_ref_from_string(&mut random, &format!("{}", term))?;
        let top_reader_context = get_context(reader.clone())?;

        for leaf_ctx in top_reader_context.leaves()? {
            let leaf_reader = leaf_ctx.reader();
            let mut docs_and_pos_enum = get_docs_and_positions(leaf_reader, &field_name, &bytes)?
                .expect("postings enum must exist");

            let max_doc = leaf_reader.max_doc()?;
            if max_doc == 0 {
                continue;
            }
            // initially advance or do next doc
            let init_doc = if random.random_bool(0.5) {
                docs_and_pos_enum.next_doc()?
            } else {
                docs_and_pos_enum.advance(random.random_range(0..max_doc))?
            };
            // now run through the scorer and check if all positions are there...
            loop {
                let doc_id = docs_and_pos_enum.doc_id();
                if doc_id == NO_MORE_DOCS {
                    break;
                }

                let global_doc = leaf_ctx.doc_base + doc_id;
                let pos = &positions_in_doc[global_doc as usize];

                assert_eq!(pos.len() as i32, docs_and_pos_enum.freq()?,);

                let read_all = random.random_range(0..20) != 0;
                // number of positions read should be random - don't read all of them
                // allways
                // let how_many = pos.len();
                let how_many = if read_all {
                    pos.len()
                } else {
                    let remain = pos.len();
                    remain - random.random_range(0..remain)
                };

                for j in 0..how_many {
                    let expected = pos[j];
                    let actual = docs_and_pos_enum.next_position()?;
                    assert_eq!(
                        expected, actual,
                        "iteration {i}, initDoc={init_doc}, doc={doc_id}, base={}, positions={:?}",
                        leaf_ctx.doc_base, pos
                    );
                }

                if random.random_range(0..10) == 0 {
                    // once is a while advance
                    let advance_target = doc_id + 1 + random.random_range(0..(max_doc - doc_id));
                    if docs_and_pos_enum.advance(advance_target)? == NO_MORE_DOCS {
                        break;
                    }
                }

                if docs_and_pos_enum.next_doc()? == NO_MORE_DOCS {
                    break;
                }
            }
        }
    }

    Ok(())
}

fn get_docs_and_positions<LR>(
    reader: &LR,
    field_name: &str,
    bytes: &BytesRef<Vec<u8>>,
) -> Result<Option<LRPosting<LR>>>
where
    LR: LeafReader,
{
    let terms_opt = reader.terms(field_name)?;
    let terms = match terms_opt {
        None => return Ok(None),
        Some(t) => t,
    };

    let mut te = terms.iterator()?;

    if te.seek_exact(bytes)? {
        let pe = te.postings_with_flags(None, ALL as i32)?;
        Ok(Some(pe))
    } else {
        Ok(None)
    }
}
