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
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    get_only_leaf_reader, new_directory, new_index_writer_config, random,
};
use std::sync::Arc;

#[allow(dead_code)] // for quick search
struct TestDocValuesIndexing;

#[test]
fn test_add_indexes() -> Result<()> {
    // TODO IndexWriter#add_indexes 未实现
    Ok(())
}
#[test]
fn test_multi_valued_doc_values_field() -> Result<()> {
    let mut random = random();

    let d = Arc::new(new_directory(&mut random)?);
    let config = new_index_writer_config(&mut random);
    let w = RandomIndexWriter::with_config(&mut random, d.clone(), config);

    let mut doc = Document::new();
    let f = NumericDocValuesField::new("field", 17);
    doc.add(f.clone());

    w.add_document(doc.clone())?;

    doc.add(f.clone());
    // Index doc values are single-valued so we should not
    // be able to add same field more than once:
    let res = w.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let r = Arc::new(w.get_reader()?);
    w.close()?;

    let leaf = get_only_leaf_reader(r.clone())?;
    let values_opt = leaf.get_numeric_doc_values("field")?;
    assert!(values_opt.is_some());
    let mut values = values_opt.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(17, values.long_value()?);

    Ok(())
}
