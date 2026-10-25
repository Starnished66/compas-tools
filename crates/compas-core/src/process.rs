//! Starting helper programs.

use std::io::{self, ErrorKind};
use std::process::Output;
use std::time::Duration;

use tokio::process::{Child, Command};

const ATTEMPTS: u32 = 20;
const PAUSE: Duration = Duration::from_millis(25);

/// A file that was only just written can be briefly busy on Linux (`ETXTBSY`) while
/// another thread's `fork` still holds it open; a short wait clears it.
async fn retrying<T>(mut start: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut attempts = 0;
    loop {
        match start() {
            Err(e) if e.kind() == ErrorKind::ExecutableFileBusy && attempts < ATTEMPTS => {
                attempts += 1;
                tokio::time::sleep(PAUSE).await;
            }
            other => return other,
        }
    }
}

pub(crate) async fn spawn(command: &mut Command) -> io::Result<Child> {
    retrying(|| command.spawn()).await
}

pub(crate) async fn output(command: &mut Command) -> io::Result<Output> {
    let mut child = spawn(command).await?;
    drop(child.stdin.take());
    child.wait_with_output().await
}
