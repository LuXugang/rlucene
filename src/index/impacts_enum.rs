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
use crate::index::impacts::ImpactsEnums;
use crate::index::impacts_source::ImpactsSource;
use crate::index::postings_enum::PostingsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;

/// Extension of `PostingsEnum` which also provides information about upcoming impacts.
pub trait ImpactsEnum: PostingsEnum + ImpactsSource {}

pub enum ImpactsEnumEnum {}

impl PostingsEnum for ImpactsEnumEnum {
    fn freq(&mut self) -> Result<i32> {
        todo!()
    }

    fn next_position(&mut self) -> Result<i32> {
        todo!()
    }

    fn start_offset(&self) -> Result<i32> {
        todo!()
    }

    fn end_offset(&self) -> Result<i32> {
        todo!()
    }

    fn get_payload(&self) -> Result<Option<&BytesRef<Vec<u8>>>> {
        todo!()
    }
}

impl DocIdSetIterator for ImpactsEnumEnum {
    fn doc_id(&self) -> i32 {
        todo!()
    }

    fn next_doc(&mut self) -> Result<i32> {
        todo!()
    }

    fn advance(&mut self, _target: i32) -> Result<i32> {
        todo!()
    }

    fn slow_advance(&mut self, target: i32) -> Result<i32> {
        todo!()
    }

    fn cost(&self) -> Result<i64> {
        todo!()
    }
}

impl ImpactsSource for ImpactsEnumEnum {
    fn advance_shallow(&mut self, _target: i32) -> Result<()> {
        todo!()
    }

    type ImpactsType = ImpactsEnums;

    fn get_impacts(&self) -> Result<&Self::ImpactsType> {
        todo!()
    }
}

impl ImpactsEnum for ImpactsEnumEnum {}
