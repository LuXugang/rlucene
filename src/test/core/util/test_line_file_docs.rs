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
use chrono::{NaiveDate, NaiveTime};

use crate::core::util::error::lucene_error::Result;
use crate::test_framework::core::util::line_file_docs::DATE_FIELD_VALUE_TO_LOCALDATETIME;

#[allow(dead_code)] // for quick search
struct TestLineFileDocs;
#[test]
fn test_date_field_normalization() -> Result<()> {
  // europarl corpus uses this data format.
  assert_eq!(
    NaiveDate::from_ymd_opt(2023, 2, 23)
      .unwrap()
      .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
    DATE_FIELD_VALUE_TO_LOCALDATETIME("2023-02-23")?
  );
  // enwiki uses this data format.
  assert_eq!(
    NaiveDate::from_ymd_opt(2010, 1, 12)
      .unwrap()
      .and_time(NaiveTime::from_hms_opt(12, 32, 45).unwrap()),
    DATE_FIELD_VALUE_TO_LOCALDATETIME("12-JAN-2010 12:32:45.000")?
  );
  Ok(())
}
