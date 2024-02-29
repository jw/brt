use log::warn;
use procfs::process::{all_processes, Process};
use ratatui::layout::Alignment;
use ratatui::text::Line;
use ratatui::widgets::{Cell, Row};
use uzers::{get_user_by_uid, User};

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
    Row::new([
        Cell::new(Line::from(process.pid.to_string()).alignment(Alignment::Right)),
        Cell::new(process.ppid.to_string()),
        Cell::new(process.command.clone()),
        Cell::new(Line::from(process.number_of_threads.to_string()).alignment(Alignment::Right)),
        Cell::new(username),
        Cell::new(process.virtual_memory.to_string()), // TODO: Get percentages
        Cell::new("n/a".to_string()),                  // TODO: Get CPU
    ])
}

pub fn get_processes(all_processes: Vec<Process>) -> Vec<BrtProcess> {
    let mut processes = Vec::new();
    for process in all_processes {
        let cells = match create_process(&process) {
            Some(value) => value,
            None => continue,
        };
        processes.push(cells);
    }
    processes
}

// #[derive(Copy)]
pub struct BrtProcess {
    pid: i32,
    ppid: i32,
    command: String,
    number_of_threads: i64,
    user: Option<User>,
    virtual_memory: u64,
    resident_memory: u64,
}

impl Default for BrtProcess {
    fn default() -> Self {
        BrtProcess {
            pid: -1,
            ppid: -1,
            command: "".to_string(),
            number_of_threads: -1,
            user: None,
            virtual_memory: 0,
            resident_memory: 0,
        }
    }
}

fn create_process(process: &Process) -> Option<BrtProcess> {
    let mut brt_process: BrtProcess = Default::default();
    let stat_result = process.stat();
    match stat_result {
        Ok(stat) => {
            brt_process.pid = stat.pid;
            brt_process.ppid = stat.ppid;
            brt_process.command = stat.comm;
            brt_process.number_of_threads = stat.num_threads;

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
            let (virtual_memory, resident_memory) = crate::get_memory(process);
            brt_process.virtual_memory = virtual_memory;
            brt_process.resident_memory = resident_memory;
        }
        Err(_e) => {
            warn!("Stat not found for process {}.", process.pid().to_string());
            return None;
        }
    }
    Some(brt_process)
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
