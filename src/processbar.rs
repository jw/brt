pub mod model;

use anyhow::{Context, Result};
use clap::Parser;
use log::debug;
use model::get_memory;
use owo_colors::OwoColorize;
use procfs::process::Process;
use procfs::{page_size, ticks_per_second, CpuInfo, Current, Uptime};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    pid: i32,
}

#[allow(dead_code)]
fn main() -> Result<()> {
    let args = Args::parse();

    let pid = args.pid;

    debug!("Checking pid {}...", pid);
    let process = Process::new(pid).with_context(|| format!("Pid {pid} not found."))?;
    let stat = process.stat().unwrap();

    debug!("ticks per second: {}", ticks_per_second());
    debug!("pagesize: {}", page_size());
    let usage = stat.utime / ticks_per_second() + stat.stime / ticks_per_second();
    debug!("usage {} ", usage);

    let uptime = Uptime::current().unwrap().uptime_duration().as_secs();
    debug!("Uptime: {}", uptime);
    let starttime = stat.starttime / ticks_per_second();
    debug!("Starttime: {}", starttime);
    let runtime = uptime - starttime;
    debug!("runtime: {}", runtime);
    let num_cores = CpuInfo::current().unwrap().num_cores();
    debug!("num cores: {}", num_cores);
    let percentage = usage as f64 * 100.0 / runtime as f64 / num_cores as f64;

    let memory = get_memory(&process);

    println!(
        "Process {} ({}) has used {:.2}% of the cpu and is using {} bytes of memory.",
        stat.comm.green(),
        pid.yellow(),
        percentage.yellow(),
        memory.yellow(),
    );

    Ok(())
}

#[derive(Debug)]
struct ProcessbarError(String);
