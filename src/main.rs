mod fd;

use rfd::FileDialog;
use send_wrapper::SendWrapper;
use slint::{ModelRc, StandardListViewItem, VecModel};
use std::{
    env,
    rc::Rc,
    sync::mpsc::{self, Sender},
    thread,
};

use crate::fd::FdCommand;

slint::slint!(import { Fark } from "./src/fark.slint";);

fn main() {
    let ui = Fark::new().unwrap();
    let current_dir = env::current_dir()
        .map(|x| x.display().to_string())
        .unwrap_or_else(|_| "".to_string());

    ui.set_path(current_dir.into());
    {
        let ui_week = ui.as_weak();
        ui.on_show_open_dialog(move || {
            let fark_week = ui_week.unwrap();
            let dir = FileDialog::new().pick_folder();
            if let Some(dir) = dir {
                fark_week.set_path(dir.display().to_string().into());
            }
        });
    }

    {
        let ui_week = ui.as_weak();
        {
            let ui_week = ui_week.unwrap();
            ui.on_search(move || {
                let name = ui_week.get_file_name();
                let path = ui_week.get_path();

                let mut fd = FdCommand::new();
                fd.set_path(&path);
                fd.file_name(&name);

                let rows: Rc<VecModel<slint::ModelRc<StandardListViewItem>>> = Rc::new(VecModel::default());
                let rows_clone = rows.clone();

                fd.run(move |path| {
                    let items = Rc::new(VecModel::default());
                    items.push(StandardListViewItem::from(slint::format!("{}", path)).into());
                    rows_clone.push(items.clone().into());
                    
                }).unwrap();

                ui_week.set_rows(rows.clone().into());
            });
        }
    }

    ui.run().unwrap();
}
