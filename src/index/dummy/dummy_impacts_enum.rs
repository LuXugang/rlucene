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
use crate::index::dummy::dummy_impacts::DummyImpacts;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::impacts_source::ImpactsSource;
use crate::index::postings_enum::PostingsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::{LuceneError, Result};

pub struct DummyImpactsEnum;

impl PostingsEnum for DummyImpactsEnum {
    fn freq(&mut self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn next_position(&mut self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn start_offset(&self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn end_offset(&self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn get_payload(&self) -> Result<Option<BytesRef>> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}

impl DocIdSetIterator for DummyImpactsEnum {
    fn doc_id(&self) -> i32 {
        unreachable!("this method should never be called")
    }

    fn next_doc(&mut self) -> Result<i32> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }
}

impl ImpactsSource for DummyImpactsEnum {
    fn advance_shallow(&mut self, _target: i32) -> Result<()> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    fn get_impacts(&self) -> Result<&Self::ImpactsType> {
        Err(LuceneError::illegal_state(
            "this method should never be called",
        ))
    }

    type ImpactsType = DummyImpacts;
}

impl ImpactsEnum for DummyImpactsEnum {}
