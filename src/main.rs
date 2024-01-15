mod fd;
mod tray;

use eyre::Result;
use fd::USING_STDOUT;
use rfd::FileDialog;
use rustix::process::{kill_process, Signal};
use rustix::thread::Pid;
use slint::Model;
use slint::{ModelRc, StandardListViewItem, VecModel};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{env, rc::Rc, thread};
use tray::FARK_PROCESS;
use number_prefix::NumberPrefix;

use crate::fd::FdCommand;

slint::slint!(import { Fark } from "./src/fark.slint";);

static FD_PID: AtomicI32 = AtomicI32::new(-1);

fn start_process(command_args: Vec<String>) -> Result<()> {
    // 获取当前可执行文件的路径
    let current_exe = std::env::current_exe()?;

    // 启动新进程并传递命令行参数
    let child = Command::new(current_exe).args(&command_args).spawn()?;
    let process = FARK_PROCESS.clone();
    {
        let mut process = process.lock().unwrap();
        process.push(child.id());
    }

    Ok(())
}

pub fn open_app() {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    args.push("window".to_string());

    let _ = start_process(args);
}

fn fark_main() {
    let ui = Fark::new().unwrap();
    let current_dir = env::current_dir()
        .map(|x| x.display().to_string())
        .unwrap_or_else(|_| "".to_string());

    let ui_week = ui.as_weak();
    let ui_week_clone = ui_week.clone();

    ui.set_path(current_dir.into());
    ui.on_show_open_dialog(move || {
        ui_week_clone
            .upgrade_in_event_loop(move |w| {
                let dir = FileDialog::new().pick_folder();
                if let Some(dir) = dir {
                    w.set_path(dir.display().to_string().into());
                }
            })
            .unwrap();
    });

    let rows: Rc<VecModel<slint::ModelRc<StandardListViewItem>>> = Rc::new(VecModel::default());
    let rows_rc = ModelRc::from(rows.clone());
    ui.set_rows(rows_rc);

    let paths = Arc::new(Mutex::new(Vec::new()));
    let paths_clone = paths.clone();
    let paths_clone_2 = paths.clone();
    let paths_clone_3 = paths.clone();

    let ui_week = ui.as_weak();
    ui.on_search(move || {
        let ui = ui_week.unwrap();
        ui.set_count(0);

        {
            let mut paths = paths_clone_2.lock().unwrap();
            paths.clear();
        }

        let rows = ui.get_rows();
        let rows_rc = rows.clone();
        let rows = rows_rc
            .as_any()
            .downcast_ref::<VecModel<slint::ModelRc<StandardListViewItem>>>()
            .expect("We know we set a VecModel earlier");

        rows.set_vec(vec![]);

        let name = ui.get_file_name();
        let path = ui.get_path();
        let mut fd = FdCommand::new();
        fd.set_path(&path);
        fd.file_name(&name);
        fd.glob(ui.get_glob());
        fd.case_sensitive(ui.get_case());
        fd.unrestricted(ui.get_unrestricted());

        let ui_week = ui.as_weak();
        let paths_clone = paths.clone();

        thread::spawn(move || {
            let ui_week_clone = ui_week.clone();
            let ui_week_clone_2 = ui_week.clone();

            let mut count = 0;
            fd.run(move |path| {
                let path = path.to_string();
                count += 1;
                let paths = paths_clone.clone();
                ui_week_clone
                    .upgrade_in_event_loop(move |w| {
                        if !w.get_started() {
                            return;
                        }

                        let rows = w.get_rows();
                        let rows_rc = rows.clone();
                        let rows = rows_rc
                            .as_any()
                            .downcast_ref::<VecModel<slint::ModelRc<StandardListViewItem>>>()
                            .expect("We know we set a VecModel earlier");

                        let items = Rc::new(VecModel::default());
                        let path = Path::new(&path);
                        let mut file_name = path
                            .file_name()
                            .map(|x| x.to_string_lossy())
                            .unwrap_or_default()
                            .to_string();

                        if console::measure_text_width(&file_name) > 50 {
                            file_name = console::truncate_str(&file_name, 50, "...").to_string();
                        }

                        let parent = path
                            .parent()
                            .unwrap_or_else(|| Path::new(""))
                            .display()
                            .to_string();

                        let display_parent =  if console::measure_text_width(&parent) > 61 {
                            console::truncate_str(&parent, 61, "...").to_string()
                        } else {
                            parent.clone()
                        };

                        let size = path.metadata().map(|x| x.len()).unwrap_or(0);
                        let size = human_size(size);

                        items.push(StandardListViewItem::from(slint::format!("{}", file_name)));
                        items.push(StandardListViewItem::from(slint::format!("{}", display_parent)));
                        items.push(StandardListViewItem::from(slint::format!("{}", size)));

                        rows.push(items.clone().into());

                        {
                            let mut paths = paths.lock().unwrap();
                            paths.push((path.display().to_string(), parent));
                        }

                        w.set_count(count);
                    })
                    .unwrap();
                thread::sleep(Duration::from_nanos(2400));
            })
            .unwrap();

            FD_PID.store(-1, Ordering::SeqCst);
            USING_STDOUT.store(false, Ordering::Relaxed);
            ui_week_clone_2
                .upgrade_in_event_loop(|w| w.set_started(false))
                .unwrap();
        });
    });

    let ui_week = ui.as_weak();
    {
        let ui_week = ui_week.unwrap();
        ui_week.on_open_file(move |i| {
            let paths = paths_clone.lock().unwrap();
            let entry = &paths[i as usize].0;
            let _ = open::that_detached(entry);
        });

        ui_week.on_open_directory(move |i| {
            let paths = paths_clone_3.lock().unwrap();
            let entry = &paths[i as usize].1;
            let _ = open::that_detached(entry);
        });
    }

    let ui_week = ui.as_weak();
    let ui_week_clone = ui_week.clone();
    {
        let ui_week = ui_week.unwrap();
        ui_week.on_stop_search(move || {
            let pid = FD_PID.load(Ordering::SeqCst);
            let pid = Pid::from_raw(pid).unwrap();
            thread::spawn(move || {
                let _ = kill_process(pid, Signal::Term);
            });

            FD_PID.store(-1, Ordering::SeqCst);

            loop {
                if !USING_STDOUT.load(Ordering::Relaxed) {
                    ui_week_clone
                        .upgrade_in_event_loop(|w| w.set_started(false))
                        .unwrap();
                    break;
                }
            }
        });
    }

    ui.run().unwrap();
}

#[inline]
fn human_size(size: u64) -> String {
    match NumberPrefix::binary(size as f64) {
        NumberPrefix::Standalone(bytes) => format!("{bytes} B"),
        NumberPrefix::Prefixed(prefix, n) => format!("{n:.1} {prefix}B"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);

    if args.next().is_some() {
        fark_main();
        return;
    }

    // 打开app
    open_app();

    // 打开图标
    tray::main().unwrap();
}
