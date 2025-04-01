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
use crate::index::base_terms_enum::IOBooleanSupplierImpl;
use crate::index::dummy::dummy_io_boolean_supplier::DummyIOBooleanSupplier;
use crate::index::terms_enum::TermsEnum;
use crate::index::BytesRef;
use crate::util::access::Shared;
use crate::util::error::lucene_error::Result;
pub trait IOBooleanSupplier {
    fn get(&mut self) -> Result<bool>;
}
pub enum IOBooleanSupplierEnum<T, S>
where
    T: TermsEnum<S>,
    S: Shared<BytesRef>,
{
    Dummy(DummyIOBooleanSupplier),
    Impl1(IOBooleanSupplierImpl<T, S>),
}
impl<T, S> IOBooleanSupplier for IOBooleanSupplierEnum<T, S>
where
    T: TermsEnum<S>,
    S: Shared<BytesRef>,
{
    fn get(&mut self) -> Result<bool> {
        todo!()
    }
}
