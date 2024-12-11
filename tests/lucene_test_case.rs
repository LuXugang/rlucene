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
use rlucene::store::fs_directory_base::FSDirectoryBase;
use rlucene::store::lock_factory::LockFactory;

pub struct LuceneTestCase<D, T>
where
    D: LockFactory,
    T: FSDirectoryBase,
{
    _marker: std::marker::PhantomData<(D, T)>,
}

impl<D, T> LuceneTestCase<D, T>
where
    D: LockFactory,
    T: FSDirectoryBase,
{
    // TODO: When we have implemented multiple directories, we need to select one randomly. Currently, we choose NIOFSDirectory.
    // fn new_directory() -> Result<FSDirectory<D, T>, TestError>{
    //     let temp_dir = TempDir::new()?;
    //     todo!()
    // }
}
