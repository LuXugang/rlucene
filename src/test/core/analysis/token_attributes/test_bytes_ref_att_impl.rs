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
use crate::core::analysis::token_attributes::bytes_term_attribute::BytesTermAttribute;
use crate::core::analysis::token_attributes::bytes_term_attribute_impl::BytesTermAttributeImpl;
use crate::core::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;
use crate::core::index::BytesRef;
use crate::core::util::attribute_impl::AttributeImpl;
use crate::core::util::error::lucene_error::Result;
use std::hash::{DefaultHasher, Hash, Hasher};
#[allow(dead_code)]
struct TestBytesRefAttImpl;
#[test]
fn test_copy_to() -> Result<()> {
  let mut t = BytesTermAttributeImpl::new();
  let mut copy = assert_copy_is_equal(&t)?;

  // first do empty
  assert_eq!(t.get_bytes_ref()?, copy.get_bytes_ref()?);
  assert!(copy.get_bytes_ref()?.is_none());

  // now after setting it
  t.set_bytes_ref(Some(BytesRef::from_string("hello")))?;
  copy = assert_copy_is_equal(&t)?;
  assert_eq!(t.get_bytes_ref()?, copy.get_bytes_ref()?);
  // no need check same instance

  Ok(())
}
fn assert_copy_is_equal(att: &BytesTermAttributeImpl) -> Result<BytesTermAttributeImpl> {
  let mut copy = BytesTermAttributeImpl::new();
  att.copy_to(&mut copy)?;
  assert!(att == &copy, "Copied instance must be equal");

  let mut h1 = DefaultHasher::new();
  att.hash(&mut h1);
  let mut h2 = DefaultHasher::new();
  copy.hash(&mut h2);

  assert_eq!(
    h1.finish(),
    h2.finish(),
    "Copied instance's hashcode must be equal"
  );

  Ok(copy)
}

#[test]
#[ignore = "Java-only: reflection over directly declared interfaces has no Rust equivalent"]
fn test_lucene9856() -> Result<()> {
  test_not_required_in_rust_lucene!();
}
