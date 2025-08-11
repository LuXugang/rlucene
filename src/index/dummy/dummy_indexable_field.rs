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
use crate::analysis::analyzer::Analyzer;
use crate::analysis::dummy::dummy_token_stream::DummyTokenStream;
use crate::document::fields::ReaderEnum;
use crate::document::invertable_field::InvertableType;
use crate::document::stored_value::StoredValue;
use crate::index::BytesRef;
use crate::index::dummy::dummy_indexable_field_type::DummyIndexableFieldType;
use crate::index::indexable_field::IndexableField;
use crate::util::number::Number;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

pub struct DummyIndexableField;

impl Display for DummyIndexableField {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}

impl IndexableField for DummyIndexableField {
    fn name(&self) -> &str {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type FieldType = DummyIndexableFieldType;

    fn field_type(&self) -> &Self::FieldType {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    type TokenStream = DummyTokenStream;

    fn token_stream<A>(
        &self,
        _analyzer: &A,
        _reuse: Option<Self::TokenStream>,
    ) -> crate::util::error::lucene_error::Result<Option<Self::TokenStream>>
    where
        A: Analyzer,
    {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn binary_value(
        &self,
    ) -> crate::util::error::lucene_error::Result<Option<Rc<BytesRef<Vec<u8>>>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn string_value(&self) -> crate::util::error::lucene_error::Result<Option<Rc<String>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn get_char_sequence_value(
        &self,
    ) -> crate::util::error::lucene_error::Result<Option<Rc<String>>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn reader_value(&self) -> crate::util::error::lucene_error::Result<Option<ReaderEnum>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn numeric_value(&self) -> crate::util::error::lucene_error::Result<Option<Number>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn stored_value(&self) -> crate::util::error::lucene_error::Result<Option<StoredValue>> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn invertable_type(&self) -> &InvertableType {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn is_reserved(&self) -> bool {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
