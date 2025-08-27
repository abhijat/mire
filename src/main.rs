use anyhow::Result;
use clap::Parser;
use nix::sys::ptrace::Options;
use nix::sys::wait::{Id, WaitPidFlag};
use nix::sys::{ptrace, wait};
use nix::unistd::Pid;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

fn control_pid(pid: Pid, task_throttle_config: TaskThrottleConfig) -> Result<()> {
    let now = Instant::now();
    let deadline = now + task_throttle_config.control_duration;

    ptrace::seize(pid, Options::empty())?;

    while Instant::now() < deadline {
        ptrace::interrupt(pid)?;
        wait::waitid(Id::Pid(pid), WaitPidFlag::WSTOPPED)?;

        thread::sleep(task_throttle_config.throttle_duration);

        ptrace::cont(pid, None)?;
        thread::sleep(task_throttle_config.free_run_duration);
    }

    ptrace::interrupt(pid)?;
    wait::waitid(Id::Pid(pid), WaitPidFlag::WSTOPPED)?;

    ptrace::detach(pid, None)?;

    Ok(())
}

fn add_trace(pid: i32, config: TaskThrottleConfig, threads: &mut Vec<JoinHandle<()>>) {
    threads.push(thread::spawn(move || {
        if let Err(err) = control_pid(Pid::from_raw(pid), config) {
            println!("error: {err}");
        }
    }));
}

fn main() -> Result<()> {
    let config: Config = Args::parse().into();
    let mut threads = Vec::new();
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_processes(ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always)),
    );

    let mut done = false;
    while !done {
        for process in sys.processes_by_name(config.process_name.as_ref()) {
            // processes_by_name() matches the process_name in the string, so look for an exact
            // match
            if process.name().to_string_lossy() != config.process_name {
                println!(
                    "process name {} does not match {}",
                    process.name().to_string_lossy(),
                    config.process_name
                );
                continue;
            }

            if let Some(cmd_line_pattern) = &config.cmd_line_pattern {
                if !process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .any(|cmd_elem| cmd_elem.contains(cmd_line_pattern))
                {
                    continue;
                }
            }

            done = true;
            if let Some(tasks) = process.tasks() {
                tasks.iter().for_each(|pid| {
                    add_trace(
                        pid.as_u32() as i32,
                        TaskThrottleConfig::from(&config),
                        &mut threads,
                    )
                });
                add_trace(
                    process.pid().as_u32() as i32,
                    TaskThrottleConfig::from(&config),
                    &mut threads,
                );
            }
        }

        if !done {
            sys.refresh_processes(ProcessesToUpdate::All, true);
        }

        if !config.wait_for_process {
            done = true;
        }
    }

    for t in threads {
        t.join().unwrap();
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 900)]
    throttle_duration_ms: u64,

    #[arg(long, default_value_t = 300)]
    free_run_duration_ms: u64,

    #[arg(long, default_value_t = 120 * 1000)]
    total_control_duration_ms: u64,

    #[arg(long, default_value_t = true)]
    wait_for_process: bool,

    #[arg(long)]
    process_name: String,

    #[arg(long)]
    cmd_line_pattern: Option<String>,
}

#[derive(Debug)]
struct Config {
    throttle_duration: Duration,
    free_run_duration: Duration,
    control_duration: Duration,
    wait_for_process: bool,
    process_name: String,
    cmd_line_pattern: Option<String>,
}

#[derive(Copy, Clone)]
struct TaskThrottleConfig {
    throttle_duration: Duration,
    free_run_duration: Duration,
    control_duration: Duration,
}

impl TaskThrottleConfig {
    fn from(config: &Config) -> Self {
        TaskThrottleConfig {
            throttle_duration: config.throttle_duration,
            free_run_duration: config.free_run_duration,
            control_duration: config.control_duration,
        }
    }
}

impl Into<Config> for Args {
    fn into(self) -> Config {
        Config {
            throttle_duration: Duration::from_millis(self.throttle_duration_ms),
            free_run_duration: Duration::from_millis(self.free_run_duration_ms),
            control_duration: Duration::from_millis(self.total_control_duration_ms),
            wait_for_process: self.wait_for_process,
            process_name: self.process_name,
            cmd_line_pattern: self.cmd_line_pattern,
        }
    }
}
