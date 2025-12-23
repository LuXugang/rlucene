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
use crate::core::analysis::analyzer::Analyzer;
use crate::core::analysis::reader::ReaderEnum;
use crate::core::analysis::token_stream::{Either2TokenStream, InnerTokenStreams};
use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
use crate::core::document::document::Document;
use crate::core::document::field::Store::No;
use crate::core::document::field::{Field, FieldBase, FieldDataEnum};
use crate::core::document::field_type::FieldType;
use crate::core::document::invertable_field::InvertableType;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::string_field::{StringField, string_field_type};
use crate::core::index::BytesRef;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_writer::IndexWriter;
use crate::core::index::index_writer_config::OpenMode::Create;
use crate::core::index::index_writer_config::{IndexWriterConfig, OpenMode};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::number::Number;
use crate::test::index::random_index_writer::RandomIndexWriter;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    get_only_leaf_reader, new_bytes_ref_from_bytes, new_bytes_ref_from_string, new_directory,
    new_index_writer_config, random,
};
use rand::RngCore;
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
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

    let r = w.get_reader()?;
    w.close()?;

    let leaf = get_only_leaf_reader(r)?;
    let values_opt = leaf.get_numeric_doc_values("field")?;
    assert!(values_opt.is_some());
    let mut values = values_opt.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(17, values.long_value()?);

    Ok(())
}
#[test]
fn test_different_typed_doc_values_field() -> Result<()> {
    let mut random = random();

    // directory + writer
    let d = Arc::new(new_directory(&mut random)?);
    let config = new_index_writer_config(&mut random);
    let w = RandomIndexWriter::with_config(&mut random, d.clone(), config);

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("field", 17));
    w.add_document(doc.clone())?;

    // Index doc values are single-valued so we should not
    // be able to add same field more than once:
    doc.add(BinaryDocValuesField::new(
        "field",
        new_bytes_ref_from_string(&mut random, "blah")?,
    ));

    let res = w.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument for mixed doc-values types, got: {:?}",
        res
    );

    let r = w.get_reader()?;
    w.close()?;

    let leaf = get_only_leaf_reader(r)?;
    let values_opt = leaf.get_numeric_doc_values("field")?;
    assert!(values_opt.is_some());

    let mut values = values_opt.unwrap();
    assert_eq!(0, values.next_doc()?);
    assert_eq!(17, values.long_value()?);

    Ok(())
}
#[test]
fn test_different_typed_doc_values_field2() -> Result<()> {
    let mut random = random();

    let d = Arc::new(new_directory(&mut random)?);
    let config = new_index_writer_config(&mut random);
    let w = RandomIndexWriter::with_config(&mut random, d.clone(), config);

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("field", 17));
    w.add_document(doc.clone())?;
    // Index doc values are single-valued so we should not
    // be able to add same field more than once:
    doc.add(SortedDocValuesField::new(
        "field",
        new_bytes_ref_from_string(&mut random, "hello")?,
    ));

    let res = w.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let r = w.get_reader()?;

    let leaf = get_only_leaf_reader(r)?;
    let values_opt = leaf.get_numeric_doc_values("field")?;
    assert!(values_opt.is_some());
    let mut values = values_opt.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(17, values.long_value()?);

    w.close()?;

    Ok(())
}
#[test]
fn test_length_prefix_across_two_pages() -> Result<()> {
    let mut random = random();

    let d = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let config = IndexWriterConfig::new();
    let w = IndexWriter::new(d.clone(), config)?;

    let mut doc = Document::new();

    let mut bytes = vec![0u8; 32_764];
    let mut b = BytesRef::from_bytes(bytes.clone());
    doc.add(SortedDocValuesField::new("field", b));
    w.add_document(doc.clone())?;

    bytes[0] = 1;
    b = BytesRef::from_bytes(bytes.clone());
    doc = Document::new();
    doc.add(SortedDocValuesField::new("field", b.clone()));
    w.add_document(doc)?;
    // TODO force_merge未实现
    let r = directory_reader_util::open_with_writer(&w)?;

    let leaf = get_only_leaf_reader(r)?;
    let mut s = leaf
        .get_sorted_doc_values("field")?
        .expect("sorted doc values must exist");

    assert_eq!(0, s.next_doc()?);
    let ord = s.ord_value()?;
    let mut bytes1 = s.lookup_ord(ord)?;

    assert_eq!(bytes.len(), bytes1.length);

    bytes[0] = 0;
    let b0 = BytesRef::from_bytes(bytes.clone());
    assert_eq!(&b0, bytes1.as_ref());

    assert_eq!(1, s.next_doc()?);
    let ord2 = s.ord_value()?;
    bytes1 = s.lookup_ord(ord2)?;
    assert_eq!(bytes.len(), bytes1.length);

    bytes[0] = 1;
    let b1 = BytesRef::from_bytes(bytes.clone());
    assert_eq!(&b1, bytes1.as_ref());

    w.close()?;

    Ok(())
}
#[test]
fn test_doc_values_unstored() -> Result<()> {
    // TODO FieldInfos.get_merged_field_infos 未实现
    Ok(())
}
#[test]
fn test_mixed_types_same_document() -> Result<()> {
    let mut random = random();
    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let config = new_index_writer_config(&mut random);
    let w = IndexWriter::new(dir.clone(), config)?;

    w.add_document(Document::new())?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 0));
    doc.add(SortedDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "hello")?,
    ));

    let res = w.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&w)?);
    assert_eq!(1, ir.num_docs()?);

    w.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_different_documents() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let config = new_index_writer_config(&mut random);
    let w = IndexWriter::new(dir.clone(), config)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 0));
    w.add_document(doc)?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "hello")?,
    ));

    let res = w.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&w)?);
    assert_eq!(1, ir.num_docs()?);

    w.close()?;

    Ok(())
}
#[test]
fn test_add_sorted_twice() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let directory = Arc::new(new_directory(&mut random)?);

    let iwc = new_index_writer_config(&mut random);
    // TODO: newLogMergePolicy 未实现
    let iwriter = IndexWriter::new(directory.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo!")?,
    ));
    iwriter.add_document(doc.clone())?;

    doc.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "bar!")?,
    ));

    let res = iwriter.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&iwriter)?);
    assert_eq!(1, ir.num_docs()?);
    iwriter.close()?;

    Ok(())
}
#[test]
fn test_add_binary_twice() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let directory = Arc::new(new_directory(&mut random)?);

    let iwc = new_index_writer_config(&mut random);
    // TODO: newLogMergePolicy 未实现
    let iwriter = IndexWriter::new(directory.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(BinaryDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo!")?,
    ));
    iwriter.add_document(doc.clone())?;

    doc.add(BinaryDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "bar!")?,
    ));

    let res = iwriter.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&iwriter)?);
    assert_eq!(1, ir.num_docs()?);

    iwriter.close()?;

    Ok(())
}
#[test]
fn test_add_numeric_twice() -> Result<()> {
    let mut random = random();

    let directory = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let iwc = new_index_writer_config(&mut random);
    // TODO: newLogMergePolicy 未实现
    let iwriter = IndexWriter::new(directory.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", 1));
    iwriter.add_document(doc.clone())?;

    doc.add(NumericDocValuesField::new("dv", 2));

    let res = iwriter.add_document(doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&iwriter)?);
    assert_eq!(1, ir.num_docs()?);

    iwriter.close()?;

    Ok(())
}
#[test]
fn test_too_large_sorted_bytes() -> Result<()> {
    let mut random = random();

    let directory = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    // TODO: newLogMergePolicy 未实现
    let iwc = new_index_writer_config(&mut random);
    let iwriter = IndexWriter::new(directory.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "just fine")?,
    ));
    iwriter.add_document(doc.clone())?;

    // huge doc: SortedDocValues too large
    let mut huge_doc = Document::new();
    let mut bytes = vec![0u8; 100_000];
    random.fill_bytes(&mut bytes);
    let b = new_bytes_ref_from_bytes(&mut random, bytes.as_ref())?;

    huge_doc.add(SortedDocValuesField::new("dv", b));

    let res = iwriter.add_document(huge_doc);
    assert!(matches!(res, Err(LuceneError::IllegalArgument(_))));

    let ir = Arc::new(directory_reader_util::open_with_writer(&iwriter)?);
    assert_eq!(1, ir.num_docs()?);

    iwriter.close()?;

    Ok(())
}
#[test]
fn test_too_large_term_sorted_set_bytes() -> Result<()> {
    let mut random = random();

    let directory = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    // TODO: newLogMergePolicy 未实现
    let iwc = new_index_writer_config(&mut random);
    let iwriter = IndexWriter::new(directory.clone(), iwc)?;

    // Initial OK doc
    let mut doc = Document::new();
    doc.add(SortedSetDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "just fine")?,
    ));
    iwriter.add_document(doc.clone())?;

    // Huge doc containing SortedSetDV with very large BytesRef
    let mut huge_doc = Document::new();
    let mut bytes = vec![0u8; 100_000];
    random.fill_bytes(&mut bytes);
    let b = BytesRef::from_bytes(bytes);

    huge_doc.add(SortedSetDocValuesField::new("dv", b));

    let res = iwriter.add_document(huge_doc);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = Arc::new(directory_reader_util::open_with_writer(&iwriter)?);
    assert_eq!(1, ir.num_docs()?);

    iwriter.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_different_segments() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);
    let iwc = new_index_writer_config(&mut random);
    let w = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 0));
    w.add_document(doc)?;
    w.commit()?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "hello")?,
    ));

    let res = w.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    w.close()?;

    Ok(())
}

#[test]
fn test_mixed_types_after_delete_all() -> Result<()> {
    // TODO writer.delete_all未实现
    // let mut random = random();
    //
    // let dir = Arc::new(new_directory(&mut random)?);
    // // TODO: 未实现MockAnalyzer
    // let iwc = new_index_writer_config(&mut random);
    // let w = IndexWriter::new(dir.clone(), iwc)?;
    //
    // let mut doc = Document::new();
    // doc.add(NumericDocValuesField::new("foo", 0));
    // w.add_document(doc)?;
    // w.delete_all()?;
    //
    // let mut doc2 = Document::new();
    // doc2.add(SortedDocValuesField::new(
    //     "foo",
    //     new_bytes_ref_from_string(&mut random, "hello")?,
    // ));
    //
    // w.add_document(doc2)?;
    //
    // w.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_after_reopen_create() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let iwc1 = new_index_writer_config(&mut random);
    {
        let w = IndexWriter::new(dir.clone(), iwc1)?;
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 0));
        w.add_document(doc)?;
        w.close()?;
    }

    let mut iwc2 = new_index_writer_config(&mut random);
    iwc2.set_open_mode(OpenMode::Create);
    let w2 = IndexWriter::new(dir.clone(), iwc2)?;

    let doc2 = Document::new();
    w2.add_document(doc2)?;

    w2.close()?;

    Ok(())
}

#[test]
fn test_mixed_types_after_reopen_append1() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let iwc1 = new_index_writer_config(&mut random);
    {
        let w = IndexWriter::new(dir.clone(), iwc1)?;
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 0));
        w.add_document(doc)?;
        w.close()?;
    }

    let iwc2 = new_index_writer_config(&mut random);
    let w2 = IndexWriter::new(dir.clone(), iwc2)?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "hello")?,
    ));

    let res = w2.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    w2.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_after_reopen_append2() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);

    let iwc1 = new_index_writer_config(&mut random);
    {
        let w = IndexWriter::new(dir.clone(), iwc1)?;
        let mut doc = Document::new();
        doc.add(SortedSetDocValuesField::new(
            "foo",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        w.add_document(doc)?;
        w.close()?;
    }

    let iwc2 = new_index_writer_config(&mut random);
    let w2 = IndexWriter::new(dir.clone(), iwc2)?;

    // Add a field first as StringField (no DV), then as BinaryDV → must error
    let mut doc2 = Document::new();
    doc2.add(StringField::with_string("foo", "bar", No)?);
    doc2.add(BinaryDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));

    let res = w2.add_document(doc2);
    // NOTE: this case follows a different code path inside
    // DefaultIndexingChain/FieldInfos, because the field (foo)
    // is first added without DocValues:
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    // TODO force_merge未实现
    w2.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_after_reopen_append3() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let iwc1 = new_index_writer_config(&mut random);
    {
        let w = IndexWriter::new(dir.clone(), iwc1)?;
        let mut doc = Document::new();
        doc.add(SortedSetDocValuesField::new(
            "foo",
            new_bytes_ref_from_string(&mut random, "foo")?,
        ));
        w.add_document(doc)?;
        w.close()?;
    }

    let iwc2 = new_index_writer_config(&mut random);
    let w2 = IndexWriter::new(dir.clone(), iwc2)?;

    // Add a StringField first (no DV), then BinaryDV → must error
    let mut doc2 = Document::new();
    doc2.add(StringField::with_string("foo", "bar", No)?);
    doc2.add(BinaryDocValuesField::new(
        "foo",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));

    let res = w2.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    // Also add another document to ensure a segment is written
    w2.add_document(Document::new())?;
    // TODO force_merge未实现
    w2.close()?;

    Ok(())
}
#[test]
fn test_mixed_types_different_threads() -> Result<()> {
    // TODO
    Ok(())
}
#[test]
fn test_mixed_types_via_add_indexes() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_illegal_type_change() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);
    let conf = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), conf)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("dv", 0));
    writer.add_document(doc)?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));

    let res = writer.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got: {:?}",
        res
    );

    let ir = directory_reader_util::open_with_writer(&writer)?;
    assert_eq!(1, ir.num_docs()?);

    writer.close()?;

    Ok(())
}
#[test]
fn test_illegal_type_change_across_segments() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);
    let conf1 = new_index_writer_config(&mut random);
    {
        let writer = IndexWriter::new(dir.clone(), conf1)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("dv", 0));
        writer.add_document(doc)?;
        writer.close()?;
    }

    let conf2 = new_index_writer_config(&mut random);
    let writer2 = IndexWriter::new(dir.clone(), conf2)?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));

    let res = writer2.add_document(doc2);
    assert!(
        matches!(res, Err(LuceneError::IllegalArgument(_))),
        "expected IllegalArgument but got {:?}",
        res
    );

    writer2.close()?;

    Ok(())
}
#[test]
fn test_type_change_after_close_and_delete_all() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);

    let conf1 = new_index_writer_config(&mut random);
    {
        let writer = IndexWriter::new(dir.clone(), conf1)?;
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("dv", 0));
        writer.add_document(doc)?;
        writer.close()?;
    }

    let conf2 = new_index_writer_config(&mut random);
    let writer2 = IndexWriter::new(dir.clone(), conf2)?;
    writer2.delete_all()?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));
    writer2.add_document(doc2)?;

    writer2.close()?;

    Ok(())
}

#[test]
fn test_type_change_after_delete_all() -> Result<()> {
    // TODO writer.delete_all未实现
    // let mut random = random();
    //
    // let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    // let conf = new_index_writer_config(&mut random);
    // let writer = IndexWriter::new(dir.clone(), conf)?;
    // let mut doc = Document::new();
    // doc.add(NumericDocValuesField::new("dv", 0));
    // writer.add_document(doc)?;
    //
    // writer.delete_all()?;
    //
    // let mut doc2 = Document::new();
    // doc2.add(SortedDocValuesField::new(
    //     "dv",
    //     new_bytes_ref_from_string(&mut random, "foo")?,
    // ));
    // writer.add_document(doc2)?;
    //
    // writer.close()?;

    Ok(())
}
#[test]
fn test_type_change_after_commit_and_delete_all() -> Result<()> {
    // TODO writer.delete_all未实现
    Ok(())
}
#[test]
fn test_type_change_after_open_create() -> Result<()> {
    let mut random = random();

    let dir = Arc::new(new_directory(&mut random)?);
    // TODO: 未实现MockAnalyzer
    let conf1 = new_index_writer_config(&mut random);
    {
        let writer = IndexWriter::new(dir.clone(), conf1)?;
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("dv", 0));
        writer.add_document(doc)?;
        writer.close()?;
    }

    let mut conf2 = new_index_writer_config(&mut random);
    conf2.set_open_mode(Create);
    let writer2 = IndexWriter::new(dir.clone(), conf2)?;

    let mut doc2 = Document::new();
    doc2.add(SortedDocValuesField::new(
        "dv",
        new_bytes_ref_from_string(&mut random, "foo")?,
    ));
    writer2.add_document(doc2)?;

    writer2.close()?;

    Ok(())
}

#[test]
fn test_type_change_via_add_indexes() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_type_change_via_add_indexes_ir() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_type_change_via_add_indexes_2() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_type_change_via_add_indexes_ir_2() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_same_field_name_for_posting_and_doc_value() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);
    let conf = new_index_writer_config(&mut random);
    let writer = IndexWriter::new(dir.clone(), conf)?;

    let mut doc = Document::new();
    doc.add(StringField::with_string("f", "mock-value", No)?);
    doc.add(NumericDocValuesField::new("f", 5));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc2 = Document::new();
    doc2.add(BinaryDocValuesField::new(
        "f",
        new_bytes_ref_from_string(&mut random, "mock")?,
    ));
    let res = writer.add_document(doc2);
    assert!(matches!(res, Err(LuceneError::IllegalArgument(_))));

    // TODO: rollback未实现
    // writer.rollback()?;
    // TODO: 这里不需要close
    writer.close()?;
    Ok(())
}

#[test]
fn test_exc_indexing_doc_before_doc_values() -> Result<()> {
    let mut random = random();
    // TODO: 未实现MockAnalyzer
    let dir = Arc::new(new_directory(&mut random)?);
    let iwc = new_index_writer_config(&mut random);
    let w = IndexWriter::new(dir.clone(), iwc)?;

    let mut ft = FieldType::from_ref(&*string_field_type::TYPE_NOT_STORED)?;
    ft.set_doc_values_type(DocValuesType::Sorted)?;
    ft.freeze();

    let bytes = BytesRef::from_string("value");
    let field = FieldImpl::new("test", bytes, ft);

    let mut doc = Document::new();
    doc.add(field);

    let res = w.add_document(doc);
    assert!(matches!(res, Err(LuceneError::UnsupportedOperation(_))));

    w.add_document(Document::new())?;
    w.close()?;
    Ok(())
}

pub struct FieldImpl {
    parent_field: Field,
}
impl FieldImpl {
    fn new(name: &str, value: BytesRef<Vec<u8>>, field_type: FieldType) -> Self {
        let parent_field = Field::new(name, value, field_type);
        FieldImpl { parent_field }
    }
}
impl FieldBase for FieldImpl {}

impl Display for FieldImpl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.parent_field)
    }
}

impl IndexableField for FieldImpl {
    fn name(&self) -> &str {
        self.parent_field.name()
    }

    type FieldType = FieldType;

    fn field_type(&self) -> &Self::FieldType {
        self.parent_field.field_type()
    }

    type TokenStream = <Field as IndexableField>::TokenStream;

    fn token_stream<'a>(
        &'a mut self,
        _token_stream: Option<&'a mut InnerTokenStreams>,
    ) -> Result<Option<Either2TokenStream<&'a mut InnerTokenStreams, &'a mut Self::TokenStream>>>
    {
        Err(LuceneError::unsupported_operation(""))
    }

    fn binary_value(&self) -> Result<Option<Cow<'_, BytesRef<Vec<u8>>>>> {
        self.parent_field.binary_value()
    }

    fn take_binary_value(&mut self) -> Result<Option<BytesRef<Vec<u8>>>> {
        self.parent_field.take_binary_value()
    }

    fn string_value(&self) -> Result<Option<Cow<'_, String>>> {
        self.parent_field.string_value()
    }

    fn take_string_value(&mut self) -> Result<Option<String>> {
        self.parent_field.take_string_value()
    }

    fn get_char_sequence_value(&self) -> Result<Option<Cow<'_, String>>> {
        self.parent_field.get_char_sequence_value()
    }

    fn take_reader_value(&mut self) -> Result<Option<ReaderEnum>> {
        self.parent_field.take_reader_value()
    }

    fn numeric_value(&self) -> Result<Option<Number>> {
        self.parent_field.numeric_value()
    }

    fn stored_value(&self) -> Option<&FieldDataEnum> {
        self.parent_field.stored_value()
    }

    fn take_stored_value(&mut self) -> Option<FieldDataEnum> {
        self.parent_field.take_stored_value()
    }

    fn invertable_type(&self) -> &InvertableType {
        self.parent_field.invertable_type()
    }

    fn init_token_stream<A>(&mut self, analyzer: &A) -> Result<()>
    where
        A: Analyzer,
    {
        self.parent_field.init_token_stream(analyzer)
    }
}
#[cfg(test)]
impl Clone for FieldImpl {
    fn clone(&self) -> Self {
        Self {
            parent_field: self.parent_field.clone(),
        }
    }
}
