//! Single-owner local storage.
use mstrchf_contracts::*;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;
mod admission;
mod audit;
mod backup;
mod commands;
mod custody;
mod database;
mod migrate;
mod receipts;
mod records;
mod recovery;
mod snapshot;
mod startup;
mod windows_fs;

#[cfg(test)]
#[path = "../tests/backup_upgrade_tests.rs"]
mod backup_upgrade_tests;
#[cfg(test)]
mod upgrade_fixture;
#[cfg(all(test, windows))]
#[path = "../tests/vfs_trace.rs"]
mod vfs_trace;

/// Trusted host configuration. The renderer must never choose this directory.
pub struct StoreConfig {
    pub app_data_root: PathBuf,
}
#[derive(Debug)]
pub struct Store {
    sender: SyncSender<Request>,
    owner: Option<JoinHandle<()>>,
}
enum Request {
    Snapshot(SyncSender<Result<Snapshot, DomainError>>),
    Command(
        Box<AppCommandV1>,
        Vec<u8>,
        SyncSender<Result<Vec<u8>, DomainError>>,
    ),
    Stop,
    #[cfg(test)]
    Pause(SyncSender<()>, mpsc::Receiver<()>),
}
impl Store {
    pub fn open(config: StoreConfig) -> Result<Self, DomainError> {
        let (sender, receiver) = mpsc::sync_channel(64);
        let (ready, opened) = mpsc::sync_channel(1);
        let owner = std::thread::Builder::new()
            .name("mstrchf-store".into())
            .spawn(move || {
                let mut db = match database::Database::open(&config.app_data_root) {
                    Ok(db) => db,
                    Err(e) => {
                        let _ = ready.send(Err(e));
                        return;
                    }
                };
                if ready.send(Ok(())).is_err() {
                    return;
                }
                while let Ok(request) = receiver.recv() {
                    match request {
                        Request::Snapshot(reply) => {
                            let _ = reply.send(db.snapshot());
                        }
                        Request::Command(command, bytes, reply) => {
                            let _ = reply.send(db.dispatch(&command, &bytes));
                        }
                        Request::Stop => break,
                        #[cfg(test)]
                        Request::Pause(ready, resume) => {
                            let _ = ready.send(());
                            let _ = resume.recv();
                        }
                    }
                }
            })
            .map_err(records::storage)?;
        match opened.recv().map_err(records::storage)? {
            Ok(()) => Ok(Self {
                sender,
                owner: Some(owner),
            }),
            Err(e) => {
                let _ = owner.join();
                Err(e)
            }
        }
    }
    pub fn snapshot(&self) -> Result<Snapshot, DomainError> {
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Snapshot(reply))
            .map_err(records::storage)?;
        receive.recv().map_err(records::storage)?
    }
    pub fn dispatch_ui(&self, command: AppCommandV1) -> Result<Snapshot, DomainError> {
        let bytes = canonical_bytes(&command)?;
        let command: AppCommandV1 = parse_document(&bytes)?;
        authorize_command(command.command.kind(), CallerClass::LocalUi)?;
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Command(Box::new(command), bytes, reply))
            .map_err(records::storage)?;
        parse_document(&receive.recv().map_err(records::storage)??)
    }
    pub fn import_result(&self, bytes: &[u8]) -> ImportOutcome {
        let error = match parse_document::<JobResultV1>(bytes) {
            Ok(_) => records::unsupported(),
            Err(e) => e,
        };
        ImportOutcome::Rejected { error }
    }
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Stop);
        if let Some(owner) = self.owner.take() {
            let _ = owner.join();
        }
    }
}

#[cfg(test)]
#[path = "../tests/mechanisms.rs"]
mod tests;
