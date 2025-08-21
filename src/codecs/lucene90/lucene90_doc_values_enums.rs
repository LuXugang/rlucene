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
use crate::codecs::lucene90_doc_values_producer::{
    DenseBinaryDocValues, DenseNumericDocValues, DenseSortedNumericDocValues,
    SpareSortedNumericDocValues, SparseBinaryDocValues, SparseNumericDocValues,
};
use crate::index::binary_doc_values::EitherBinaryDocValues3;
use crate::index::doc_values::{EmptyBinary, EmptyNumeric};
use crate::index::numeric_doc_values::Either3NumericDocValues;
use crate::index::singleton_sorted_numeric_doc_values::SingletonSortedNumericDocValues;
use crate::index::sorted_numeric_doc_values::Either4SortedNumericDocValues;

// 1. NumericDocValues
pub type Lucene90NumericDocValuesEnum<I> =
    Either3NumericDocValues<DenseNumericDocValues<I>, SparseNumericDocValues<I>, EmptyNumeric>;
// 2.SortedNumericDocValues
pub type Lucene90SortedNumericDocValues<I> = Either4SortedNumericDocValues<
    DenseSortedNumericDocValues<I>,
    SpareSortedNumericDocValues<I>,
    SingletonSortedNumericDocValues<Lucene90NumericDocValuesEnum<I>>,
    SingletonSortedNumericDocValues<EmptyNumeric>,
>;

// 3. BinaryDocValues
pub type Lucene90BinaryDocValuesEnum<I> =
    EitherBinaryDocValues3<DenseBinaryDocValues<I>, SparseBinaryDocValues<I>, EmptyBinary>;
