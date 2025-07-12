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
use crate::analysis::token_attributes::offset_attribute::OffsetAttribute;
use crate::analysis::token_attributes::payload_attribute::PayloadAttribute;
use crate::analysis::token_attributes::position_increment_attribute::PositionIncrementAttribute;
use crate::analysis::token_attributes::term_frequency_attribute::TermFrequencyAttribute;
use crate::analysis::token_attributes::term_to_bytes_ref_attribute::TermToBytesRefAttribute;

pub trait AttributeSource {
    type OffsetAttribute: OffsetAttribute;
    fn get_offset_attribute(&self) -> &Self::OffsetAttribute {
        unimplemented!("get_offset_attribute()  must be implemented if it needs to be used")
    }

    type PositionIncrementAttribute: PositionIncrementAttribute;
    fn get_position_increment_attribute(&self) -> &Self::PositionIncrementAttribute {
        unimplemented!(
            "get_position_increment_attribute() must be implemented if it needs to be used"
        )
    }
    type PayloadAttribute: PayloadAttribute;
    fn get_payload_attribute(&self) -> &Self::PayloadAttribute {
        unimplemented!("get_payload_attribute() must be implemented if it needs to be used")
    }

    type TermToBytesRefAttribute: TermToBytesRefAttribute;
    fn get_term_to_bytes_ref_attribute(&self) -> &Self::TermToBytesRefAttribute {
        unimplemented!(
            "get_term_to_bytes_ref_attribute() must be implemented if it needs to be used"
        )
    }
    type TermFrequencyAttribute: TermFrequencyAttribute;
    fn get_term_frequency_attribute(&self) -> &Self::TermFrequencyAttribute {
        unimplemented!("get_term_frequency_attribute() must be implemented if it needs to be used")
    }
}
