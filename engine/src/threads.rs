use crossbeam_channel::{bounded, select, Receiver};
use ctrlc::Error;

pub fn was_ctrl_c_received(ctrl_c_events: &Result<Receiver<()>, Error>) -> bool {
    select! {
        recv(ctrl_c_events.as_ref().unwrap()) -> _ => {
            return true;
        },
        default() => {
            return false;
        }
    }
}

pub fn exit_on_ctrl_c(ctrl_channel: &Result<Receiver<()>, Error>) {
    if was_ctrl_c_received(&ctrl_channel) {
        println!("Ctrl-C acknowledged, program exiting...");
        std::process::exit(0);
    }
}

pub fn setup_ctrl_channel() -> Result<Receiver<()>, Error> {
    let (sender, receiver) = bounded(100);
    ctrlc::set_handler(move || {
        println!("Received ctrl-c, exiting gracefully...");
        let _ = sender.send(());
    })?;

    Ok(receiver)
}
