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
use crate::codecs::block_tree::lucene90_block_tree_terms_writer::Lucene90BlockTreeTermsWriter;
use crate::codecs::lucene101::lucene101_postings_writer::Lucene101PostingsWriter;
use crate::codecs::norms_producer::NormsProducer;
use crate::codecs::push_postings_writer_base::PushPostingsWriterBase;
use crate::index::fields::Fields;
use crate::store::IndexOutput;
use crate::util::error::lucene_error::Result;
/// Abstract API that consumes terms, doc, freq, prox, offset and payloads postings. Concrete
/// implementations of this actually do "something" with the postings (write it into the index in a
/// specific format).
pub trait FieldsConsumer {
    /// Write all fields, terms and postings. This is the "pull" API, allowing you to iterate more than
    /// once over the postings, somewhat analogous to using a DOM API to traverse an XML tree.
    ///
    /// # Notes
    ///
    /// - You must compute index statistics, including each Term’s `doc_freq` and `total_term_freq`, as
    ///   well as the summary `sum_total_term_freq`, `sum_total_doc_freq` and `doc_count`.
    /// - You must skip terms that have no docs and fields that have no terms, even though the
    ///   provided `Fields` API will expose them; this typically requires lazily writing the field or
    ///   term until you’ve actually seen the first term or document.
    /// - The provided `Fields` instance is limited: you cannot call any methods that return
    ///   statistics/counts; you cannot pass a non-null live docs when pulling docs/positions enums.
    fn write<F, N>(&mut self, fields: &mut F, norms: &mut N) -> Result<()>
    where
        F: Fields,
        N: NormsProducer;
    fn close(&mut self) -> Result<()>;
}

pub enum FieldsConsumerEnum<O>
where
    O: IndexOutput,
{
    Lucene90(Lucene90BlockTreeTermsWriter<O, PushPostingsWriterBase<Lucene101PostingsWriter<O>>>),
}
impl<O> FieldsConsumer for FieldsConsumerEnum<O>
where
    O: IndexOutput,
{
    fn write<F, N>(&mut self, fields: &mut F, norms: &mut N) -> Result<()>
    where
        F: Fields,
        N: NormsProducer,
    {
        match self {
            FieldsConsumerEnum::Lucene90(writer) => writer.write(fields, norms),
        }
    }

    fn close(&mut self) -> Result<()> {
        match self {
            FieldsConsumerEnum::Lucene90(writer) => writer.close(),
        }
    }
}
