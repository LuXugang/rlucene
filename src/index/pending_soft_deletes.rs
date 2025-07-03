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
pub(crate) struct PendingSoftDeletes;

pub(crate) mod pending_soft_deletes_util {
    use crate::search::doc_id_set_iterator::disi_const::NO_MORE_DOCS;
    use crate::search::doc_id_set_iterator::DocIdSetIterator;
    use crate::util::bits::Bits;
    use crate::util::error::lucene_error::Result;
    pub(crate) fn count_soft_deletes(
        soft_deleted_docs: Option<&mut impl DocIdSetIterator>,
        hard_deletes: Option<&impl Bits>,
    ) -> Result<i32> {
        let mut count = 0;
        if let Some(docs) = soft_deleted_docs {
            loop {
                let doc = docs.next_doc()?;
                if doc == NO_MORE_DOCS {
                    break;
                }
                if hard_deletes.is_none_or(|bits| bits.get(doc)) {
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}
