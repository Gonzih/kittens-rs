#![allow(dead_code)]

async fn raw_runtime_bypass() {
    let (_sender, mut receiver) = tokio::sync::mpsc::channel::<u8>(1);
    tokio::select! {
        biased;
        _ = receiver.recv() => {}
        _ = core::future::pending::<()>() => {}
    }
    let handle = tokio::spawn(async {});
    drop(handle);
}

fn main() {}
