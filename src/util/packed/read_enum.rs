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
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;
use crate::util::packed::mutable_packed64_enum::MutablePacked64Enum;
use crate::util::packed::{NullReader, Reader};

pub enum PackedIntsReadEnum {
    PackedReader(MutablePacked64Enum),
    NullReader(NullReader),
}

impl Accountable for PackedIntsReadEnum {
    fn ram_bytes_used(&self) -> Result<i64> {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.ram_bytes_used(),
            PackedIntsReadEnum::NullReader(op) => op.ram_bytes_used(),
        }
    }
}

impl Reader for PackedIntsReadEnum {
    fn get(&self, index: i32) -> Result<i64> {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.get(index),
            PackedIntsReadEnum::NullReader(op) => op.get(index),
        }
    }

    fn get_bulk(&self, index: i32, arr: &mut [i64], off: i32, len: i32) -> Result<i32> {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.get_bulk(index, arr, off, len),
            PackedIntsReadEnum::NullReader(op) => op.get_bulk(index, arr, off, len),
        }
    }

    fn size(&self) -> i32 {
        match self {
            PackedIntsReadEnum::PackedReader(op) => op.size(),
            PackedIntsReadEnum::NullReader(op) => op.size(),
        }
    }
}
impl Default for PackedIntsReadEnum {
    // used for padding value
    fn default() -> Self {
        PackedIntsReadEnum::NullReader(NullReader::new(0))
    }
}
