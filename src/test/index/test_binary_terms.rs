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
use crate::core::document::field_type::FieldType;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::term_query::TermQuery;
use crate::core::search::top_docs::TopDocsLike;
use crate::core::util::error::lucene_error::Result;
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    new_directory, new_field, new_searcher, new_searcher_with_reader, new_string_field_binary,
    random,
};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestBinaryTerms;
#[test]
fn test_binary() -> Result<()> {
    let mut random = random();
    let directory = Arc::new(new_directory(&mut random)?);

    let iw = RandomIndexWriter::new(&mut random, directory.clone());
    let mut field_types = HashMap::new();
    let mut bytes: BytesRef<Vec<u8>> = BytesRef::with_capacity(2);

    for i in 0..256u16 {
        bytes.bytes[0] = i as u8;
        bytes.bytes[1] = (255 - i) as u8;
        bytes.length = 2;

        let mut doc = Document::new();

        let mut custom_type = FieldType::default();
        custom_type.set_stored(true)?;
        doc.add(new_field(
            "id",
            i.to_string(),
            &custom_type,
            &mut field_types,
        )?);
        doc.add(new_string_field_binary(
            "bytes",
            bytes.clone(),
            No,
            &mut field_types,
        )?);

        iw.add_document(doc)?;
    }

    let ir = Arc::new(iw.get_reader()?);
    let is = new_searcher_with_reader(ir)?;

    for i in 0..256u16 {
        bytes.bytes[0] = i as u8;
        bytes.bytes[1] = (255 - i) as u8;
        bytes.length = 2;
        let term = Term::new("bytes", bytes.clone());
        let query = TermQuery::new(term);
        let docs = is.search(query, 5)?;
        assert_eq!(docs.total_hits().value(), 1);
        let v = is
            .stored_fields()?
            .document(docs.score_docs[0].doc)?
            .get("id")?
            .unwrap()
            .to_string();
        assert_eq!(v, i.to_string());
    }

    Ok(())
}
#[test]
fn test_to_string() -> Result<()> {
    let bytes = BytesRef::from_bytes(vec![0xffu8, 0xfeu8]);
    let term = Term::new("foo", bytes);
    assert_eq!("foo:[ff fe]", term.to_string());
    Ok(())
}
