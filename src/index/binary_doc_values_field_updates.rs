/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
*/
use std::sync::Arc;

use parking_lot::Mutex;

use crate::index::doc_values_field_updates::{
    dvfu_util, AbstractIterator, AbstractIteratorBase, DocValuesFieldInner, DocValuesFieldIterator,
    DocValuesFieldUpdatesBase,
};
use crate::index::doc_values_type::DocValuesType;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::long_values::LongValues;
use crate::util::packed::abstract_paged_mutable::AbstractPagedMutable;
use crate::util::packed::paged_growable_writer::PagedGrowableWriter;
use crate::util::packed::PackedInts;

/// A [`DocValuesFieldUpdates`](crate::index::doc_values_field_updates::DocValuesFieldUpdates) which holds updates for documents of a single `BinaryDocValuesField`.
///
/// # Note
/// This API is experimental and may change in future versions.
pub(crate) struct BinaryDocValuesFieldUpdates {
    offsets: AbstractPagedMutable<PagedGrowableWriter>,
    lengths: AbstractPagedMutable<PagedGrowableWriter>,
    values: BytesRefBuilder<Vec<u8>>,
    lock: Mutex<()>,
}
impl BinaryDocValuesFieldUpdates {
    #[allow(unused)]
    fn new() -> Result<BinaryDocValuesFieldUpdates> {
        let sub_reader1 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
        let offsets = AbstractPagedMutable::new(1, dvfu_util::PAGE_SIZE, sub_reader1)?;
        let sub_reader2 = PagedGrowableWriter::with_fill_page(1, PackedInts::FAST);
        let lengths = AbstractPagedMutable::new(1, dvfu_util::PAGE_SIZE, sub_reader2)?;
        Ok(BinaryDocValuesFieldUpdates {
            offsets,
            lengths,
            values: BytesRefBuilder::new(),
            lock: Mutex::new(()),
        })
    }
}

impl Accountable for BinaryDocValuesFieldUpdates {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}

impl DocValuesFieldUpdatesBase for BinaryDocValuesFieldUpdates {
    fn add_value(&mut self, _doc: i32, _value: i64, _index: i32) -> Result<()> {
        Err(LuceneError::unreachable(
            "BinaryDocValuesFieldUpdates does not support add_value",
        ))
    }

    fn add_byte_ref(&mut self, _doc: i32, value: &BytesRef<Vec<u8>>, index: i32) -> Result<()> {
        let _guard = self.lock.lock();
        self.offsets.set(index as i64, self.values.length() as i64);
        self.lengths.set(index as i64, value.length as i64);
        self.values.append_ref(value);
        Ok(())
    }

    fn add_iterator<T: DocValuesFieldIterator>(
        &mut self,
        doc_id: i32,
        mut iterator: T,
    ) -> Result<()> {
        self.add_byte_ref(doc_id, iterator.binary_value()?, 0)
    }

    fn iterator(
        &mut self,
        inner: Arc<Mutex<DocValuesFieldInner>>,
        del_gen: i64,
    ) -> Result<impl DocValuesFieldIterator> {
        let base = AbstractIteratorBaseImpl::new(
            Some(&mut self.offsets),
            Some(&mut self.lengths),
            Some(self.values.get_bytes_mut_ref()),
        );
        Ok(AbstractIterator::new(inner, del_gen, base))
    }
    fn swap(&mut self, i: i32, j: i32) -> Result<()> {
        let temp_offset = self.offsets.get(j as i64)?;
        let value = self.offsets.get(i as i64)?;
        self.offsets.set(j as i64, value);
        self.offsets.set(i as i64, temp_offset);

        let tem_length = self.lengths.get(j as i64)?;
        let length = self.lengths.get(i as i64)?;
        self.lengths.set(j as i64, length);
        self.lengths.set(i as i64, tem_length);
        Ok(())
    }

    fn grow(&mut self, size: i32) -> Result<()> {
        let offset_result = self.offsets.grow_with_size(size as i64)?;
        if offset_result.is_some() {
            self.offsets = offset_result.unwrap();
        }
        let length_result = self.lengths.grow_with_size(size as i64)?;
        if length_result.is_some() {
            self.lengths = length_result.unwrap();
        }
        Ok(())
    }

    fn resize(&mut self, _size: i32) -> Result<()> {
        self.offsets = self.offsets.resize(_size as i64)?;
        self.lengths = self.lengths.resize(_size as i64)?;
        Ok(())
    }

    fn sub_type(&self) -> DocValuesType {
        DocValuesType::Binary
    }
}

/// # Note
/// To implement Default, we wrap the mutable reference fields here with Option.
///
/// Implementing Default is solely for enabling sorting within the
/// PriorityQueue.
#[derive(Default)]
pub struct AbstractIteratorBaseImpl<'a> {
    offsets: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
    offset: i32,
    lengths: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
    length: i32,
    values: Option<&'a mut BytesRef<Vec<u8>>>,
}
#[allow(unused)]
impl<'a> AbstractIteratorBaseImpl<'a> {
    pub fn new(
        offsets: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
        lengths: Option<&'a mut AbstractPagedMutable<PagedGrowableWriter>>,
        values: Option<&'a mut BytesRef<Vec<u8>>>,
    ) -> AbstractIteratorBaseImpl<'a> {
        AbstractIteratorBaseImpl {
            offsets,
            offset: 0,
            lengths,
            length: 0,
            values,
        }
    }
}
impl AbstractIteratorBase for AbstractIteratorBaseImpl<'_> {
    fn set(&mut self, idx: i64) -> Result<()> {
        debug_assert!(self.offsets.is_some());
        debug_assert!(self.lengths.is_some());
        debug_assert!(self.offsets.as_mut().unwrap().get(idx)? <= i32::MAX as i64);
        self.offset = self.offsets.as_mut().unwrap().get(idx)? as i32;
        debug_assert!(self.lengths.as_mut().unwrap().get(idx)? <= i32::MAX as i64);
        self.length = self.lengths.as_mut().unwrap().get(idx)? as i32;
        Ok(())
    }

    fn long_value(&mut self) -> Result<i64> {
        Err(LuceneError::not_implemented(
            "BinaryDocValuesIterator does not support long_value",
        ))
    }

    fn binary_value(&mut self) -> Result<&BytesRef<Vec<u8>>> {
        debug_assert!(self.values.is_some());
        self.values.as_mut().unwrap().offset = self.offset as usize;
        self.values.as_mut().unwrap().length = self.length as usize;
        Ok(self.values.as_mut().unwrap())
    }
}
