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
use std::fs::File;
use std::io;
use std::io::Write;

pub enum OutputEnum {
    File(File),
    Stdout,
    Stderr,
}

impl Write for OutputEnum {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputEnum::File(file) => file.write(buf),
            OutputEnum::Stdout => io::stdout().write(buf),
            OutputEnum::Stderr => io::stderr().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputEnum::File(file) => file.flush(),
            OutputEnum::Stdout => io::stdout().flush(),
            OutputEnum::Stderr => io::stderr().flush(),
        }
    }
}
