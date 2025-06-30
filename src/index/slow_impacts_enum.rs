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
use crate::index::impact::Impact;
use crate::index::impacts::Impacts;
use crate::index::impacts_enum::ImpactsEnum;
use crate::index::impacts_source::ImpactsSource;
use crate::index::postings_enum::PostingsEnum;
use crate::index::BytesRef;
use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
use crate::search::doc_id_set_iterator::DocIdSetIterator;
use crate::util::error::lucene_error::Result;
use std::borrow::Cow;
/// [`ImpactsEnum`] that doesn't index impacts but implements the API in a legal way.
/// This is typically used for short postings that do not need skipping.
pub struct SlowImpactsEnum<P>
where
    P: PostingsEnum,
{
    delegate: P,
}
impl<P> SlowImpactsEnum<P>
where
    P: PostingsEnum,
{
    pub fn new(delegate: P) -> Self {
        SlowImpactsEnum { delegate }
    }
}

impl<P> PostingsEnum for SlowImpactsEnum<P>
where
    P: PostingsEnum,
{
    fn freq(&mut self) -> Result<i32> {
        self.delegate.freq()
    }

    fn next_position(&mut self) -> Result<i32> {
        self.delegate.next_position()
    }

    fn start_offset(&self) -> Result<i32> {
        self.delegate.start_offset()
    }

    fn end_offset(&self) -> Result<i32> {
        self.delegate.end_offset()
    }

    fn get_payload(&self) -> Result<Option<Cow<BytesRef<Vec<u8>>>>> {
        self.delegate.get_payload()
    }
}

impl<P> DocIdSetIterator for SlowImpactsEnum<P>
where
    P: PostingsEnum,
{
    fn doc_id(&self) -> i32 {
        self.delegate.doc_id()
    }

    fn next_doc(&mut self) -> Result<i32> {
        self.delegate.next_doc()
    }

    fn cost(&self) -> Result<i64> {
        self.delegate.cost()
    }
}

impl<P> ImpactsSource for SlowImpactsEnum<P>
where
    P: PostingsEnum,
{
    fn advance_shallow(&mut self, target: i32) -> Result<()> {
        Ok(())
    }

    type Impacts = DummyImpacts;

    fn get_impacts(&mut self) -> Result<Self::Impacts> {
        Ok(DummyImpacts::new())
    }
}

impl<P> ImpactsEnum for SlowImpactsEnum<P> where P: PostingsEnum {}

pub struct DummyImpacts {
    impacts: Vec<Impact>,
}
impl Default for DummyImpacts {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyImpacts {
    pub fn new() -> Self {
        DummyImpacts {
            impacts: vec![Impact::new(i32::MAX, 0)],
        }
    }
}
impl Impacts for DummyImpacts {
    fn num_levels(&self) -> i32 {
        1
    }

    fn get_doc_id_up_to(&self, _level: i32) -> i32 {
        NO_MORE_DOCS
    }

    fn get_impacts(&mut self, _level: i32) -> Result<Cow<[Impact]>> {
        Ok(Cow::Borrowed(self.impacts.as_slice()))
    }
}
