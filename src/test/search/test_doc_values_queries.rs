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
use crate::core::document::numeric_doc_values_field::numeric_doc_values_field_util;
use crate::core::util::error::lucene_error::Result;

#[allow(dead_code)]
struct TestDocValuesQueries;

#[test]
fn test_set_equals() -> Result<()> {
    assert_eq!(
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 42])?,
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 42])?
    );

    assert_eq!(
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 42, 32416190071])?,
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 32416190071, 42])?
    );

    assert_ne!(
        numeric_doc_values_field_util::new_slow_set_query("field", vec![42])?,
        numeric_doc_values_field_util::new_slow_set_query("field2", vec![42])?
    );

    assert_ne!(
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 42])?,
        numeric_doc_values_field_util::new_slow_set_query("field", vec![17, 32416190071])?
    );

    Ok(())
}
