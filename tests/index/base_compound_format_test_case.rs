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
use crate::util::lucene_test_case::new_directory;
use crate::util::test_error::TestError;
use rand::rngs::StdRng;
use rand::Rng;
use rlucene::codecs::{Codec, CompoundFormat, LATEST_CODEC};
use rlucene::index::segment_info::SegmentInfo;
use rlucene::store::directory::Directory;
use rlucene::store::IO_CONTEXT_DEFAULT;
use rlucene::util::{StringHelper, LATEST};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub trait BaseCompoundFormatTestCase {
    fn test_empty(&self, random: &mut StdRng) -> Result<(), TestError> {
        let dir = Arc::new(Mutex::new(new_directory(random)?));
        let mut si = new_segment_info(random, dir.clone(), "_123")?;
        si.set_files(HashSet::new());
        LATEST_CODEC
            .compound_format()
            .write(dir.clone(), &si, &IO_CONTEXT_DEFAULT)?;
        let cfs = LATEST_CODEC
            .compound_format()
            .get_compound_reader(dir.clone(), &si)?;
        assert_eq!(0, cfs.list_all()?.len());
        Ok(())
    }
}

fn new_segment_info<D: Directory>(
    random: &mut StdRng,
    dir: Arc<Mutex<D>>,
    name: &str,
) -> Result<SegmentInfo<D>, TestError> {
    let min_version = if random.gen_bool(0.5) {
        None
    } else {
        Some((*LATEST).clone())
    };
    let id = StringHelper::random_id();
    let value = SegmentInfo::new(
        dir,
        Some((*LATEST).clone()),
        min_version,
        name.to_string(),
        Option::from(10_000),
        false,
        false,
        HashMap::new(),
        Vec::from(id),
        HashMap::new(),
        None,
    )?;
    Ok(value)
}
