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

/**
 * A `Directory` provides an abstraction layer for storing a list of files. A directory
 * contains only files (no sub-folder hierarchy).
 *
 * Implementing classes must comply with the following:
 *
 * A file in a directory can be created `#createOutput`, appended to, then closed.
 * A file open for writing may not be available for read access until the corresponding
 * `IndexOutput` is closed.
 * Once a file is created it must only be opened for input `#openInput`, or deleted
 * `#deleteFile`.
 *
 * NOTE: If your application requires external synchronization, you should `not
 * synchronize on the `Directory` implementation instance as this may cause deadlock; use
 * your own (non-Lucene) objects instead.
 *
 */
#[allow(dead_code)]
pub trait Directory {
    /**
     * Returns names of all files stored in this directory
     *
     */
    fn list_all() -> Vec<String>;
    /**
     * Removes an existing file in the directory.
     *
     */
    fn delete_file(name: &str);

    /**
     * Returns the byte length of a file in the directory.
     *
     */
    fn file_length(name: &str) -> u64;

    /**
     * Creates a new, empty file in the directory and returns an `IndexOutput` instance for
     * appending data to this file.
     */
    fn create_output(name: &str);
}
