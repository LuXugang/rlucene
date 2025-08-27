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
use crate::util::error::lucene_error::Result;
pub trait IndexReader {
    fn max_doc(&self) -> Result<i32>;

    fn num_docs(&self) -> Result<i32>;

    fn num_deleted_docs(&self) -> Result<i32> {
        Ok(self.max_doc()? - self.num_docs()?)
    }

    fn inc_ref(&self) -> Result<()> {
        todo!()
    }

    fn dec_ref(&self) -> Result<()> {
        todo!()
    }

    fn ensure_open(&self) -> Result<()> {
        // TODO
        Ok(())
    }

    fn has_deletions(&self) -> Result<bool> {
        Ok(self.num_deleted_docs()? > 0)
    }

    fn do_close(&mut self) -> Result<()>;

    fn check_integrity(&self) -> Result<()>;
}
