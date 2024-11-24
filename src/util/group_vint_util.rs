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
use crate::store::data_input::DataInput;
use crate::store::data_io_error_enum::DataIOErrorEnum;
use crate::store::data_output::DataOutput;

pub const MAX_LENGTH_PER_GROUP: usize = 17;

pub struct GroupVIntUtil;

impl GroupVIntUtil {
    pub fn read_group_vint<T: DataInput>(
        _data_input: &T,
        _dst: &mut [i64],
        _offset: i32,
    ) -> Result<(), DataIOErrorEnum> {
        todo!()
    }

    pub fn write_group_vint<T: DataOutput>(
        _data_output: &T,
        _scratch: &mut [u8],
        _values: &mut [i64],
        _limit: i32,
    ) -> Result<(), DataIOErrorEnum> {
        todo!()
    }
}
