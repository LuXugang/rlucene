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
use crate::index::BytesRef;
use crate::util::error::lucene_error::LuceneError;
use crate::util::error::lucene_error::Result;
use derive_getters::Getters;
/// Contains statistics for a specific term
///
/// This struct holds statistics for this term across all documents for scoring purposes:
///
/// - `doc_freq`: number of documents this term occurs in.
/// - `total_term_freq`: number of tokens for this term.
///
/// The following conditions are always true:
///
/// - All statistics are positive integers: never zero or negative.
/// - `doc_freq <= total_term_freq`
/// - `doc_freq <= sum_doc_freq` of the collection
/// - `total_term_freq <= sumtotal_term_freq` of the collection
///
/// Values may include statistics on deleted documents that have not yet been merged away.
///
/// Be careful when performing calculations on these values because they are represented as 64-bit
/// integer values, you may need to cast to `double` for your use.
///
/// - **term**: Term bytes.  
///   This value is never `null`.
///
/// - **doc_freq**: number of documents containing the term in the collection, in the range  
///   `[1 .. total_term_freq()]`.  
///   This is the document-frequency for the term: the count of documents where the term appears  
///   at least one time.  
///   This value is always a positive number, and never exceeds `total_term_freq`.  
///   It also cannot exceed [`CollectionStatistics::sum_doc_freq()`](crate::search::collection_statistics::CollectionStatistics::get_sum_doc_freq).  
///   See also: [`TermsEnum::doc_freq()`](crate::index::terms_enum::TermsEnum::doc_freq)
///
/// - **total_term_freq**: number of occurrences of the term in the collection, in the range  
///   `[doc_freq() .. CollectionStatistics::sum_total_term_freq()]`.  
///   This is the token count for the term: the number of times it appears in the field across  
///   all documents.  
///   This value is always a positive number, always at least `doc_freq()`,  
///   and never exceeds [`CollectionStatistics::sum_total_term_freq()`](crate::search::collection_statistics::CollectionStatistics::get_sum_total_term_freq).  
///   See also: [`TermsEnum::total_term_freq()`](crate::index::terms_enum::TermsEnum::total_term_freq)
#[derive(Getters)]
pub struct TermStatistics {
    term: BytesRef<Vec<u8>>,
    doc_freq: i64,
    total_term_freq: i64,
}

impl TermStatistics {
    /// Creates a new `TermStatistics` instance for a term.
    ///
    /// # Error
    ///
    /// - Error if `doc_freq` is zero or negative.  
    /// - Error if `total_term_freq` is less than `doc_freq`.  
    pub fn new(term: BytesRef<Vec<u8>>, doc_freq: i64, total_term_freq: i64) -> Result<Self> {
        // In Rust, BytesRef cannot be null, so no null check is needed.
        if doc_freq <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "doc_freq must be positive, doc_freq: {doc_freq}"
            )));
        }
        if total_term_freq <= 0 {
            return Err(LuceneError::illegal_argument(format!(
                "total_term_freq must be positive, total_term_freq: {total_term_freq}"
            )));
        }
        if total_term_freq < doc_freq {
            return Err(LuceneError::illegal_argument(format!(
                "total_term_freq must be at least doc_freq, total_term_freq: {total_term_freq}, doc_freq: {doc_freq}"
            )));
        }
        Ok(TermStatistics {
            term,
            doc_freq,
            total_term_freq,
        })
    }
}
