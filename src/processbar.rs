use anyhow::{Context, Result};
use clap::Parser;
use log::debug;
use owo_colors::OwoColorize;
use procfs::process::Process;
use procfs::{page_size, ticks_per_second, Current, Uptime};

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
    let percentage = usage as f64 * 100.0 / runtime as f64;
    println!(
        "{} ({}) has used {:.2}% of the cpu.",
        stat.comm.green(),
        pid.yellow(),
        percentage.red(),
    );
    Ok(())
}

#[derive(Debug)]
struct ProcessbarError(String);
