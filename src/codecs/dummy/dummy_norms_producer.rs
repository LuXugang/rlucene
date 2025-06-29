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

use crate::codecs::dummy::dummy_numeric_doc_values::DummyNumericDocValues;
use crate::codecs::norms_producer::NormsProducer;
use crate::index::field_info::FieldInfo;

pub struct DummyNormsProducer;
impl NormsProducer for DummyNormsProducer {
    type NumericDocValues = DummyNumericDocValues;

    fn get_norms(
        &mut self,
        _field: &Rc<FieldInfo>,
    ) -> crate::util::error::lucene_error::Result<Self::NumericDocValues> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }

    fn check_integrity(&mut self) -> crate::util::error::lucene_error::Result<()> {
        unreachable!("Dummy implementation: this method should never be called in real usage")
    }
}
