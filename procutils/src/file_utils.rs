// Copyright (c) 2024, 🌟夕元 & 🌟VEA
// All Rights Reserved
// 
// This file is part of LinuxProcessTrace distributed under the BSD 3-Clause License. 
// See the LICENSE file at the root directory of this project for more details.

use std::fs;
use std::io;

/// read a file
pub fn read_path(path: &str) -> io::Result<String> {
    let result = fs::read_to_string(path)?;
    Ok(result)
}
