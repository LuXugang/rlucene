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
use crate::core::document::double_doc_values_field::DoubleDocValuesField;
use crate::core::document::field::Store;
use crate::core::document::float_doc_values_field::FloatDocValuesField;
use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
use crate::core::document::sorted_numeric_doc_values_field::SortedNumericDocValuesField;
use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
use crate::core::document::stored_field::StoredField;
use crate::core::document::string_field::StringField;
use crate::core::index::BytesRef;
use crate::core::index::composite_reader::get_context;
use crate::core::index::directory_reader::directory_reader_util;
use crate::core::index::index_reader::IndexReader;
use crate::core::index::index_reader_context::IndexReaderContext;
use crate::core::index::index_writer::{IndexWriter, SOURCE, SOURCE_FLUSH, SOURCE_MERGE};
use crate::core::index::indexable_field::IndexableField;
use crate::core::index::leaf_reader::LeafReader;
use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
use crate::core::index::multi_doc_values::MultiDocValues;
use crate::core::index::numeric_doc_values::NumericDocValues;
use crate::core::index::sorted_doc_values::SortedDocValues;
use crate::core::index::stored_fields::StoredFields;
use crate::core::index::term::Term;
use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
use crate::core::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::core::search::sort::Sort;
use crate::core::search::sort_field::MissingValueEnum::{StringFirst, StringLast};
use crate::core::search::sort_field::{SortField, SortFieldType, SortFiledBase};
use crate::core::search::sorted_numeric_sort_field::SortedNumericSortField;
use crate::core::search::sorted_set_sort_field::SortedSetSortField;
use crate::core::search::term_query::TermQuery;
use crate::core::util::bit_set::BitSet;
use crate::core::util::bits::Bits;
use crate::core::util::error::lucene_error::{LuceneError, Result};
use crate::core::util::fixed_bit_set::FixedBitSet;
use crate::core::util::numeric_utils::NumericUtils;
use crate::test::analysis::mock_analyzer::MockAnalyzer;
use crate::test::util::lucene_test_case::lucene_test_case_util::{
    at_least_usize, get_only_leaf_reader, new_directory_shared,
    new_index_writer_config_with_analyzer, new_searcher_with_reader, random,
};
use crate::test::util::test_util::TestUtil;
use rand::RngExt;
use std::sync::Arc;

#[allow(dead_code)] // for quick search
pub struct TestIndexSorting;

#[test]
fn test_numeric_already_sorted() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_string_already_sorted() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_multi_valued_numeric_already_sorted() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_multi_valued_string_already_sorted() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_basic_string() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::String)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
        "foo",
        BytesRef::from_string("zzz"),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
        "foo",
        BytesRef::from_string("aaa"),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(SortedDocValuesField::new(
        "foo",
        BytesRef::from_string("mmm"),
    ));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_sorted_doc_values("foo")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    let ord_value = values.ord_value()?;
    assert_eq!("aaa", values.lookup_ord(ord_value)?.utf8_to_string()?);

    assert_eq!(1, values.next_doc()?);
    let ord_value = values.ord_value()?;
    assert_eq!("mmm", values.lookup_ord(ord_value)?.utf8_to_string()?);

    assert_eq!(2, values.next_doc()?);
    let ord_value = values.ord_value()?;
    assert_eq!("zzz", values.lookup_ord(ord_value)?.utf8_to_string()?);
    writer.close()?;
    Ok(())
}

#[test]
fn test_basic_multi_valued_string() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortedSetSortField::new("foo", false)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 3));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("zzz"),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 1));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("aaa"),
    ));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("zzz"),
    ));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("bcg"),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 2));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("mmm"),
    ));
    doc.add(SortedSetDocValuesField::new(
        "foo",
        BytesRef::from_string("pppp"),
    ));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(2_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(3_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_missing_string_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::String, reverse)?;
        sort_field.set_missing_value(StringFirst)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "foo",
            BytesRef::from_string("zzz"),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "foo",
            BytesRef::from_string("mmm"),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_sorted_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("zzz", values.lookup_ord(ord_value)?.utf8_to_string()?);

            assert_eq!(1, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("mmm", values.lookup_ord(ord_value)?.utf8_to_string()?);
        } else {
            assert_eq!(1, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("mmm", values.lookup_ord(ord_value)?.utf8_to_string()?);

            assert_eq!(2, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("zzz", values.lookup_ord(ord_value)?.utf8_to_string()?);
        }

        writer.close()?;
    }

    Ok(())
}
#[test]
fn test_missing_multi_valued_string_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortedSetSortField::new("foo", reverse)?;
        sort_field.set_missing_value(StringFirst)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("zzz"),
        ));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("zzza"),
        ));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("zzzd"),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("mmm"),
        ));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("nnnn"),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_string_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::String, reverse)?;
        sort_field.set_missing_value(StringLast)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "foo",
            BytesRef::from_string("zzz"),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(SortedDocValuesField::new(
            "foo",
            BytesRef::from_string("mmm"),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_sorted_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(1, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("zzz", values.lookup_ord(ord_value)?.utf8_to_string()?);

            assert_eq!(2, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("mmm", values.lookup_ord(ord_value)?.utf8_to_string()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("mmm", values.lookup_ord(ord_value)?.utf8_to_string()?);

            assert_eq!(1, values.next_doc()?);
            let ord_value = values.ord_value()?;
            assert_eq!("zzz", values.lookup_ord(ord_value)?.utf8_to_string()?);
        }

        assert_eq!(NO_MORE_DOCS, values.next_doc()?);
        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_string_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortedSetSortField::new("foo", reverse)?;
        sort_field.set_missing_value(StringLast)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("zzz"),
        ));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("zzzd"),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("mmm"),
        ));
        doc.add(SortedSetDocValuesField::new(
            "foo",
            BytesRef::from_string("ppp"),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_basic_long() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Long)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 18));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", -1));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 7));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(-1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(7_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(18_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_basic_multi_valued_long() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortedNumericSortField::new(
        "foo",
        SortFieldType::Long,
    )?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 3));
    doc.add(SortedNumericDocValuesField::new("foo", 18));
    doc.add(SortedNumericDocValuesField::new("foo", 35));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 1));
    doc.add(SortedNumericDocValuesField::new("foo", -1));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 2));
    doc.add(SortedNumericDocValuesField::new("foo", 7));
    doc.add(SortedNumericDocValuesField::new("foo", 22));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(2_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(3_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_missing_long_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Long, reverse)?;
        sort_field.set_missing_value(i64::MIN)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 18));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 7));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);
        } else {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_long_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Long, reverse)?;
        sort_field.set_missing_value(i64::MIN)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        doc.add(SortedNumericDocValuesField::new("foo", 18));
        doc.add(SortedNumericDocValuesField::new("foo", 27));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new("foo", 7));
        doc.add(SortedNumericDocValuesField::new("foo", 24));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}
#[test]
fn test_missing_long_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Long, reverse)?;
        sort_field.set_missing_value(i64::MAX)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 18));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 7));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);
        }

        assert_eq!(NO_MORE_DOCS, values.next_doc()?);
        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_long_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Long, reverse)?;
        sort_field.set_missing_value(i64::MAX)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new("foo", 18));
        doc.add(SortedNumericDocValuesField::new("foo", 65));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        doc.add(SortedNumericDocValuesField::new("foo", 7));
        doc.add(SortedNumericDocValuesField::new("foo", 34));
        doc.add(SortedNumericDocValuesField::new("foo", 74));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_basic_int() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Int)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 18));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", -1));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("foo", 7));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(-1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(7_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(18_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_basic_multi_valued_int() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortedNumericSortField::new(
        "foo",
        SortFieldType::Int,
    )?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 3));
    doc.add(SortedNumericDocValuesField::new("foo", 18));
    doc.add(SortedNumericDocValuesField::new("foo", 34));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 1));
    doc.add(SortedNumericDocValuesField::new("foo", -1));
    doc.add(SortedNumericDocValuesField::new("foo", 34));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 2));
    doc.add(SortedNumericDocValuesField::new("foo", 7));
    doc.add(SortedNumericDocValuesField::new("foo", 22));
    doc.add(SortedNumericDocValuesField::new("foo", 27));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(2_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(3_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_missing_int_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Int, reverse)?;
        sort_field.set_missing_value(i32::MIN)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 18));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 7));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);
        } else {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_int_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Int, reverse)?;
        sort_field.set_missing_value(i32::MIN)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        doc.add(SortedNumericDocValuesField::new("foo", 18));
        doc.add(SortedNumericDocValuesField::new("foo", 187667));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new("foo", 7));
        doc.add(SortedNumericDocValuesField::new("foo", 34));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_int_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Int, reverse)?;
        sort_field.set_missing_value(i32::MAX)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 18));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("foo", 7));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(7_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(18_i64, values.long_value()?);
        }

        assert_eq!(NO_MORE_DOCS, values.next_doc()?);
        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_int_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Int, reverse)?;
        sort_field.set_missing_value(i32::MAX)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new("foo", 18));
        doc.add(SortedNumericDocValuesField::new("foo", 6372));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        doc.add(SortedNumericDocValuesField::new("foo", 7));
        doc.add(SortedNumericDocValuesField::new("foo", 8));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_basic_double() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Double)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(DoubleDocValuesField::new("foo", 18.0));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(DoubleDocValuesField::new("foo", -1.0));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(DoubleDocValuesField::new("foo", 7.0));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(-1.0, f64::from_bits(values.long_value()? as u64));

    assert_eq!(1, values.next_doc()?);
    assert_eq!(7.0, f64::from_bits(values.long_value()? as u64));

    assert_eq!(2, values.next_doc()?);
    assert_eq!(18.0, f64::from_bits(values.long_value()? as u64));

    writer.close()?;
    Ok(())
}
#[test]
fn test_basic_multi_valued_double() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortedNumericSortField::new(
        "foo",
        SortFieldType::Double,
    )?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 3));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(7.54),
    ));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(27.0),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 1));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(-1.0),
    ));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(0.0),
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 2));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(7.0),
    ));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::double_to_sortable_long(7.67),
    ));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(2_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(3_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_missing_double_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Double, reverse)?;
        sort_field.set_missing_value(f64::NEG_INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(DoubleDocValuesField::new("foo", 18.0));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(DoubleDocValuesField::new("foo", 7.0));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(18.0, f64::from_bits(values.long_value()? as u64));

            assert_eq!(1, values.next_doc()?);
            assert_eq!(7.0, f64::from_bits(values.long_value()? as u64));
        } else {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(7.0, f64::from_bits(values.long_value()? as u64));

            assert_eq!(2, values.next_doc()?);
            assert_eq!(18.0, f64::from_bits(values.long_value()? as u64));
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_double_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Double, reverse)?;
        sort_field.set_missing_value(f64::NEG_INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(18.0),
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(18.76),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(7.0),
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(70.0),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_double_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Double, reverse)?;
        sort_field.set_missing_value(f64::INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(DoubleDocValuesField::new("foo", 18.0));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(DoubleDocValuesField::new("foo", 7.0));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(18.0, f64::from_bits(values.long_value()? as u64));

            assert_eq!(2, values.next_doc()?);
            assert_eq!(7.0, f64::from_bits(values.long_value()? as u64));
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(7.0, f64::from_bits(values.long_value()? as u64));

            assert_eq!(1, values.next_doc()?);
            assert_eq!(18.0, f64::from_bits(values.long_value()? as u64));
        }

        assert_eq!(NO_MORE_DOCS, values.next_doc()?);
        writer.close()?;
    }

    Ok(())
}
#[test]
fn test_missing_multi_valued_double_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Double, reverse)?;
        sort_field.set_missing_value(f64::INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(18.0),
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(8262.0),
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(7.0),
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::double_to_sortable_long(7.87),
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_basic_float() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Float)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(FloatDocValuesField::new("foo", 18.0));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(FloatDocValuesField::new("foo", -1.0));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(FloatDocValuesField::new("foo", 7.0));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(-1.0f32, f32::from_bits(values.long_value()? as u32));

    assert_eq!(1, values.next_doc()?);
    assert_eq!(7.0f32, f32::from_bits(values.long_value()? as u32));

    assert_eq!(2, values.next_doc()?);
    assert_eq!(18.0f32, f32::from_bits(values.long_value()? as u32));

    writer.close()?;
    Ok(())
}

#[test]
fn test_basic_multi_valued_float() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortedNumericSortField::new(
        "foo",
        SortFieldType::Float,
    )?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 3));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::float_to_sortable_int(18.0) as i64,
    ));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::float_to_sortable_int(29.0) as i64,
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 1));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::float_to_sortable_int(-1.0) as i64,
    ));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::float_to_sortable_int(34.0) as i64,
    ));
    writer.add_document(doc)?;
    writer.commit()?;

    let mut doc = Document::new();
    doc.add(NumericDocValuesField::new("id", 2));
    doc.add(SortedNumericDocValuesField::new(
        "foo",
        NumericUtils::float_to_sortable_int(7.0) as i64,
    ));
    writer.add_document(doc)?;
    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let leaf = get_only_leaf_reader(&reader)?;
    assert_eq!(3, leaf.max_doc()?);

    let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

    assert_eq!(0, values.next_doc()?);
    assert_eq!(1_i64, values.long_value()?);

    assert_eq!(1, values.next_doc()?);
    assert_eq!(2_i64, values.long_value()?);

    assert_eq!(2, values.next_doc()?);
    assert_eq!(3_i64, values.long_value()?);

    writer.close()?;
    Ok(())
}

#[test]
fn test_missing_float_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Float, reverse)?;
        sort_field.set_missing_value(f32::NEG_INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(FloatDocValuesField::new("foo", 18.0));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(FloatDocValuesField::new("foo", 7.0));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(18.0f32, f32::from_bits(values.long_value()? as u32));

            assert_eq!(1, values.next_doc()?);
            assert_eq!(7.0f32, f32::from_bits(values.long_value()? as u32));
        } else {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(7.0f32, f32::from_bits(values.long_value()? as u32));

            assert_eq!(2, values.next_doc()?);
            assert_eq!(18.0f32, f32::from_bits(values.long_value()? as u32));
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_multi_valued_float_first() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Float, reverse)?;
        sort_field.set_missing_value(f32::NEG_INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(18.0) as i64,
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(726.0) as i64,
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(7.0) as i64,
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(18.0) as i64,
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}

#[test]
fn test_missing_float_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field = SortField::with_reverse(Some("foo"), SortFieldType::Float, reverse)?;
        sort_field.set_missing_value(f32::INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(FloatDocValuesField::new("foo", 18.0));
        writer.add_document(doc)?;
        writer.commit()?;

        writer.add_document(Document::new())?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(FloatDocValuesField::new("foo", 7.0));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();

        if reverse {
            assert_eq!(1, values.next_doc()?);
            assert_eq!(18.0f32, f32::from_bits(values.long_value()? as u32));

            assert_eq!(2, values.next_doc()?);
            assert_eq!(7.0f32, f32::from_bits(values.long_value()? as u32));
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(7.0f32, f32::from_bits(values.long_value()? as u32));

            assert_eq!(1, values.next_doc()?);
            assert_eq!(18.0f32, f32::from_bits(values.long_value()? as u32));
        }

        assert_eq!(NO_MORE_DOCS, values.next_doc()?);
        writer.close()?;
    }

    Ok(())
}
#[test]
fn test_missing_multi_valued_float_last() -> Result<()> {
    for reverse in [true, false] {
        let mut random = random();
        let dir = new_directory_shared(&mut random)?;

        let analyzer = MockAnalyzer::new(&mut random);
        let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

        let mut sort_field =
            SortedNumericSortField::with_reverse("foo", SortFieldType::Float, reverse)?;
        sort_field.set_missing_value(f32::INFINITY)?;
        let index_sort = Sort::with_fields(vec![sort_field])?;
        iwc.set_index_sort(index_sort)?;

        let writer = IndexWriter::new(dir.clone(), iwc)?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 2));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(726.0) as i64,
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(18.0) as i64,
        ));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 3));
        writer.add_document(doc)?;
        writer.commit()?;

        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new("id", 1));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(12.67) as i64,
        ));
        doc.add(SortedNumericDocValuesField::new(
            "foo",
            NumericUtils::float_to_sortable_int(7.0) as i64,
        ));
        writer.add_document(doc)?;
        writer.force_merge(1)?;

        let reader = directory_reader_util::open_from_writer(&writer)?;
        let leaf = get_only_leaf_reader(&reader)?;
        assert_eq!(3, leaf.max_doc()?);

        let mut values = leaf.get_numeric_doc_values("id")?.unwrap();

        if reverse {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);
        } else {
            assert_eq!(0, values.next_doc()?);
            assert_eq!(1_i64, values.long_value()?);

            assert_eq!(1, values.next_doc()?);
            assert_eq!(2_i64, values.long_value()?);

            assert_eq!(2, values.next_doc()?);
            assert_eq!(3_i64, values.long_value()?);
        }

        writer.close()?;
    }

    Ok(())
}
#[test]
fn test_random1() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Arc::new(Sort::with_fields(vec![SortField::new(
        Some("foo"),
        SortFieldType::Long,
    )?])?);
    iwc.set_index_sort(index_sort.clone())?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let num_docs = at_least_usize(&mut random, 200);
    let mut deleted = FixedBitSet::new(num_docs as usize);

    for i in 0..num_docs {
        let mut doc = Document::new();
        doc.add(NumericDocValuesField::new(
            "foo",
            random.random_range(0..20),
        ));
        doc.add(StringField::from_string("id", i.to_string(), Store::Yes)?);
        doc.add(NumericDocValuesField::new("id", i as i64));
        writer.add_document(doc)?;

        if random.random_range(0..5) == 0 {
            directory_reader_util::open_from_writer(&writer)?.close()?;
        } else if random.random_range(0..30) == 0 {
            writer.force_merge(2)?;
        } else if random.random_range(0..4) == 0 {
            let id = TestUtil::next_usize(&mut random, 0, i);
            deleted.set(id);
            writer.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
        }
    }

    let reader = Arc::new(directory_reader_util::open_from_writer(&writer)?);
    let irc = get_context(reader.clone())?;
    for ctx in irc.leaves()? {
        let leaf = ctx.reader();

        let info = &leaf.get_segment_info().info;
        let source = info.get_diagnostics().get(SOURCE);

        match source {
            Some(src) if src == SOURCE_FLUSH || src == SOURCE_MERGE => {
                assert!(Arc::ptr_eq(&index_sort, &info.get_index_sort().unwrap()));

                let mut values = leaf.get_numeric_doc_values("foo")?.unwrap();
                let mut previous = i64::MIN;

                for doc_id in 0..leaf.max_doc()? {
                    assert_eq!(doc_id, values.next_doc()?);
                    let value = values.long_value()?;
                    assert!(value >= previous);
                    previous = value;
                }
            },
            _ => unreachable!("unexpected segment source"),
        }
    }

    let searcher = new_searcher_with_reader(reader.clone())?;
    let mut stored_fields = reader.stored_fields()?;

    for i in 0..num_docs {
        let term_query = TermQuery::new(Term::from_text("id", i.to_string()));
        let top_docs = searcher.search(term_query, 1)?;

        if deleted.get(i)? {
            assert_eq!(0, top_docs.total_hits.value());
        } else {
            assert_eq!(1, top_docs.total_hits.value());

            let mut values = MultiDocValues::get_numeric_values(&reader, "id")?.unwrap();
            assert_eq!(
                top_docs.score_docs[0].doc,
                values.advance(top_docs.score_docs[0].doc)?
            );
            assert_eq!(i as i64, values.long_value()?);
            let document = stored_fields.document(top_docs.score_docs[0].doc)?;
            assert_eq!(&i.to_string(), document.get("id")?.unwrap().as_ref());
        }
    }

    writer.close()?;
    Ok(())
}
#[test]
fn test_multi_valued_random1() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Arc::new(Sort::with_fields(vec![SortedNumericSortField::new(
        "foo",
        SortFieldType::Long,
    )?])?);
    iwc.set_index_sort(index_sort.clone())?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;
    let num_docs = at_least_usize(&mut random, 200);
    let mut deleted = FixedBitSet::new(num_docs as usize);

    for i in 0..num_docs {
        let mut doc = Document::new();
        let num = random.random_range(0..10);
        for _ in 0..num {
            doc.add(SortedNumericDocValuesField::new(
                "foo",
                random.random_range(0..2000),
            ));
        }
        doc.add(StringField::from_string("id", i.to_string(), Store::Yes)?);
        doc.add(NumericDocValuesField::new("id", i as i64));
        writer.add_document(doc)?;

        if random.random_range(0..5) == 0 {
            directory_reader_util::open_from_writer(&writer)?.close()?;
        } else if random.random_range(0..30) == 0 {
            writer.force_merge(2)?;
        } else if random.random_range(0..4) == 0 {
            let id = TestUtil::next_usize(&mut random, 0, i);
            deleted.set(id);
            writer.delete_documents_with_terms(vec![Term::from_text("id", id.to_string())])?;
        }
    }

    let reader = Arc::new(directory_reader_util::open_from_writer(&writer)?);
    let searcher = new_searcher_with_reader(reader.clone())?;
    let mut stored_fields = reader.stored_fields()?;

    for i in 0..num_docs {
        let term_query = TermQuery::new(Term::from_text("id", i.to_string()));
        let top_docs = searcher.search(term_query, 1)?;

        if deleted.get(i)? {
            assert_eq!(0, top_docs.total_hits.value());
        } else {
            assert_eq!(1, top_docs.total_hits.value());

            let mut values = MultiDocValues::get_numeric_values(&reader, "id")?.unwrap();
            assert_eq!(
                top_docs.score_docs[0].doc,
                values.advance(top_docs.score_docs[0].doc)?
            );
            assert_eq!(i as i64, values.long_value()?);
            let document = stored_fields.document(top_docs.score_docs[0].doc)?;
            assert_eq!(&i.to_string(), document.get("id")?.unwrap().as_ref());
        }
    }

    writer.close()?;
    Ok(())
}
#[test]
fn test_concurrent_updates() -> Result<()> {
    // TODO 多线程未实现
    Ok(())
}
#[test]
fn test_bad_dv_update() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Long)?])?;
    iwc.set_index_sort(index_sort)?;

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    let mut doc = Document::new();
    doc.add(StringField::from_bytes_ref(
        "id",
        BytesRef::from_string("0"),
        Store::No,
    )?);
    doc.add(NumericDocValuesField::new("foo", random.random::<i64>()));
    writer.add_document(doc)?;
    writer.commit()?;

    let err = writer
        .update_doc_values(
            Term::from_text("id", "0"),
            vec![NumericDocValuesField::new("foo", -1).into()],
        )
        .unwrap_err();
    match err {
        LuceneError::IllegalArgument(msg) => {
            assert_eq!(
                "cannot update docvalues field involved in the index sort, field=foo, sort=<long: \"foo\">",
                msg.to_string()
            );
        },
        _ => unreachable!("expected IllegalArgument"),
    }

    let err = writer
        .update_numeric_doc_value(Term::from_text("id", "0"), "foo", -1)
        .unwrap_err();
    match err {
        LuceneError::IllegalArgument(msg) => {
            assert_eq!(
                "cannot update docvalues field involved in the index sort, field=foo, sort=<long: \"foo\">",
                msg.to_string()
            );
        },
        _ => unreachable!("expected IllegalArgument"),
    }

    writer.close()?;
    Ok(())
}
#[test]
fn test_concurrent_dv_updates() -> Result<()> {
    // TODO 多线程未实现
    Ok(())
}
#[test]
fn test_bad_add_indexes() -> Result<()> {
    // TODO add_indexes未实现
    Ok(())
}
#[test]
fn test_add_indexes() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_add_indexes_with_deletions() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_add_indexes_with_directory() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_add_indexes_with_deletions_and_directory() -> Result<()> {
    // TODO
    Ok(())
}
#[test]
fn test_bad_sort() -> Result<()> {
    let mut random = random();
    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);

    let err = iwc.set_index_sort(Sort::get_relevance()?).err().unwrap();
    match err {
        LuceneError::IllegalArgument(msg) => {
            assert_eq!("Cannot sort index with sort field <score>", msg.to_string());
        },
        _ => unreachable!("expected IllegalArgument"),
    }

    Ok(())
}
#[test]
fn test_illegal_change_sort() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::Long)?])?;
    iwc.set_index_sort(index_sort)?;
    {
        let writer = IndexWriter::new(dir.clone(), iwc)?;
        writer.add_document(Document::new())?;
        directory_reader_util::open_from_writer(&writer)?.close()?;
        writer.add_document(Document::new())?;
        writer.force_merge(1)?;
        writer.close()?;
    }

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc2 = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("bar"), SortFieldType::Long)?])?;
    iwc2.set_index_sort(index_sort)?;

    let err = IndexWriter::new(dir.clone(), iwc2).err().unwrap();
    match err {
        LuceneError::IllegalArgument(msg) => {
            let message = msg.to_string();
            assert!(message.contains("cannot change previous indexSort=<long: \"foo\">"));
            assert!(message.contains("to new indexSort=<long: \"bar\">"));
        },
        _ => unreachable!("expected IllegalArgument"),
    }

    Ok(())
}
#[test]
fn test_random2() -> Result<()> {
    //TODO  PositionsTokenStream 未实现
    Ok(())
}
#[test]
fn test_random3() -> Result<()> {
    // TODO
    Ok(())
}
#[test]
fn test_tie_break() -> Result<()> {
    let mut random = random();
    let dir = new_directory_shared(&mut random)?;

    let analyzer = MockAnalyzer::new(&mut random);
    let mut iwc = new_index_writer_config_with_analyzer(&mut random, analyzer);
    let index_sort = Sort::with_fields(vec![SortField::new(Some("foo"), SortFieldType::String)?])?;
    iwc.set_index_sort(index_sort)?;
    // iwc.set_merge_policy(new_log_merge_policy(&mut random)?);

    let writer = IndexWriter::new(dir.clone(), iwc)?;

    for id in 0..1000 {
        let mut doc = Document::new();
        doc.add(StoredField::from_i32("id", id)?);

        let value = if id < 500 { "bar2" } else { "bar1" };
        doc.add(SortedDocValuesField::new(
            "foo",
            BytesRef::from_string(value),
        ));
        writer.add_document(doc)?;

        if id == 500 {
            writer.commit()?;
        }
    }

    writer.force_merge(1)?;

    let reader = directory_reader_util::open_from_writer(&writer)?;
    let mut stored_fields = reader.stored_fields()?;

    for doc_id in 0..1000 {
        let expected_id = if doc_id < 500 {
            500 + doc_id
        } else {
            doc_id - 500
        };

        let document = stored_fields.document(doc_id)?;
        let field = document.get_field("id");
        assert_eq!(
            expected_id,
            field.unwrap().numeric_value()?.unwrap().to_i32().unwrap()
        );
    }

    writer.close()?;
    Ok(())
}

#[test]
fn test_index_sort_with_sparse_field() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_index_sort_on_sparse_field() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_wrong_sort_field_type() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_delete_by_term_or_query() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_sort_docs() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_sort_docs_and_freqs() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_sort_docs_and_freqs_and_positions() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_sort_docs_and_freqs_and_positions_and_offsets() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_parent_field_not_configured() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_block_contains_parent_field() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_index_sort_with_blocks() -> Result<()> {
    // TODO
    Ok(())
}

#[test]
fn test_mix_random_documents_with_blocks() -> Result<()> {
    // TODO
    Ok(())
}
