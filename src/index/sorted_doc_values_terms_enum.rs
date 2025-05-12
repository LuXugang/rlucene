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

/// Implements a [`TermsEnum`](TermsEnum) wrapping a provided
/// [`SortedDocValues`](SortedDocValues).
pub struct SortedDocValuesTermsEnum;
// where
//     I: IndexInput,
//     AV: AccessVec<u8>,
// {
//     values: SortedDocValues<I, AV>,
//     current_ord: i32,
//     bytes: BytesRef<AV>,
// }
//
// impl<I, AV> SortedDocValuesTermsEnum<I, AV>
// where
//     I: IndexInput,
//     AV: AccessVec<u8>,
// {
//     /// Creates a new TermsEnum over the provided values.
//     pub fn new(values: SortedDocValuesEnum<I, AV>) -> Self {
//         Self {
//             values,
//             current_ord: -1,
//             bytes: BytesRef::new(),
//         }
//     }
// }
//
// impl<I, AV> BytesRefIterator<AV> for SortedDocValuesTermsEnum<I, AV>
// where
//     I: IndexInput,
//     AV: AccessVec<u8>,
// {
//     fn next(&mut self) -> Result<Option<Cow<BytesRef<AV>>>> {
//         self.current_ord += 1;
//         if self.current_ord >= self.values.get_value_count()? {
//             Ok(None)
//         } else {
//             match self.values.lookup_ord(self.current_ord)? {
//                 Cow::Owned(bytes) => {
//                     self.bytes = bytes;
//                 }
//                 Cow::Borrowed(bytes) => {
//                     self.bytes = bytes.clone();
//                 }
//             }
//             Ok(Some(Cow::Borrowed(&self.bytes)))
//         }
//     }
// }
//
// impl<I, AV> TermsEnum<AV> for SortedDocValuesTermsEnum<I, AV>
// where
//     I: IndexInput,
//     AV: AccessVec<u8>,
// {
//     fn attributes(&self) -> Result<&AttributeSource> {
//         Err(LuceneError::not_implemented(""))
//     }
//
//     fn seek_exact(&mut self, text: &BytesRef<AV>) -> Result<bool> {
//         let ord = self.values.lookup_term(text)?;
//         if ord >= 0 {
//             self.current_ord = ord;
//             self.bytes = text.clone();
//             Ok(true)
//         } else {
//             Ok(false)
//         }
//     }
//
//     fn seek_ceil(&mut self, text: &BytesRef<AV>) -> Result<SeekStatus> {
//         let ord = self.values.lookup_term(text)?;
//         if ord >= 0 {
//             self.current_ord = ord;
//             self.bytes = text.clone();
//             Ok(SeekStatus::Found)
//         } else {
//             self.current_ord = -ord - 1;
//             if self.current_ord == self.values.get_value_count()? {
//                 Ok(SeekStatus::End)
//             } else {
//                 // TODO: hmm, can we avoid this extra lookup?
//                 match self.values.lookup_ord(self.current_ord)? {
//                     Cow::Owned(bytes) => {
//                         self.bytes = bytes;
//                     }
//                     Cow::Borrowed(bytes) => {
//                         self.bytes = bytes.clone();
//                     }
//                 }
//                 Ok(SeekStatus::NotFound)
//             }
//         }
//     }
//
//     fn seek_exact_with_ord(&mut self, ord: i64) -> Result<()> {
//         debug_assert!(ord >= 0 && ord < self.values.get_value_count()? as
// i64);         self.current_ord = ord as i32;
//         match self.values.lookup_ord(self.current_ord)? {
//             Cow::Owned(bytes) => {
//                 self.bytes = bytes;
//             }
//             Cow::Borrowed(bytes) => {
//                 self.bytes = bytes.clone();
//             }
//         }
//         Ok(())
//     }
//
//     fn seek_exact_with_state(&mut self, _term: &BytesRef<AV>, state:
// &TermStateEnum) -> Result<()> {         debug_assert!({ matches!(state,
// TermStateEnum::Ord(_)) });         match state {
//             TermStateEnum::Ord(ord_term_state) =>
// self.seek_exact_with_ord(ord_term_state.ord)?,             _ => return
// Err(LuceneError::illegal_state("state should be OrdTermState")),         }
//         Ok(())
//     }
//
//     fn term(&self) -> Result<Cow<BytesRef<AV>>> {
//         Ok(Cow::Borrowed(&self.bytes))
//     }
//
//     fn ord(&self) -> Result<i64> {
//         Ok(self.current_ord as i64)
//     }
//
//     fn doc_freq(&self) -> Result<i32> {
//         Err(LuceneError::unsupported_operation(""))
//     }
//
//     fn total_term_freq(&self) -> Result<i64> {
//         Err(LuceneError::unsupported_operation(""))
//     }
//
//     type PostingsEnum = DummyPostingsEnum;
//
//     fn postings_with_flags(
//         &mut self,
//         _reuse: Option<Self::PostingsEnum>,
//         _flags: i32,
//     ) -> Result<Self::PostingsEnum> {
//         Err(LuceneError::unsupported_operation(""))
//     }
//
//     type ImpactsEnum = DummyImpactsEnum;
//
//     fn impacts(&mut self, _flags: i32) -> Result<Self::ImpactsEnum> {
//         Err(LuceneError::unsupported_operation(""))
//     }
//
//     type TermState = TermStateEnum;
//
//     fn term_state(&self) -> Result<Self::TermState> {
//         let mut state = OrdTermState::default();
//         state.ord = self.current_ord as i64;
//         Ok(TermStateEnum::Ord(state))
//     }
// }
