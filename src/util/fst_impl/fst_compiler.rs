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
use crate::store::DataOutput;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::{LuceneError, Result};
use crate::util::fst_impl::fst::DummyBytesReader;
use crate::util::fst_impl::fst_reader::FstReader;
/// This class is used for FST backed by non-FSTReader DataOutput. It does not allow getting the
/// reverse BytesReader nor writing to a DataOutput.
struct NullFSTReader;
#[allow(unused)]
impl Accountable for NullFSTReader {
    fn ram_bytes_used(&self) -> Result<i64> {
        todo!()
    }
}
#[allow(unused)]
impl FstReader for NullFSTReader {
    type FstBytesReader = DummyBytesReader;

    fn get_reverse_bytes_reader(&self) -> Result<Self::FstBytesReader> {
        Err(LuceneError::unsupported_operation(
            "FST was not constructed with getOnHeapReaderWriter()".to_string(),
        ))
    }

    fn write_to(&self, _out: &mut impl DataOutput) -> Result<()> {
        Err(LuceneError::unsupported_operation(
            "FST was not constructed with getOnHeapReaderWriter()".to_string(),
        ))
    }
}
