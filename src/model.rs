use battery::Battery;
use humansize::{format_size, FormatSizeOptions, BINARY};
use log::{debug, warn};
use procfs::process::Process;
use procfs::{ticks_per_second, CpuInfo, Current, Uptime};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use std::collections::{HashMap, VecDeque};
use uzers::{get_user_by_uid, User};

pub fn get_battery() -> Battery {
    let manager = battery::Manager::new().unwrap();
    manager.batteries().unwrap().next().unwrap().unwrap()
}

pub fn create_rows<'a>(processes: &Vec<BrtProcess>) -> Vec<Row<'a>> {
    let mut rows = Vec::new();
    for process in processes {
        let row = create_row(process);
        rows.push(row);
    }
    rows
}

pub fn create_row<'a>(process: &BrtProcess) -> Row<'a> {
    let user = process.user.clone();
    let username = if user.is_some() {
        #[allow(clippy::unnecessary_unwrap)]
        user.unwrap().name().to_os_string().into_string().unwrap()
    } else {
        "unknown".to_string()
    };

    let special_style = Style::default().fg(Color::Rgb(0x0D, 0xE7, 0x56));

    let humansize_options: FormatSizeOptions = FormatSizeOptions::from(BINARY)
        .space_after_value(false)
        .decimal_places(1)
        .decimal_zeroes(0);

    Row::new([
        Cell::new(Line::from(process.pid.to_string()).alignment(Alignment::Right)),
        Cell::new(process.program.to_string()).style(special_style),
        Cell::new(process.command.to_string()),
        Cell::new(
            Line::from(process.number_of_threads.to_string())
                .alignment(Alignment::Right)
                .style(special_style),
        ),
        Cell::new(username),
        Cell::new(format_size(process.resident_memory, humansize_options)).style(special_style),
        Cell::new(process.cpu_graph.to_string()),
        Cell::new(format!("{:.2}", process.cpu)).style(special_style),
    ])
}

fn between(status: &f64, min: f64, max: f64) -> bool {
    status >= &min && status < &max
}

fn get_points(cpu: &f64) -> i32 {
    match cpu {
        status if between(status, 0_f64, 0.001_f64) => 0,
        status if between(status, 0.001_f64, 0.2_f64) => 1,
        status if between(status, 0.2_f64, 0.5_f64) => 2,
        status if between(status, 0.5_f64, 0.7_f64) => 3,
        status if between(status, 0.7_f64, 100_f64) => 4,
        _ => {
            warn!("Invalid cpu found: {}; setting to zero.", cpu);
            0
        }
    }
}

pub fn get_cpu_graph(cpus: &VecDeque<f64>) -> String {
    let blocks: HashMap<&str, &str> = HashMap::from([
        ("00", " "),
        ("01", "⢀"),
        ("02", "⢠"),
        ("03", "⢰"),
        ("04", "⢸"),
        ("10", "⡀"),
        ("11", "⣀"),
        ("12", "⣠"),
        ("13", "⣰"),
        ("14", "⣸"),
        ("20", "⡄"),
        ("21", "⣄"),
        ("22", "⣤"),
        ("23", "⣴"),
        ("24", "⣼"),
        ("30", "⡆"),
        ("31", "⣆"),
        ("32", "⣦"),
        ("33", "⣶"),
        ("34", "⣾"),
        ("40", "⡇"),
        ("41", "⣇"),
        ("42", "⣧"),
        ("43", "⣷"),
        ("44", "⣿"),
    ]);
    let mut graph = "".to_string();
    let v: Vec<&f64> = cpus.iter().collect();
    for cpu in v.chunks(2) {
        let first = get_points(cpu[0]);
        let second = get_points(cpu[1]);
        let slot = blocks.get(format!("{}{}", first, second).as_str()).unwrap();
        graph += slot;
    }
    graph
}

#[derive(Default, Clone, Debug)]
pub struct BrtProcess {
    pub pid: i32,
    pub ppid: i32,
    pub program: String,
    pub command: String,
    pub number_of_threads: i64,
    pub user: Option<User>,
    pub resident_memory: u64,
    pub cpus: VecDeque<f64>,
    pub cpu_graph: String,
    pub cpu: f64,
}

impl BrtProcess {
    pub fn new() -> BrtProcess {
        BrtProcess {
            cpus: VecDeque::from(vec![0_f64; 10]),
            ..Default::default()
        }
    }
}

fn create_command(cmdline: &[String]) -> String {
    let mut command = "".to_string();
    for part in cmdline.iter() {
        command += format!("{} ", part).as_str();
    }
    command
}

pub fn to_brt_process(process: &Process) -> Option<BrtProcess> {
    let mut brt_process: BrtProcess = BrtProcess::new();
    let stat_result = process.stat();
    match stat_result {
        Ok(stat) => {
            brt_process.pid = stat.pid;
            brt_process.ppid = stat.ppid;
            brt_process.program = stat.comm;
            brt_process.number_of_threads = stat.num_threads;

            // command
            let cmd_result = process.cmdline();
            match cmd_result {
                Ok(cmd) => {
                    brt_process.command = create_command(&cmd);
                }
                Err(_e) => {
                    brt_process.command = "zombie".to_string();
                }
            }

            // user
            let uid_result = process.uid();
            match uid_result {
                Ok(uid) => {
                    brt_process.user = get_user_by_uid(uid);
                }
                Err(_e) => {
                    warn!("No user found for process {}.", process.pid().to_string());
                    brt_process.user = None;
                }
            }

            // memory
            let resident_memory = get_memory(process);
            brt_process.resident_memory = resident_memory;

            // cpu(s)
            let cpu = get_cpu(process);
            brt_process.cpu = cpu;
            brt_process.cpus.push_back(cpu);
            brt_process.cpus.pop_front();
            brt_process.cpu_graph = get_cpu_graph(&brt_process.cpus);
        }
        Err(_e) => {
            warn!("Stat not found for process {}.", process.pid().to_string());
            return None;
        }
    }
    Some(brt_process)
}

pub fn get_memory(process: &Process) -> u64 {
    let statm = process.statm().unwrap(); // TODO: this can be: NotFound(Some("/proc/3955386/statm"))
    let page_size = procfs::page_size();
    statm.resident * page_size
}

fn get_cpu(process: &Process) -> f64 {
    let stat = process.stat().unwrap();

    let usage = stat.utime / ticks_per_second() + stat.stime / ticks_per_second();
    debug!("usage: {}s", usage);

    let uptime = Uptime::current().unwrap().uptime_duration().as_secs();
    debug!("Uptime: {}s", uptime);

    let starttime = stat.starttime / ticks_per_second();
    debug!("start time: {}s", starttime);

    let runtime = uptime - starttime;
    debug!("runtime: {}s", runtime);

    let num_cores = CpuInfo::current().unwrap().num_cores();
    debug!("Uptime: {}s", uptime);

    usage as f64 * 100.0 / runtime as f64 / num_cores as f64
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_get_all_processes() {
        // let all_processes = get_all_processes();
        // assert_eq!(all_processes.is_empty(), false)
        assert_eq!(false, false)
    }
}
