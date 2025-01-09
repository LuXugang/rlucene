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
use crc32fast::Hasher;
use rand::Rng;
use rlucene::store::{BufferedChecksum, Checksum, HasherChecksum};

#[allow(dead_code)] // for quick search
struct TestBufferedChecksum {}
#[test]
fn test_simple() {
    let mut crc = Hasher::new();
    crc.update(&[1]);
    crc.update(&[2]);
    crc.update(&[3]);

    let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));
    buffered.update(1);
    buffered.update(2);
    buffered.update(3);

    assert_eq!(buffered.get_value(), crc.finalize() as u64);
}

#[test]
fn test_random() {
    let mut raw_crc = Hasher::new();
    let mut buffered = BufferedChecksum::new(HasherChecksum::new(Hasher::new()));

    let mut rng = rand::thread_rng();
    let iterations = 10000;

    for _ in 0..iterations {
        match rng.gen_range(0..4) {
            0 => {
                let length = rng.gen_range(0..1024);
                let mut bytes = vec![0; length];
                rng.fill(bytes.as_mut_slice());
                raw_crc.update(&bytes);
                buffered.update_bytes(&bytes, 0, length as u32);
            }
            1 => {
                let b = rng.gen_range(0..=255) as u8;
                raw_crc.update(&[b]);
                buffered.update(b);
            }
            2 => {
                raw_crc = Hasher::new();
                buffered.reset();
            }
            3 => {
                assert_eq!(buffered.get_value(), raw_crc.clone().finalize() as u64);
            }
            _ => unreachable!(),
        }
    }

    assert_eq!(buffered.get_value(), raw_crc.finalize() as u64);
}
// TODO: not finished
