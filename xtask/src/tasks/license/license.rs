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
use std::{fs, path::Path};

pub(crate) fn load_license_text(file_path: &Path) -> String {
    fs::read_to_string(file_path).expect("Failed to read license file")
}

fn check_license_in_file(file_path: &Path, license_text: &str) -> bool {
    let content = fs::read_to_string(file_path).expect("Unable to read file");
    content.starts_with(license_text)
}

pub fn check_licenses_in_dir(dir: &Path, license_text: &str) -> bool {
    let mut all_valid = true;

    for entry in fs::read_dir(dir).expect("Unable to read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            all_valid &= check_licenses_in_dir(&path, license_text);
        } else if path.extension().map(|ext| ext == "rs").unwrap_or(false)
            && !check_license_in_file(&path, license_text)
        {
            println!(
                "Missing or incorrect license in file: \x1b[31m{}\x1b[0m",
                path.display()
            );
            all_valid = false;
        }
    }
    all_valid
}
