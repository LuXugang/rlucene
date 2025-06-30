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
