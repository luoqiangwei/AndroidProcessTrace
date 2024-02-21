// Copyright (c) 2024, 🌟夕元 & 🌟VEA
// All Rights Reserved
// 
// This file is part of LinuxProcessTrace distributed under the BSD 3-Clause License. 
// See the LICENSE file at the root directory of this project for more details.

//! The `process_trace` crate provides functionality for tracing processes.
//! 
//! It uses the `procutils` module to analyze and monitor processes on a Linux system.
//! Example usage:
//! 
//! ```ignore
//! let monitor_list: Vec<&str> = vec!["init"];
//! procutils::proc_analysis::trace_process(60, 10, &monitor_list);
//! ```

pub use procutils::*;

fn main() {
    // Modify this... To trace process
    let monitor_list: Vec<&str> = vec!["second_stage"];
    procutils::proc_analysis::trace_process(60, 10, &monitor_list);
}
