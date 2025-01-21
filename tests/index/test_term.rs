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
use rlucene::index::term::Term;

#[allow(dead_code)] // for quick search
pub struct TestTerm;
#[test]
fn test_equals() {
    let base = Term::new_from_text("same".to_string(), "same");
    let same = Term::new_from_text("same".to_string(), "same");
    let different_field = Term::new_from_text("different".to_string(), "same");
    let different_text = Term::new_from_text("same".to_string(), "different");
    assert_eq!(base, base);
    assert_eq!(base, same);
    assert_ne!(base, different_field);
    assert_ne!(base, different_text);
}
