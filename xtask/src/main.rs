/*
 * MIT License
 *
 * Copyright (c) 2025 Lu Xugang
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
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
        // let test_dir = project_dir.join("tests");
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
                    tasks::license::license_checker::load_license_text(license_path.unwrap().as_path());

                println!("Checking licenses in src/ and test/...");

                let src_valid =
                    tasks::license::license_checker::check_licenses_in_dir(&src_dir, &license_text);
                // let test_valid =
                //     tasks::license::license_checker::check_licenses_in_dir(&test_dir, &license_text);

                // if src_valid && test_valid {
                    if src_valid{
                    eprintln!("\x1b[32mAll files have the correct license header\x1b[0m.");
                } else {
                    eprintln!(
                        "License check failed: you should copy the correct license header from \x1b[31m{}\x1b[0m",
                        license_header_path
                    );
                    process::exit(1);
                }
            }

            _ => {
                eprintln!("\x1b[31mUnknown task: {}\x1b[31m", task);
            }
        }
    }
}
