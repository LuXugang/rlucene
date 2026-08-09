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
use crate::core::codecs::codec::{self, Codec};
use crate::core::util::error::lucene_error::{LuceneError, Result};

#[allow(dead_code)] // for quick search
struct TestNamedSPILoader;

#[test]
fn test_lookup() -> Result<()> {
  let current_name = codec::get_default().get_name().to_string();
  let codec = codec::for_name(&current_name)?;
  assert_eq!(current_name, codec.get_name());
  Ok(())
}

#[test]
fn test_bogus_lookup() -> Result<()> {
  assert!(matches!(
    codec::for_name("dskfdskfsdfksdfdsf"),
    Err(LuceneError::IllegalArgument(_))
  ));
  Ok(())
}
