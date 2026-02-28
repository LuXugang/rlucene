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
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::term::Term;
use crate::core::search::index_searcher::IndexSearcher;
use crate::core::search::regexp_query::RegexpQuery;
use crate::core::util::error::lucene_error::Result;
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::DefaultIndexSearch;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least, new_directory_shared, new_field, new_index_writer_config, new_searcher_with_reader,
    random,
};
use crate::test::util::test_util::TestUtil;
use rand::Rng;
use std::collections::HashMap;

#[allow(dead_code)] // for quick search
pub struct TestRegexpRandom;
fn set_up<R: Rng + ?Sized>(random: &mut R) -> Result<DefaultIndexSearch> {
    let dir = new_directory_shared(random)?;

    let mut config = new_index_writer_config(random);
    config.set_max_buffered_docs(TestUtil::next_int(random, 50, 1000));

    let writer = RandomIndexWriter::with_config(random, dir.clone(), config);
    let mut field_to_type = HashMap::new();

    let mut doc = Document::new();

    let mut custom_type = FieldType::from_ref(&*text_field_type::TYPE_STORED)?;
    custom_type.set_omit_norms(true)?;

    for i in 0..1000 {
        let mut field = new_field("field", "", &custom_type, &mut field_to_type)?;
        let s = format!("{:03}", i);
        field.set_string_value(s)?;
        doc.add(field);
        writer.add_document(doc)?;
        doc = Document::new();
    }

    let reader = writer.get_reader()?;
    writer.close()?;
    new_searcher_with_reader(reader)
}
fn n<R: Rng + ?Sized>(random: &mut R) -> char {
    (0x30u8 + random.random_range(0..10) as u8) as char
}

fn fill_pattern<R: Rng + ?Sized>(random: &mut R, wildcard_pattern: &str) -> String {
    let mut sb = String::new();
    for ch in wildcard_pattern.chars() {
        match ch {
            'N' => sb.push(n(random)),
            _ => sb.push(ch),
        }
    }
    sb
}

fn assert_pattern_hits<IRC, R: Rng + ?Sized>(
    random: &mut R,
    searcher: &IndexSearcher<IRC>,
    pattern: &str,
    num_hits: usize,
) -> Result<()>
where
    IRC: IndexReaderContext,
{
    let wq = RegexpQuery::new(Term::from_text("field", fill_pattern(random, pattern)))?;
    let docs = searcher.search(wq, 25)?;
    assert_eq!(
        num_hits,
        docs.total_hits.value(),
        "Incorrect hits for pattern: {}",
        pattern
    );
    Ok(())
}
// TODO IMPORTANT 测试未完成
fn test_regexps() -> Result<()> {
    let mut random = random();
    let searcher = set_up(&mut random)?;

    let num = at_least(&mut random, 1);
    for _ in 0..num {
        assert_pattern_hits(&mut random, &searcher, "NNN", 1)?;
        assert_pattern_hits(&mut random, &searcher, ".NN", 10)?;
        assert_pattern_hits(&mut random, &searcher, "N.N", 10)?;
        assert_pattern_hits(&mut random, &searcher, "NN.", 10)?;
    }

    for _ in 0..num {
        assert_pattern_hits(&mut random, &searcher, ".{1,2}N", 100)?;
        assert_pattern_hits(&mut random, &searcher, "N.{1,2}", 100)?;
        assert_pattern_hits(&mut random, &searcher, ".{1,3}", 1000)?;

        assert_pattern_hits(&mut random, &searcher, "NN[3-7]", 5)?;
        assert_pattern_hits(&mut random, &searcher, "N[2-6][3-7]", 25)?;
        assert_pattern_hits(&mut random, &searcher, "[1-5][2-6][3-7]", 125)?;
        assert_pattern_hits(&mut random, &searcher, "[0-4][3-7][4-8]", 125)?;
        assert_pattern_hits(&mut random, &searcher, "[2-6][0-4]N", 25)?;
        assert_pattern_hits(&mut random, &searcher, "[2-6]NN", 5)?;

        assert_pattern_hits(&mut random, &searcher, "NN.*", 10)?;
        assert_pattern_hits(&mut random, &searcher, "N.*", 100)?;
        assert_pattern_hits(&mut random, &searcher, ".*", 1000)?;

        assert_pattern_hits(&mut random, &searcher, ".*NN", 10)?;
        assert_pattern_hits(&mut random, &searcher, ".*N", 100)?;

        assert_pattern_hits(&mut random, &searcher, "N.*N", 10)?;

        // combo of ? and * operators
        assert_pattern_hits(&mut random, &searcher, ".N.*", 100)?;
        assert_pattern_hits(&mut random, &searcher, "N..*", 100)?;

        assert_pattern_hits(&mut random, &searcher, ".*N.", 100)?;
        assert_pattern_hits(&mut random, &searcher, ".*..", 1000)?;
        assert_pattern_hits(&mut random, &searcher, ".*.N", 100)?;
    }

    Ok(())
}
