use humansize::{format_size, FormatSizeOptions, BINARY};
use log::{debug, warn};
use procfs::process::{all_processes, Process};
use procfs::{ticks_per_second, CpuInfo, Current, Uptime};
use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use uzers::{get_user_by_uid, User};

#[allow(dead_code)]
pub fn get_battery() -> String {
    let manager = battery::Manager::new().unwrap();
    let battery = manager.batteries().unwrap().next().unwrap().unwrap();
    let percentage = battery.state_of_charge().value * 100.0;
    format!("{}%", percentage)
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
        Cell::new(format!("{:.2}", process.cpu)).style(special_style),
    ])
}

pub fn get_processes(all_processes: &Vec<Process>) -> Vec<BrtProcess> {
    let mut processes = Vec::new();
    for process in all_processes {
        let cells = match create_process(process) {
            Some(value) => value,
            None => continue,
        };
        processes.push(cells);
    }
    processes
}

// #[derive(Copy)]
pub struct BrtProcess {
    pub pid: i32,
    pub ppid: i32,
    pub program: String,
    pub command: String,
    pub number_of_threads: i64,
    pub user: Option<User>,
    pub resident_memory: u64,
    pub cpu: f64,
}

impl Default for BrtProcess {
    fn default() -> Self {
        BrtProcess {
            pid: -1,
            ppid: -1,
            program: "".to_string(),
            command: "".to_string(),
            number_of_threads: -1,
            user: None,
            resident_memory: 0,
            cpu: 0.0,
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

fn create_process(process: &Process) -> Option<BrtProcess> {
    let mut brt_process: BrtProcess = Default::default();
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

            // cpu
            let cpu = get_cpu(process);
            brt_process.cpu = cpu
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
    debug!("Starttime: {}s", starttime);

    let runtime = uptime - starttime;
    debug!("runtime: {}s", runtime);

    let num_cores = CpuInfo::current().unwrap().num_cores();
    debug!("Uptime: {}s", uptime);

    usage as f64 * 100.0 / runtime as f64 / num_cores as f64
}

pub fn get_all_processes() -> Vec<Process> {
    let all_processes: Vec<Process> = all_processes()
        .expect("Can't read /proc")
        .filter_map(|p| match p {
            Ok(p) => Some(p),
            Err(e) => match e {
                procfs::ProcError::NotFound(_) => None,
                procfs::ProcError::Io(_e, _path) => None,
                x => {
                    println!("Can't read process due to error {x:?}");
                    None
                }
            },
        })
        .collect();
    all_processes
}
