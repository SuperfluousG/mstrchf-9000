//! Single-owner local storage.
use mstrchf_contracts::*;
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;
mod admission;
mod audit;
mod backup;
mod commands;
mod compat;
mod custody;
pub use compat::b1_v1::B1RetainedV1 as HistoricalB1Snapshot;
pub use compat::draft005::B1Draft005 as Draft005ReceiptSnapshot;
pub use compat::{CommandDisposition, ReceiptSnapshot, UiCommandOutcome};
/// Enum names for read-only typed navigation of retained B1 responses.
/// Checked snapshots and their decoders remain privately constructible.
pub mod historical_b1 {
    pub use crate::compat::b1_v1::{
        Action, AgeBand, AllergenCode, DietCode, EquipmentCode, ErrorCode, HardRequirement,
        HouseholdStage, JobState, LoyaltyStatus, MealLocation, MealType, MobilityTemperature,
        Never, PortionPresetName, Provider, ReminderChannel, ReminderPrompt, RequiredAction,
        RestrictionDeclaration, Retailer, SpendingStyle, Weekday,
    };
}
/// Enum names for read-only typed navigation of fixed draft005 responses.
/// These types cannot mutate or construct a checked receipt snapshot.
pub mod draft005_receipt {
    pub use crate::compat::draft005::{
        Action, AgeBand, AllergenCode, DietCode, EquipmentCode, ErrorCode, HardRequirement,
        HouseholdStage, JobState, LoyaltyStatus, MealLocation, MealType, MobilityTemperature,
        Never, PortionPresetName, Provider, ReminderChannel, ReminderPrompt, RequiredAction,
        RestrictionDeclaration, Retailer, SpendingStyle, Weekday,
    };
}

#[cfg(all(debug_assertions, feature = "synthetic-development"))]
mod current;
mod database;
mod migrate;
mod receipts;
mod records;
mod recovery;
mod snapshot;
mod startup;
mod upgrade;
mod windows_fs;

#[cfg(all(debug_assertions, feature = "synthetic-development"))]
pub mod development;
#[cfg(all(test, debug_assertions, feature = "synthetic-development"))]
#[path = "../tests/development.rs"]
mod development_tests;

#[cfg(test)]
#[path = "../tests/backup_upgrade_tests.rs"]
mod backup_upgrade_tests;
#[cfg(all(test, windows))]
#[path = "../tests/barrier_evidence.rs"]
mod barrier_evidence;
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
        SyncSender<Result<UiCommandOutcome, DomainError>>,
    ),
    Replay(
        String,
        Vec<u8>,
        SyncSender<Result<UiCommandOutcome, DomainError>>,
    ),
    Stop,
    #[cfg(test)]
    Pause(SyncSender<()>, mpsc::Receiver<()>),
}
impl Store {
    pub fn open(config: StoreConfig) -> Result<Self, DomainError> {
        Self::open_mode(config, false)
    }
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    pub fn open_development(config: StoreConfig) -> Result<Self, DomainError> {
        Self::open_mode(config, true)
    }
    fn open_mode(config: StoreConfig, development: bool) -> Result<Self, DomainError> {
        let (sender, receiver) = mpsc::sync_channel(64);
        let (ready, opened) = mpsc::sync_channel(1);
        let owner = std::thread::Builder::new()
            .name("mstrchf-store".into())
            .spawn(move || {
                let opened = {
                    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
                    if development {
                        database::Database::open_development(&config.app_data_root)
                    } else {
                        database::Database::open(&config.app_data_root)
                    }
                    #[cfg(not(all(debug_assertions, feature = "synthetic-development")))]
                    {
                        let _ = development;
                        database::Database::open(&config.app_data_root)
                    }
                };
                let mut db = match opened {
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
                            let _ = reply.send(db.dispatch_outcome(&command, &bytes));
                        }
                        Request::Replay(id, bytes, reply) => {
                            let _ = reply.send(db.replay(&id, &bytes));
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
    pub fn dispatch_ui(&self, command: AppCommandV1) -> Result<UiCommandOutcome, DomainError> {
        let bytes = canonical_bytes(&command)?;
        let command: AppCommandV1 = parse_document(&bytes)?;
        authorize_command(command.command.kind(), CallerClass::LocalUi)?;
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Command(Box::new(command), bytes, reply))
            .map_err(records::storage)?;
        receive.recv().map_err(records::storage)?
    }
    pub fn replay_ui(
        &self,
        original_command_bytes: &[u8],
    ) -> Result<UiCommandOutcome, DomainError> {
        let id = compat::envelope(original_command_bytes)?;
        let (reply, receive) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Replay(id, original_command_bytes.to_vec(), reply))
            .map_err(records::storage)?;
        receive.recv().map_err(records::storage)?
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

#[cfg(test)]
#[path = "../tests/compatibility.rs"]
mod compatibility_tests;

#[cfg(all(test, windows))]
#[path = "../tests/compatibility_cleanup.rs"]
mod compatibility_cleanup_tests;
