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
mod tasks;

use std::path::{Path, PathBuf};
use std::{env, fs, process};

fn find_file(dir: &Path, file_name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).expect("Failed to read directory") {
        let entry = entry.expect("Invalid directory entry");
        let path = entry.path();

        if path.is_file() && path.file_name().map_or(false, |name| name == file_name) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, file_name) {
                return Some(found);
            }
        }
    }
    None
}
fn main() {
    let project_dir = env::current_dir().unwrap();
    let xtask_dir = project_dir.join("xtask");
    let task_file = find_file(&xtask_dir, "tasks.txt").expect("Task file not found: tasks.txt");

    if !task_file.exists() {
        eprintln!("Task file not found: tasks.txt");
        process::exit(1);
    }
    let content = fs::read_to_string(task_file).expect("Failed to read tasks.txt");
    for line in content.lines() {
        let task = line.trim();
        if task.is_empty() || task.starts_with('#') {
            // skip empty lines and comments
            continue;
        }

        println!("Executing task: {}", task);

        let src_dir = project_dir.join("src");
        let test_dir = project_dir.join("tests");
        match task {
            "license-check" => {
                let license_path = find_file(&xtask_dir, "LICENSE_HEADER");
                let license_header_path: String =
                    license_path.as_ref().unwrap().to_str().unwrap().to_string();
                if license_path.is_none() {
                    eprintln!("LICENSE_HEADER file not found: LICENSE_HEADER");
                    process::exit(1);
                }

                let license_text =
                    tasks::license::license::load_license_text(license_path.unwrap().as_path());

                println!("Checking licenses in src/ and test/...");

                let src_valid =
                    tasks::license::license::check_licenses_in_dir(&src_dir, &license_text);
                let test_valid =
                    tasks::license::license::check_licenses_in_dir(&test_dir, &license_text);

                if src_valid && test_valid {
                    println!("All files have the correct license header.");
                } else {
                    eprintln!(
                        "License check failed: you should copy the correct license header from {}",
                        license_header_path
                    );
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown task: {}", task);
            }
        }
    }
}
