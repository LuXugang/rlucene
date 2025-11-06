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
use crate::core::index::BytesRef;
use crate::core::index::doc_values_type::DocValuesType;
use crate::core::index::term::Term;
use crate::core::store::DataOutput;
use crate::core::util::error::lucene_error::Result;
use std::fmt::Display;
use std::sync::Arc;

/// An in-place update to a DocValues field.
pub struct DocValuesUpdate {
    pub(crate) doc_values_type: DocValuesType,
    pub term: Arc<Term>,
    pub field: String,
    // used in BufferedDeletes to apply this update only to a slice of docs.
    // It's initialized to BufferedUpdates.MAX_INT
    // since it's safe and most often used this way we save object creations.
    pub doc_id_upto: i32,
    pub has_value: bool,
    pub sub_update: DocValuesUpdateEnum,
}
impl DocValuesUpdate {
    const RAW_SIZE_IN_BYTES: i32 = 0;
    pub fn new<T, F>(
        doc_values_type: DocValuesType,
        term: F,
        field: T,
        doc_id_upto: i32,
        sub_update: DocValuesUpdateEnum,
    ) -> Self
    where
        T: Into<String>,
        F: Into<Arc<Term>>,
    {
        let field = field.into();
        let term = term.into();
        debug_assert!(doc_id_upto >= 0, "{doc_id_upto} must be >= 0");
        let has_value = sub_update.has_value();
        DocValuesUpdate {
            doc_values_type,
            term,
            field,
            doc_id_upto,
            has_value,
            sub_update,
        }
    }

    pub(crate) fn has_value(&self) -> bool {
        self.has_value
    }
    fn size_in_bytes(&self) -> i32 {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    #[cfg(debug_assertions)]
    pub fn prepare_for_apply(&mut self, doc_id_upto: i32) -> Option<DocValuesUpdate> {
        if doc_id_upto == self.doc_id_upto {
            return None;
        }
        let sub_update = self.sub_update.prepare_for_apply();
        Some(DocValuesUpdate::new(
            self.doc_values_type,
            self.term.clone(),
            self.field.clone(),
            doc_id_upto,
            sub_update,
        ))
    }
}
impl Display for DocValuesUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "term={}, field={}, value={}, docIDUpTo={}",
            self.term,
            self.field,
            self.sub_update.value_to_string(),
            self.doc_id_upto
        )
    }
}
pub trait DocValuesUpdateBase {
    #[allow(dead_code)]
    fn value_size_in_bytes(&self) -> i64 {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    fn value_to_string(&self) -> String;
    #[allow(dead_code)]
    fn write_to<D: DataOutput>(&self, _bytes: &mut BytesRef<Vec<u8>>) -> Result<()> {
        unimplemented!("Not used in Java Lucene, so we did not implement it")
    }
    fn has_value(&self) -> bool;
    #[cfg(debug_assertions)]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum;
}
/// An in-place update to a binary DocValues field.
#[derive(Clone)]
pub struct BinaryDocValuesUpdate {
    value: Option<BytesRef<Vec<u8>>>,
}
impl BinaryDocValuesUpdate {
    const RAW_VALUE_SIZE_IN_BYTES: i32 = 0;
    pub fn new(value: Option<BytesRef<Vec<u8>>>) -> Self {
        BinaryDocValuesUpdate { value }
    }
    pub fn get_value(&self) -> BytesRef<Vec<u8>> {
        debug_assert!(self.value.is_some());
        self.value.as_ref().unwrap().clone()
    }
}
impl DocValuesUpdateBase for BinaryDocValuesUpdate {
    fn value_to_string(&self) -> String {
        match &self.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        }
    }

    fn has_value(&self) -> bool {
        self.value.is_some()
    }

    #[cfg(debug_assertions)]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        DocValuesUpdateEnum::Binary(BinaryDocValuesUpdate::new(self.value.clone()))
    }
}
#[derive(Clone)]
pub struct NumericDocValuesUpdate {
    value: Option<i64>,
}
impl NumericDocValuesUpdate {
    pub fn new(value: Option<i64>) -> Self {
        NumericDocValuesUpdate { value }
    }
    pub fn get_value(&self) -> i64 {
        debug_assert!(
            self.value.is_some(),
            "getValue should only be called if this update has a value"
        );
        *self.value.as_ref().unwrap()
    }
}
impl DocValuesUpdateBase for NumericDocValuesUpdate {
    fn value_to_string(&self) -> String {
        match self.value {
            Some(v) => v.to_string(),
            None => "null".to_string(),
        }
    }

    fn has_value(&self) -> bool {
        self.value.is_some()
    }

    #[cfg(debug_assertions)]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        DocValuesUpdateEnum::Numeric(NumericDocValuesUpdate::new(self.value))
    }
}

pub enum DocValuesUpdateEnum {
    Binary(BinaryDocValuesUpdate),
    Numeric(NumericDocValuesUpdate),
}
impl DocValuesUpdateEnum {
    pub fn get_binary(&self) -> Option<&BinaryDocValuesUpdate> {
        debug_assert!(matches!(self, DocValuesUpdateEnum::Binary(_)));
        match self {
            DocValuesUpdateEnum::Binary(b) => Some(b),
            _ => None,
        }
    }

    pub fn get_numeric(&self) -> Option<&NumericDocValuesUpdate> {
        debug_assert!(matches!(self, DocValuesUpdateEnum::Numeric(_)));
        match self {
            DocValuesUpdateEnum::Numeric(n) => Some(n),
            _ => None,
        }
    }
}
impl DocValuesUpdateBase for DocValuesUpdateEnum {
    fn value_to_string(&self) -> String {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.value_to_string(),
            DocValuesUpdateEnum::Numeric(n) => n.value_to_string(),
        }
    }

    fn has_value(&self) -> bool {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.has_value(),
            DocValuesUpdateEnum::Numeric(n) => n.has_value(),
        }
    }

    #[cfg(debug_assertions)]
    fn prepare_for_apply(&mut self) -> DocValuesUpdateEnum {
        match self {
            DocValuesUpdateEnum::Binary(b) => b.prepare_for_apply(),
            DocValuesUpdateEnum::Numeric(n) => n.prepare_for_apply(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::document::binary_doc_values_field::BinaryDocValuesField;
    use crate::core::document::document::Document;
    use crate::core::document::field::Store;
    use crate::core::document::numeric_doc_values_field::NumericDocValuesField;
    use crate::core::document::sorted_doc_values_field::SortedDocValuesField;
    use crate::core::document::sorted_set_doc_values_field::SortedSetDocValuesField;
    use crate::core::document::string_field::StringField;
    use crate::core::index::BytesRef;
    use crate::core::index::binary_doc_values::BinaryDocValues;
    use crate::core::index::composite_reader::get_context;
    use crate::core::index::directory_reader::directory_reader_util;
    use crate::core::index::index_reader::IndexReader;
    use crate::core::index::index_reader_context::IndexReaderContext;
    use crate::core::index::index_writer::IndexWriter;
    use crate::core::index::index_writer_config::DISABLE_AUTO_FLUSH;
    use crate::core::index::leaf_reader::LeafReader;
    use crate::core::index::live_index_writer_config::LiveIndexWriterConfig;
    use crate::core::index::numeric_doc_values::NumericDocValues;
    use crate::core::index::sorted_doc_values::SortedDocValues;
    use crate::core::index::sorted_set_doc_values::SortedSetDocValues;
    use crate::core::index::term::Term;
    use crate::core::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::core::util::bits::Bits;
    use crate::core::util::error::lucene_error::Result;
    use crate::test::util::lucene_test_case::lucene_test_case_util::{
        new_bytes_ref_from_string, new_bytes_ref_with_length, new_directory,
        new_index_writer_config, random,
    };
    use rand::Rng;
    use std::sync::Arc;

    #[allow(dead_code)] // for quick search
    struct TestBinaryDocValuesUpdates;

    fn get_value(bdv: &mut impl BinaryDocValues) -> Result<i64> {
        let term = bdv.binary_value()?;
        let mut idx = term.offset;
        debug_assert!(term.length > 0);
        let mut b = term.bytes[idx];
        idx += 1;

        let mut value = (b & 0x7F) as i64;
        let mut shift = 7;
        while (b as i64 & 0x80) != 0 {
            b = term.bytes[idx];
            idx += 1;
            value |= ((b & 0x7F) as i64) << shift;
            shift += 7;
        }

        Ok(value)
    }
    // encodes a long into a BytesRef as VLong so that we get varying number of bytes when we update
    fn to_bytes(mut value: i64) -> Result<BytesRef<Vec<u8>>> {
        let mut random = random();
        let mut bytes: BytesRef<Vec<u8>> = new_bytes_ref_with_length(10, &mut random)?;
        let mut upto = 0usize;

        while (value & !0x7f) != 0 {
            bytes.bytes[bytes.offset + upto] = ((value & 0x7f) | 0x80) as u8;
            upto += 1;
            value = ((value as u64) >> 7) as i64;
        }

        bytes.bytes[bytes.offset + upto] = value as u8;
        upto += 1;
        bytes.length = upto;

        Ok(bytes)
    }
    fn doc(id: i32) -> Result<Document> {
        let mut doc = Document::new();

        let id_field = StringField::with_string("id", format!("doc-{}", id), Store::No)?;
        doc.add(id_field);

        let val_bytes = to_bytes((id + 1) as i64)?;
        let val_field = BinaryDocValuesField::new("val", val_bytes);
        doc.add(val_field);
        Ok(doc)
    }
    #[test]
    fn test_updates_are_flushed() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);
        // TODO: 未实现MockAnalyzer
        let mut config = new_index_writer_config(&mut random);
        config.set_ram_buffer_size_mb(0.00000001);
        let mut writer = IndexWriter::new(dir.clone(), config)?;
        writer.add_document(doc(0)?)?; // val=1
        writer.add_document(doc(1)?)?; // val=2
        writer.add_document(doc(3)?)?; // val=4
        writer.commit()?;

        assert_eq!(1, writer.get_flush_deletes_count());

        writer.update_binary_doc_value(Term::from_text("id", "doc-0"), "val", to_bytes(5)?)?;
        assert_eq!(2, writer.get_flush_deletes_count());

        writer.update_binary_doc_value(Term::from_text("id", "doc-1"), "val", to_bytes(6)?)?;
        assert_eq!(3, writer.get_flush_deletes_count());

        writer.update_binary_doc_value(Term::from_text("id", "doc-2"), "val", to_bytes(7)?)?;
        assert_eq!(4, writer.get_flush_deletes_count());

        writer.get_config_mut().set_ram_buffer_size_mb(1000.0);
        writer.update_binary_doc_value(Term::from_text("id", "doc-2"), "val", to_bytes(7)?)?;
        assert_eq!(4, writer.get_flush_deletes_count());

        writer.close()?;
        Ok(())
    }
    #[test]
    fn test_simple() -> Result<()> {
        let mut random = random();
        // TODO: 未实现MockAnalyzer
        let dir = Arc::new(new_directory(&mut random)?);

        let mut config = new_index_writer_config(&mut random);
        // make sure random config doesn't flush on us
        config.set_max_buffered_docs(10);
        config.set_ram_buffer_size_mb(DISABLE_AUTO_FLUSH as f64);

        let mut writer = IndexWriter::new(dir.clone(), config)?;

        writer.add_document(doc(0)?)?; // val=1
        writer.add_document(doc(1)?)?; // val=2

        if random.random_bool(0.5) {
            // randomly commit before the update is sent
            writer.commit()?;
        }

        writer.update_binary_doc_value(Term::from_text("id", "doc-0"), "val", to_bytes(2)?)?; // doc=0, exp=2

        // Open reader: either NRT or non-NRT
        let reader = if random.random_bool(0.5) {
            writer.close()?;
            directory_reader_util::open(dir.clone())?
        } else {
            let r = directory_reader_util::open_with_writer(&mut writer)?;
            writer.close()?;
            r
        };

        let reader = get_context(Arc::new(reader))?;
        assert_eq!(1, reader.leaves()?.len());
        let leaf = &reader.leaves()?[0];
        let r = leaf.reader();

        let mut bdv = r.get_binary_doc_values("val")?.unwrap();
        assert_eq!(0, bdv.next_doc()?);
        assert_eq!(2, get_value(&mut bdv)?);

        assert_eq!(1, bdv.next_doc()?);
        assert_eq!(2, get_value(&mut bdv)?);

        Ok(())
    }
    #[test]
    fn test_update_few_segments() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);

        let mut config = new_index_writer_config(&mut random);
        config.set_max_buffered_docs(2); // generate few segments
        // TODO: 未实现 NoMergePolicy
        // config.set_merge_policy(NoMergePolicy::INSTANCE);
        let mut writer = IndexWriter::new(dir.clone(), config)?;

        let num_docs = 10;
        let mut expected_values = vec![0i64; num_docs];
        for i in 0..num_docs {
            writer.add_document(doc(i as i32)?)?;
            expected_values[i] = (i + 1) as i64;
        }
        writer.commit()?;

        // update few docs
        for i in 0..num_docs {
            if random.random_range(0.0..1.0) < 0.4 {
                let value = ((i + 1) * 2) as i64;
                writer.update_binary_doc_value(
                    Term::from_text("id", &format!("doc-{i}")),
                    "val",
                    to_bytes(value)?,
                )?;
                expected_values[i] = value;
            }
        }

        let reader = if random.random_bool(0.5) {
            writer.close()?;
            directory_reader_util::open(dir.clone())?
        } else {
            let r = directory_reader_util::open_with_writer(&mut writer)?;
            writer.close()?;
            r
        };
        let reader = get_context(Arc::new(reader))?;
        for context in reader.leaves()?.iter() {
            let r = context.reader();
            let bdv = r.get_binary_doc_values("val")?;
            assert!(bdv.is_some(), "BinaryDocValues should not be None");
            let mut bdv = bdv.unwrap();

            let max_doc = r.max_doc()?;
            for i in 0..max_doc {
                assert_eq!(i, bdv.next_doc()?);
                let expected = expected_values[(i + context.doc_base) as usize];
                let actual = get_value(&mut bdv)?;
                assert_eq!(expected, actual);
            }
        }

        Ok(())
    }
    #[test]
    fn test_reopen() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_updates_and_deletes() -> Result<()> {
        // TODO
        Ok(())
    }
    #[test]
    fn test_updates_with_deletes() -> Result<()> {
        let mut random = random();
        // update and delete different documents in the same commit session
        let dir = Arc::new(new_directory(&mut random)?);

        // TODO: 未实现 MockAnalyzer / NoMergePolicy
        let mut config = new_index_writer_config(&mut random);
        config.set_max_buffered_docs(10); // control segment flushing
        // config.set_merge_policy(NoMergePolicy::INSTANCE);
        let mut writer = IndexWriter::new(dir.clone(), config)?;

        writer.add_document(doc(0)?)?;
        writer.add_document(doc(1)?)?;

        if random.random_bool(0.5) {
            writer.commit()?;
        }

        // update and delete different documents in the same commit session
        writer.delete_documents_with_terms(vec![Term::from_text("id", "doc-0")])?;
        writer.update_binary_doc_value(Term::from_text("id", "doc-1"), "val", to_bytes(17_i64)?)?;

        // open reader
        let reader = if random.random_bool(0.5) {
            writer.close()?;
            directory_reader_util::open(dir.clone())?
        } else {
            let r = directory_reader_util::open_with_writer(&mut writer)?;
            writer.close()?;
            r
        };
        let reader = get_context(Arc::new(reader))?;
        let leaves = reader.leaves()?;
        let r = leaves[0].reader();
        let live_docs = r.get_live_docs()?.unwrap();
        assert!(!live_docs.get(0));

        let mut bdv = r.get_binary_doc_values("val")?.unwrap();
        assert_eq!(1, bdv.advance(1)?);
        assert_eq!(17_i64, get_value(&mut bdv)?);

        Ok(())
    }
    fn test_multiple_doc_values_types() -> Result<()> {
        let mut random = random();
        let dir = Arc::new(new_directory(&mut random)?);

        let mut config = new_index_writer_config(&mut random);
        config.set_max_buffered_docs(10); // prevent merges
        let writer = IndexWriter::new(dir.clone(), config)?;

        for i in 0..4 {
            let mut doc = Document::new();
            doc.add(StringField::with_string("dvUpdateKey", "dv", Store::No)?);
            doc.add(NumericDocValuesField::new("ndv", i as i64));
            doc.add(BinaryDocValuesField::new(
                "bdv",
                new_bytes_ref_from_string(&mut random, &i.to_string())?,
            ));
            doc.add(SortedDocValuesField::new(
                "sdv",
                new_bytes_ref_from_string(&mut random, &i.to_string())?,
            ));
            doc.add(SortedSetDocValuesField::new(
                "ssdv",
                new_bytes_ref_from_string(&mut random, &i.to_string())?,
            ));
            doc.add(SortedSetDocValuesField::new(
                "ssdv",
                new_bytes_ref_from_string(&mut random, &(i * 2).to_string())?,
            ));
            writer.add_document(doc)?;
        }
        writer.commit()?;

        // update all docs' bdv field
        writer.update_binary_doc_value(
            Term::from_text("dvUpdateKey", "dv"),
            "bdv",
            to_bytes(17_i64)?,
        )?;
        writer.close()?;

        let reader = directory_reader_util::open(dir.clone())?;
        let reader = get_context(Arc::new(reader))?;
        let leaves = reader.leaves()?;
        let r = leaves[0].reader();

        let mut ndv = r.get_numeric_doc_values("ndv")?.unwrap();
        let mut bdv = r.get_binary_doc_values("bdv")?.unwrap();
        let mut sdv = r.get_sorted_doc_values("sdv")?.unwrap();
        let mut ssdv = r.get_sorted_set_doc_values("ssdv")?.unwrap();

        let max_doc = r.max_doc()?;
        for i in 0..max_doc {
            // NumericDocValues
            assert_eq!(i, ndv.next_doc()?);
            assert_eq!(i as i64, ndv.long_value()?);

            // BinaryDocValues
            assert_eq!(i, bdv.next_doc()?);
            assert_eq!(17_i64, get_value(&mut bdv)?);

            // SortedDocValues
            assert_eq!(i, sdv.next_doc()?);
            let v = sdv.ord_value()?;
            let term = sdv.lookup_ord(v)?;
            let v: BytesRef<Vec<u8>> = new_bytes_ref_from_string(&mut random, &i.to_string())?;
            assert_eq!(&v, term.as_ref());

            // SortedSetDocValues
            assert_eq!(i, ssdv.next_doc()?);
            let ord = ssdv.next_ord()?;
            let term = ssdv.lookup_ord(ord)?;
            let parsed = term.utf8_to_string()?.parse::<i32>()?;
            assert_eq!(i, parsed);
            // For the i=0 case, we added the same value twice, which was dedup'd by IndexWriter so it has
            // only one value:
            if i == 0 {
                assert_eq!(1, ssdv.doc_value_count()?);
            } else {
                assert_eq!(2, ssdv.doc_value_count()?);
                let ord = ssdv.next_ord()?;
                let term = ssdv.lookup_ord(ord)?;
                let parsed = term.utf8_to_string()?.parse::<i32>()?;
                assert_eq!(i * 2, parsed);
            }
        }

        Ok(())
    }
}
