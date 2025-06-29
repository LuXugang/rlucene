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
use std::rc::Rc;

use crate::codecs::doc_values_enum::norms::Lucene90NormNumericDocValuesEnum;
use crate::codecs::lucene90_norms_producer::Lucene90NormsProducer;
use crate::index::field_info::FieldInfo;
use crate::index::numeric_doc_values::NumericDocValues;
use crate::store::IndexInput;
use crate::util::error::lucene_error::Result;

/// A trait that produces field normalization values.
pub trait NormsProducer {
    type NumericDocValues: NumericDocValues;
    /// Returns `NumericDocValues` for the given field.
    ///
    /// The returned instance is not required to be thread-safe:
    /// it will only be used by a single thread.
    ///
    /// Behavior is undefined if the given field does not have norms enabled.
    fn get_norms(&mut self, field: &Rc<FieldInfo>) -> Result<Self::NumericDocValues>;

    /// Checks consistency of this producer.
    ///
    /// Note: this may be expensive in terms of I/O,
    /// for example it might compute a checksum over large data files.
    fn check_integrity(&mut self) -> Result<()>;

    /// Returns an instance optimized for merging.
    ///
    /// This instance may only be used from the thread that acquires it.
    ///
    /// By default, this method returns `None`, which indicates that no new
    /// `NormsProducerEnum` is required for merging, and the current instance
    /// should be used directly during merge operations.
    fn get_merge_instance(&self) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        Ok(None)
    }
}

pub enum NormsProducerEnum<I>
where
    I: IndexInput,
{
    Lucene90(Lucene90NormsProducer<I>),
}
impl<I> NormsProducer for NormsProducerEnum<I>
where
    I: IndexInput,
{
    type NumericDocValues = Lucene90NormNumericDocValuesEnum<I>;

    fn get_norms(&mut self, field: &Rc<FieldInfo>) -> Result<Lucene90NormNumericDocValuesEnum<I>> {
        match self {
            NormsProducerEnum::Lucene90(producer) => producer.get_norms(field),
        }
    }

    fn check_integrity(&mut self) -> Result<()> {
        match self {
            NormsProducerEnum::Lucene90(producer) => producer.check_integrity(),
        }
    }
}
