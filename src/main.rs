mod fd;
mod tray;

use eyre::Result;
use rfd::FileDialog;
use rustix::process::{kill_process, Signal};
use rustix::thread::Pid;
use slint::Model;
use slint::{ModelRc, StandardListViewItem, VecModel};
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::time::Duration;
use std::{env, rc::Rc, thread};
use tray::FARK_PROCESS;

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

    let ui_week = ui.as_weak();
    let (tx, rx) = mpsc::channel();
    ui.on_search(move || {
        let ui = ui_week.unwrap();
        let rows = ui.get_rows();
        let rows_rc = rows.clone();
        let rows = rows_rc
            .as_any()
            .downcast_ref::<VecModel<slint::ModelRc<StandardListViewItem>>>()
            .expect("We know we set a VecModel earlier");

        if rows.row_count() > 0 {
            for _ in 0..rows.row_count() {
                rows.remove(0);
            }
        }

        let name = ui.get_file_name();
        let path = ui.get_path();
        let mut fd = FdCommand::new();
        fd.set_path(&path);
        fd.file_name(&name);

        if ui.get_glob() {
            fd.glob(true);
        } else {
            fd.glob(false);
        }

        let ui_week = ui.as_weak();

        let tx = tx.clone();
        thread::spawn(move || {
            let ui_week_clone = ui_week.clone();
            let ui_week_clone_2 = ui_week.clone();

            fd.run(tx, move |path| {
                let path = path.to_string();
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
                        items.push(StandardListViewItem::from(slint::format!("{}", path)));
                        rows.push(items.clone().into());
                    })
                    .unwrap();
                thread::sleep(Duration::from_millis(1));
            })
            .unwrap();

            FD_PID.store(-1, Ordering::SeqCst);
            ui_week_clone_2
                .upgrade_in_event_loop(|w| w.set_started(false))
                .unwrap();
        });
    });

    let ui_week = ui.as_weak();
    {
        let ui_week = ui_week.unwrap();
        let rows = ui_week.get_rows();

        ui_week.on_current_row_changed(move |i| {
            let entry = rows.row_data(i as usize).expect("1");
            let entry = entry.row_data(0).expect("2");
            let path = &entry.text;
            let _ = open::that_detached(path.to_string());
        });
    }

    let ui_week = ui.as_weak();
    let ui_week_clone = ui_week.clone();
    {
        let ui_week = ui_week.unwrap();
        ui_week.on_stop_search(move || {
            let pid = FD_PID.load(Ordering::SeqCst);
            let pid = Pid::from_raw(pid).unwrap();
            kill_process(pid, Signal::Term).unwrap();
            if rx.recv().is_ok() {
                ui_week_clone
                    .upgrade_in_event_loop(|w| w.set_started(false))
                    .unwrap();
            }
        });
    }

    ui.run().unwrap();
}

fn main() {
    let mut args = std::env::args().skip(1);

    if args.next().is_some() {
        fark_main();
        return;
    }

    //打开app
    open_app();

    //打开图标
    tray::main().unwrap();
}
