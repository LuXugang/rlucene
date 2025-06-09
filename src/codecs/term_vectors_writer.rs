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
use crate::index::field_info::FieldInfo;
use crate::index::{BytesRef, BytesRefBuilder};
use crate::store::DataInput;
use crate::util::accountable::Accountable;
use crate::util::error::lucene_error::Result;

pub trait TermVectorsWriter: Accountable {
    fn start_document(&mut self, num_vector_fields: i32) -> Result<()>;

    fn finish_document(&mut self) -> Result<()> {
        Ok(())
    }
    fn start_field(
        &mut self,
        field_info: &FieldInfo,
        num_terms: usize,
        positions: bool,
        offsets: bool,
        payloads: bool,
    ) -> Result<()>;

    fn finish_field(&mut self) -> Result<()> {
        Ok(())
    }

    fn start_term(&mut self, term: &BytesRef<Vec<u8>>, freq: i32) -> Result<()>;

    fn finish_term(&mut self) -> Result<()> {
        Ok(())
    }

    fn add_position(
        &mut self,
        position: i32,
        start_offset: i32,
        end_offset: i32,
        payload: Option<&BytesRef<Vec<u8>>>,
    ) -> Result<()>;

    fn finish(&mut self, num_docs: i32) -> Result<()>;

    fn add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
    ) -> Result<()>;

    fn default_add_prox(
        &mut self,
        num_prox: usize,
        positions: &mut Option<impl DataInput>,
        offsets: &mut Option<impl DataInput>,
    ) -> Result<()> {
        let mut position = 0;
        let mut last_offset = 0;
        let mut payload: Option<BytesRefBuilder<Vec<u8>>> = None;

        for _ in 0..num_prox {
            let (start_offset, end_offset);
            let this_payload;

            if let Some(pos_input) = positions.as_mut() {
                let code = pos_input.read_vint()?;
                position += (code as u32 >> 1) as i32;

                if code & 1 != 0 {
                    // This position has a payload
                    let payload_len = pos_input.read_vint()? as usize;

                    if payload.is_none() {
                        payload = Some(BytesRefBuilder::new());
                    }
                    let builder = payload.as_mut().unwrap();
                    builder.grow_no_copy(payload_len);
                    pos_input.read_bytes(&mut builder.bytes_ref.bytes, 0, payload_len as i32)?;
                    builder.set_length(payload_len);
                    this_payload = Some(builder.get_bytes_ref());
                } else {
                    this_payload = None;
                }
            } else {
                position = -1;
                this_payload = None;
            }

            if let Some(off_input) = offsets.as_mut() {
                start_offset = last_offset + off_input.read_vint()?;
                end_offset = start_offset + off_input.read_vint()?;
                last_offset = end_offset;
            } else {
                start_offset = -1;
                end_offset = -1;
            }

            self.add_position(position, start_offset, end_offset, this_payload)?;
        }

        Ok(())
    }
}
