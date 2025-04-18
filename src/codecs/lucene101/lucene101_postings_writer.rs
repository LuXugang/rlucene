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
pub struct Lucene101PostingsWriter;

pub mod lucene101_pw_util {
    use crate::index::impact::Impact;
    use crate::store::DataOutput;
    use crate::util::error::lucene_error::Result;

    /// Special vints that are encoded on 2 bytes if they require 15 bits or less.
    /// VInt becomes especially slow when the number of bytes is variable, so this
    /// special layout helps in the case when the number likely requires 15 bits or less.
    pub(crate) fn write_vint15(out: &mut impl DataOutput, v: i32) -> Result<()> {
        debug_assert!(v >= 0);
        write_vlong15(out, v as i64)
    }

    /// @see [`write_vint15`]
    pub(crate) fn write_vlong15(out: &mut impl DataOutput, v: i64) -> Result<()> {
        debug_assert!(v >= 0);
        if v & !0x7FFF == 0 {
            out.write_short(v as i16)?;
        } else {
            let prefix = 0x8000 | (v & 0x7FFF);
            out.write_short(prefix as i16)?;
            out.write_vlong(v >> 15)?;
        }
        Ok(())
    }
    pub(crate) fn write_impacts(impacts: &[Impact], out: &mut impl DataOutput) -> Result<()> {
        let mut previous = Impact { freq: 0, norm: 0 };
        for impact in impacts {
            debug_assert!(impact.freq > previous.freq);
            debug_assert!((impact.norm as u64) > (previous.norm as u64));
            let freq_delta = impact.freq - previous.freq - 1;
            let norm_delta = impact.norm - previous.norm - 1;
            if norm_delta == 0 {
                // most of time, norm only increases by 1, so we can fold everything in a single byte
                out.write_vint(freq_delta << 1)?;
            } else {
                out.write_vint((freq_delta << 1) | 1)?;
                out.write_zlong(norm_delta)?;
            }
            previous = impact.clone();
        }
        Ok(())
    }
}
