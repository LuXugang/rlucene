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

use crate::core::document::float_range::FloatRange;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestFloatRange;

#[test]
fn test_to_string_float_range() -> Result<()> {
  let range = FloatRange::new(
    "foo",
    &[0.1_f32, 1.1_f32, 2.1_f32, 3.1_f32],
    &[0.2_f32, 1.2_f32, 2.2_f32, 3.2_f32],
  )?;

  assert_eq!(
    "FloatRange <foo: [0.1 : 0.2] [1.1 : 1.2] [2.1 : 2.2] [3.1 : 3.2]>",
    range.to_string()
  );
  Ok(())
}
