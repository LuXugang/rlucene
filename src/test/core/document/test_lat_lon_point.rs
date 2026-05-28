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

use crate::core::document::lat_lon_point::LatLonPoint;
use crate::core::search::query::QueryBase;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)] // for quick search
struct TestLatLonPoint;
#[test]
fn test_to_string() -> Result<()> {
  assert_eq!(
    "LatLonPoint <field:18.313693958334625,-65.22744401358068>",
    LatLonPoint::new("field", 18.313694, -65.227444)?.to_string()
  );

  assert_eq!(
    "field:[18.000000016763806 TO 18.999999999068677],[-65.9999999217689 TO -65.00000006519258]",
    LatLonPoint::new_box_query("field", 18.0, 19.0, -66.0, -65.0)?.as_string("")?
  );

  assert_eq!(
    "field:18.0,19.0 +/- 25.0 meters",
    LatLonPoint::new_distance_query("field", 18.0, 19.0, 25.0)?.as_string("")?
  );

  Ok(())
}
