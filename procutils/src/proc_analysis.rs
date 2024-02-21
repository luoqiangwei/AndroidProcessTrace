// Copyright (c) 2024, 🌟夕元 & 🌟VEA
// All Rights Reserved
// 
// This file is part of LinuxProcessTrace distributed under the BSD 3-Clause License. 
// See the LICENSE file at the root directory of this project for more details.

use libc::{pid_t, sysconf, time_t, _SC_CLK_TCK};
use crate::file_utils::read_path;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::thread::{self, sleep};
use std::process::{Command, Output};
use std::str::from_utf8;
use std::time::Duration;

// Procfs some path
const GLOBAL_SYSTEM_INFO: &str = "/proc/stat";

// We need string literal to format String
/// Declare an string
#[macro_export]
macro_rules! TASK_STATUS_TEMPLATE { () => { "/proc/{}/status" } }

/// Declare an string
#[macro_export]
macro_rules! TASK_STAT_TEMPLATE { () => { "/proc/{}/stat" } }

/// Declare an string
#[macro_export]
macro_rules! SUBTASK_PATH_TEMPLATE { () => { "/proc/{}/task" } }

/// Declare an string
#[macro_export]
macro_rules! TASK_STATUS_TID_TEMPLATE { () => { "/proc/{}/task/{}/status" } }

/// Declare an string
#[macro_export]
macro_rules! TASK_STAT_TID_TEMPLATE { () => { "/proc/{}/task/{}/stat" } }

/// Declare an string
#[macro_export]
macro_rules! TASK_SMAPS_PID_TEMPLATE { () => { "/proc/{}/smaps" }; }

// procfs status some data type
const TASK_VM_RSS_PREFIX: &str = "VmRSS:\t";
const TASK_RSS_ANON_PREFIX: &str = "RssAnon:\t";
const TASK_RSS_FILE_PREFIX: &str = "RssFile:\t";
const TASK_RSS_SHMEM_PREFIX: &str = "RssShmem:\t";
const TASK_VM_SWAP_PREFIX: &str = "VmSwap:\t";
const TASK_PSS_PREFIX: &str = "Pss:\t";
const TASK_VOLUNTARY_SWITCH_PREFIX: &str = "voluntary_ctxt_switches:\t";
const TASK_NONVOLUNTARY_SWITCH_PREFIX: &str = "nonvoluntary_ctxt_switches:\t";
const GLOBAL_CPU_STAT_PREFIX: &str = "cpu "; // static mark global lifecycle

macro_rules! OUTPUT_FILE_TEMPLATE { () => { "resource_trace_{}.csv" }; }

// /proc/pid/stat shift
const PROCESS_STAT_MINFLT_SHIFT: usize = 9;
const PROCESS_STAT_MAJFLT_SHIFT: usize = 11;
const PROCESS_STAT_UTIME_SHIFT: usize = 13;
const PROCESS_STAT_STIME_SHIFT: usize = 14;
const PROCESS_STAT_PRIORITY_SHIFT: usize = 17;
const PROCESS_STAT_NICE_SHIFT: usize = 18;
const PROCESS_STAT_NUM_THREADS_SHIFT: usize = 19;
const PROCESS_STAT_STARTTIME_SHIFT: usize = 21;

// /proc/stat
const SYSTEM_GLOBAL_USER_TIME_SHIFT: usize = 0;
const SYSTEM_GLOBAL_SYSTEM_TIME_SHIFT: usize = 2;

// Record process info of each piece
#[derive(Default, Clone)]
struct RecordItem {
    timestamp: i64,
    pss: isize,
    vm_rss: isize,
    vm_anon: isize,
    vm_file: isize,
    vm_shmem: isize,
    vm_swap: isize,
    voluntary_ctxt_switches: usize,
    nonvoluntary_ctxt_switches: usize,
    minflt: usize,
    majflt: usize,
    utime: f64,
    stime: f64,
    totalcputime: f64,
    global_utime: f64,
    global_stime: f64,
    global_total_cpu_time: f64,
    cpu_occupancy_rate: f64,
    priority: i64,
    nice: i64,
    num_threads: i64,
    start_time: i64,
}

#[derive(Default)]
struct RecordProcess {
    pid: pid_t,
    record_infos: Vec<RecordItem>,
}

fn dump_csv_info(record: &RecordProcess, process_name: &str) {
    let out_path = format!(OUTPUT_FILE_TEMPLATE!(), process_name);
    let mut out = File::create(&out_path).unwrap_or_else(|_| panic!("Open file {} failed!", out_path));
    match write!(out, "time,pss,vmRss,vmAnon,vmFile,vmShmem,vmSwap,voluntaryCtxtSwitches,nonvoluntaryCtxtSwitches,minflt,\
            majflt,utime,stime,totalcputime,gutime,gstime,gtotalcputime,cpuOccupancyRate,priority,nice,numThreads,startTime \r\n") {
        Ok(_) => {},
        Err(_) => {
            panic!("dump_csv_info failed!");
        },
    }
    for item in &record.record_infos {
        match write!(out, "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{} \r\n",
                item.timestamp, item.pss, item.vm_rss, item.vm_anon, item.vm_file, item.vm_shmem,
                item.vm_swap, item.voluntary_ctxt_switches, item.nonvoluntary_ctxt_switches,
                item.minflt, item.majflt, item.utime, item.stime, item.totalcputime, item.global_utime,
                item.global_stime, item.global_total_cpu_time, item.cpu_occupancy_rate,
                item.priority, item.nice, item.num_threads, item.start_time) {
            Ok(_) => {},
            Err(_) => {
                panic!("dump_csv_info failed!");
            },
        }
    }
}

fn get_process_pid(chr: &str) -> pid_t {
    let mut pid: pid_t = -1;
    let output: Output = Command::new("sh")
            .arg("-c")
            .arg(format!("ps -ef | grep {} | grep -v grep | awk '{{print $2}}'", chr))
            .output()
            .expect("Failed to execute command");
    if output.status.success() {
        pid = from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .parse::<pid_t>()
                .unwrap_or(-1);
    }
    if pid == -1 {
        panic!("error pid: {}!", pid);
    }
    pid
}

fn get_pss_info(item: &mut RecordItem, pid: pid_t) {
    let path = format!(TASK_SMAPS_PID_TEMPLATE!(), pid);
    let content = read_path(&path)
            .unwrap_or_else(|_| panic!("Read path {} failed!", path));
    let lines = content.lines();

    for line in lines {
        if !line.starts_with(TASK_PSS_PREFIX) {
            continue;
        }
        let number_part = line
                .trim_start_matches(TASK_PSS_PREFIX)
                .trim_end_matches(" kB")
                .trim();
        if let Ok(number) = number_part.parse::<isize>() {
            item.pss += number;
        }
    }
}

fn get_global_cpu_info(item: &mut RecordItem) {
    let content = read_path(GLOBAL_SYSTEM_INFO)
            .unwrap_or_else(|_| panic!("Read path {} failed!", GLOBAL_SYSTEM_INFO));
    let lines = content.lines();
    for line in lines {
        if !line.starts_with(GLOBAL_CPU_STAT_PREFIX) {
            continue;
        }
        let temp_str = line.trim_start_matches(GLOBAL_CPU_STAT_PREFIX)
                .trim();
        let process_stat_strs: Vec<&str> = temp_str.split_whitespace().collect();
        // SAFETY:
        // Safe because we've verified that the system call returns correctly
        let clock_ticks = unsafe { sysconf(_SC_CLK_TCK) as f64 };
        item.global_utime += process_stat_strs[SYSTEM_GLOBAL_USER_TIME_SHIFT]
                .parse::<f64>()
                .unwrap_or(0.0) / clock_ticks;
        item.global_stime += process_stat_strs[SYSTEM_GLOBAL_SYSTEM_TIME_SHIFT]
                .parse::<f64>()
                .unwrap_or(0.0) / clock_ticks;
    }
    item.global_total_cpu_time = item.global_stime + item.global_utime;
}

fn monitor_thread(monitor_time: i64, monitor_iterval: i64,
        monitor_process_name: String) {
    let mut frist_flag: bool = true;
    let mut time_count: time_t = 0;
    let mut record_process = RecordProcess::default();
    let mut record_item: RecordItem = RecordItem::default();
    let mut last_record_item: RecordItem;
    let mut tmp_record_item: RecordItem;

    record_process.pid = get_process_pid(&monitor_process_name);

    while time_count < monitor_time {
        last_record_item = record_item;
        record_item = RecordItem::default();
        record_item.timestamp = time_count;
        get_global_cpu_info(&mut record_item);
        get_pss_info(&mut record_item, record_process.pid);
        for entry in fs::read_dir(format!(SUBTASK_PATH_TEMPLATE!(), record_process.pid))
                .unwrap_or_else(|_| panic!("List dir {} failed!", record_process.pid)) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => { 
                    println!("get dir entry failed");
                    continue;
                },
            };
            let pid_dir_path = entry.file_name();
            let file = fs::File::open(format!(TASK_STATUS_TID_TEMPLATE!(),
                    record_process.pid, pid_dir_path.to_string_lossy()));
            let file = match file {
                Ok(file) => file,
                Err(_) => {
                    println!("open file {} failed!", pid_dir_path.to_string_lossy());
                    continue;
                }
            };
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => { continue; }
                };
                if line.starts_with(TASK_RSS_ANON_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_RSS_ANON_PREFIX)
                            .trim_end_matches(" kB")
                            .trim();
                    record_item.vm_anon = t.parse::<isize>()
                            .expect("parse vm_anon failed!");
                } else if line.starts_with(TASK_VM_RSS_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_VM_RSS_PREFIX)
                            .trim_end_matches(" kB")
                            .trim();
                    record_item.vm_rss = t.parse::<isize>()
                        .expect("parse vm_rss failed!");
                } else if line.starts_with(TASK_RSS_FILE_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_RSS_FILE_PREFIX)
                            .trim_end_matches(" kB")
                            .trim();
                    record_item.vm_file = t.parse::<isize>()
                        .expect("parse vm_file failed!");
                } else if line.starts_with(TASK_RSS_SHMEM_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_RSS_SHMEM_PREFIX)
                            .trim_end_matches(" kB")
                            .trim();
                    record_item.vm_shmem = t.parse::<isize>()
                        .expect("parse vm_shmem failed!");
                } else if line.starts_with(TASK_VM_SWAP_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_VM_SWAP_PREFIX)
                            .trim_end_matches(" kB")
                            .trim();
                    record_item.vm_swap = t.parse::<isize>()
                        .expect("parse vm_swap failed!");
                } else if line.starts_with(TASK_NONVOLUNTARY_SWITCH_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_NONVOLUNTARY_SWITCH_PREFIX)
                            .trim();
                    record_item.nonvoluntary_ctxt_switches = t.parse::<usize>().expect("nonvoluntary_ctxt_switches failed");
                } else if line.starts_with(TASK_VOLUNTARY_SWITCH_PREFIX) {
                    let t = line
                            .trim_start_matches(TASK_VOLUNTARY_SWITCH_PREFIX)
                            .trim();
                    record_item.voluntary_ctxt_switches = t.parse::<usize>().expect("voluntary_ctxt_switches failed");
                }
            }
            let content = read_path(&format!(TASK_STAT_TID_TEMPLATE!(),
                    record_process.pid, pid_dir_path.to_string_lossy()))
                    .expect("read TASK_STAT_TID_TEMPLATE failed!");
            let process_stat_strs: Vec<&str> = content.split_whitespace().collect();
            if process_stat_strs.len() > PROCESS_STAT_STIME_SHIFT {
                let minflt = process_stat_strs[PROCESS_STAT_MINFLT_SHIFT].parse::<usize>().expect("minflt");
                let majflt = process_stat_strs[PROCESS_STAT_MAJFLT_SHIFT].parse::<usize>().expect("majflt");
                // SAFETY:
                // Safe because we've verified that the system call returns correctly
                let clock_ticks = unsafe { sysconf(_SC_CLK_TCK) as f64 };
                let utime = process_stat_strs[PROCESS_STAT_UTIME_SHIFT].parse::<f64>().expect("utime") / clock_ticks;
                let stime = process_stat_strs[PROCESS_STAT_STIME_SHIFT].parse::<f64>().expect("stime") / clock_ticks;
                record_item.minflt += minflt;
                record_item.majflt += majflt;
                record_item.utime += utime;
                record_item.stime += stime;
                record_item.totalcputime += utime + stime;
                record_item.priority = process_stat_strs[PROCESS_STAT_PRIORITY_SHIFT].parse::<i64>().expect("priority");
                record_item.nice = process_stat_strs[PROCESS_STAT_NICE_SHIFT].parse::<i64>().expect("nice");
                record_item.num_threads = process_stat_strs[PROCESS_STAT_NUM_THREADS_SHIFT].parse::<i64>().expect("num_threads");
                record_item.start_time = process_stat_strs[PROCESS_STAT_STARTTIME_SHIFT].parse::<i64>().expect("start_time");
            }
        }
        if !frist_flag {
            tmp_record_item = record_item.clone();
            println!("{},{},{},{},{},{},{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{}",
                    tmp_record_item.timestamp, tmp_record_item.pss, tmp_record_item.vm_rss, tmp_record_item.vm_anon, tmp_record_item.vm_file, tmp_record_item.vm_shmem,
                    tmp_record_item.vm_swap, tmp_record_item.voluntary_ctxt_switches, tmp_record_item.nonvoluntary_ctxt_switches,
                    tmp_record_item.minflt, tmp_record_item.majflt, tmp_record_item.utime, tmp_record_item.stime, tmp_record_item.totalcputime, tmp_record_item.global_utime,
                    tmp_record_item.global_stime, tmp_record_item.global_total_cpu_time, tmp_record_item.cpu_occupancy_rate,
                    tmp_record_item.priority, tmp_record_item.nice, tmp_record_item.num_threads, tmp_record_item.start_time);
            // Record difference
            tmp_record_item.majflt = record_item.majflt - last_record_item.majflt;
            tmp_record_item.minflt = record_item.minflt - last_record_item.minflt;
            tmp_record_item.nonvoluntary_ctxt_switches = record_item.nonvoluntary_ctxt_switches - last_record_item.nonvoluntary_ctxt_switches;
            tmp_record_item.stime = record_item.stime - last_record_item.stime;
            tmp_record_item.utime = record_item.utime - last_record_item.utime;
            tmp_record_item.global_stime = record_item.global_stime - last_record_item.global_stime;
            tmp_record_item.global_utime = record_item.global_utime - last_record_item.global_utime;
            tmp_record_item.voluntary_ctxt_switches = record_item.voluntary_ctxt_switches - last_record_item.voluntary_ctxt_switches;
            tmp_record_item.totalcputime = record_item.totalcputime - last_record_item.totalcputime;
            tmp_record_item.global_total_cpu_time = record_item.global_total_cpu_time - last_record_item.global_total_cpu_time;
            tmp_record_item.cpu_occupancy_rate = tmp_record_item.totalcputime / tmp_record_item.global_total_cpu_time;
            record_process.record_infos.push(tmp_record_item);
        }
        frist_flag = false;
        sleep(Duration::from_secs(monitor_iterval as u64));
        time_count += monitor_iterval;
    }

    dump_csv_info(&record_process, &monitor_process_name);
}

/// trace process
pub fn trace_process(monitor_time: i64, monitor_iterval: i64,
        lists: &Vec<&str>) {
    let mut works: Vec<thread::JoinHandle<_>> = Vec::new();
    // Start thread to monitor process
    for &s in lists {
        let process_name = s.to_string();
        works.push(thread::spawn(move || monitor_thread(monitor_time, monitor_iterval,
            process_name)));
    }
    // Wait sub thread finish
    for (i, t) in (0_i32..).zip(works.into_iter()) {
        match t.join() {
            Ok(_) => { println!("Thread {} finish.", i) },
            Err(_) => { println!("Thread {} error!", i) },   
        }
    }
}