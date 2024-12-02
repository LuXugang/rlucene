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
use crate::store::flush_info::FlushInfo;
use crate::store::merge_info::MergeInfo;
use crate::store::ReadAdvice;
use crate::util::error::runtime_error::RuntimeError;

/**
 * IOContext holds additional details on the merge/search context. An IOContext object can never be
 * passed as a `NONE` parameter to either `Directory#openInput` or `Directory#createOutput`
 */
#[derive(Clone)]
pub struct IOContext {
    context: Context,
    read_advice: ReadAdvice,
    merge_info: Option<MergeInfo>,
    flush_info: Option<FlushInfo>,
}

#[derive(Clone)]
pub enum Context {
    /** Context for reads and writes that are associated with a merge. */
    Merge,
    /** Context for writes that are associated with a segment flush. */
    Flush,
    /** Default context, can be used for reading or writing. */
    Default,
}

impl IOContext {
    pub fn new(
        context: Option<Context>,
        read_advice: Option<ReadAdvice>,
        merge_info: Option<MergeInfo>,
        flush_info: Option<FlushInfo>,
    ) -> Result<IOContext, RuntimeError> {
        let context = context.ok_or(RuntimeError::illegal_argument(
            "context must not be None".to_string(),
        ))?;
        let read_advice = read_advice.ok_or(RuntimeError::illegal_argument(
            "read_advice must not be None".to_string(),
        ))?;
        if matches!(context, Context::Merge) && merge_info.is_none() {
            return Err(RuntimeError::illegal_argument(
                "merge_info must not be None if context is MERGE".to_string(),
            ));
        }
        if matches!(context, Context::Flush) && flush_info.is_none() {
            return Err(RuntimeError::illegal_argument(
                "flush_info must not be None if context is FLUSH".to_string(),
            ));
        }
        if (matches!(context, Context::Flush) || matches!(context, Context::Merge))
            && matches!(read_advice, ReadAdvice::Sequential)
        {
            return Err(RuntimeError::illegal_argument(
                "The FLUSH and MERGE contexts must use the SEQUENTIAL read access advice"
                    .to_string(),
            ));
        }
        Ok(Self {
            context,
            read_advice,
            merge_info,
            flush_info,
        })
    }
    /** Creates a default IOContext for reading/writing with the given `ReadAdvice` */
    fn new_with_read_advice(read_advice: ReadAdvice) -> Result<IOContext, RuntimeError> {
        Self::new(None, Some(read_advice), None, None)
    }

    /** Creates an `IOContext` for flushing. */
    fn new_with_flush(flush_info: FlushInfo) -> Result<IOContext, RuntimeError> {
        Self::new(
            Some(Context::Flush),
            Some(ReadAdvice::Sequential),
            None,
            Some(flush_info),
        )
    }
    /** Creates an `IOContext` for merging. */
    fn new_with_merge(merge_info: MergeInfo) -> Result<IOContext, RuntimeError> {
        Self::new(
            Some(Context::Merge),
            Some(ReadAdvice::Sequential),
            Some(merge_info),
            None,
        )
    }

    fn with_read_advice(&self, read_advice: ReadAdvice) -> Result<IOContext, RuntimeError> {
        if matches!(self.context, Context::Default) {
            // TODO: maybe should statically define all types of context
            Self::new_with_read_advice(read_advice)
        } else {
            Ok(self.clone())
        }
    }
}
