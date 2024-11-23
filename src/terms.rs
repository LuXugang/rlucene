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
#[allow(dead_code)]
pub(crate) trait Terms {
    /**
     * Returns the number of documents that have at least one term for this field. Note that, just
     * like other term measures, this measure does not take deleted documents into account.
     */
    fn get_doc_count();
    /**
     * Returns the sum of TermsEnum#doc_freq() for all terms in this field. Note that, just
     * like other term measures, this measure does not take deleted documents into account.
     */
    fn get_sum_doc_freq();
}
