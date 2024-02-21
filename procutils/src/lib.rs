// Copyright (c) 2024, 🌟夕元 & 🌟VEA
// All Rights Reserved
// 
// This file is part of LinuxProcessTrace distributed under the BSD 3-Clause License. 
// See the LICENSE file at the root directory of this project for more details.

//! The `processutils` library provides utilities for process analysis.
//!
//! It includes two modules:
//! - The `file_utils` module, used for file operations.
//! - The `proc_analysis` module, provides utilities for analyzing the process.

/// This module is used for file operate.
/// 
/// use to operate file
pub mod file_utils;

/// This module is used for process analysis.
/// 
/// It provides various utilities to analyze the process running on a computer,
/// such as CPU usage, memory consumption and etc.
pub mod proc_analysis;
