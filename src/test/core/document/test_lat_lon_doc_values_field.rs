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

use crate::core::document::lat_lon_doc_values_field::LatLonDocValuesField;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestLatLonDocValuesField;
#[test]
fn test_to_string() -> Result<()> {
  assert_eq!(
    "LatLonDocValuesField <field:18.313693958334625,-65.22744401358068>",
    LatLonDocValuesField::new("field", 18.313694, -65.227444)?.to_string()
  );

  assert_eq!(
    "<distance:\"field\" latitude=18 longitude=19>",
    LatLonDocValuesField::new_distance_sort("field", 18.0, 19.0)?.to_string()
  );

  Ok(())
}
