mod fd;

use rfd::FileDialog;
use slint::Model;
use slint::{ModelRc, StandardListViewItem, VecModel};
use std::time::Duration;
use std::{env, rc::Rc, thread};

use crate::fd::FdCommand;

slint::slint!(import { Fark } from "./src/fark.slint";);

fn main() {
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
    let ui_week_clone = ui_week.clone();
    ui.on_search(move || {
        let rows = ui_week_clone.unwrap().get_rows();
        let rows_rc = ModelRc::from(rows.clone());
        let rows = rows_rc
            .as_any()
            .downcast_ref::<VecModel<slint::ModelRc<StandardListViewItem>>>()
            .expect("We know we set a VecModel earlier");

        if rows.row_count() > 0 {
            for _ in 0..rows.row_count() {
                rows.remove(0);
            }
        }

        ui_week
            .upgrade_in_event_loop(move |ui_week| {
                let name = ui_week.get_file_name();
                let path = ui_week.get_path();
                let mut fd = FdCommand::new();
                fd.set_path(&path);
                fd.file_name(&name);

                let ui_week = ui_week.as_weak();
                thread::spawn(move || {
                    let ui_week_clone = ui_week.clone();
                    fd.run(move |path| {
                        let path = path.to_string();
                        ui_week_clone
                            .upgrade_in_event_loop(move |w| {
                                let rows = w.get_rows();
                                let rows_rc = ModelRc::from(rows.clone());
                                let rows = rows_rc
                                    .as_any()
                                    .downcast_ref::<VecModel<slint::ModelRc<StandardListViewItem>>>(
                                    )
                                    .expect("We know we set a VecModel earlier");
                                let items = Rc::new(VecModel::default());
                                items.push(StandardListViewItem::from(slint::format!("{}", path)));
                                rows.push(items.clone().into());
                            })
                            .unwrap();
                        thread::sleep(Duration::from_millis(1));
                    })
                    .unwrap();
                });
            })
            .unwrap();
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

    ui.run().unwrap();
}
