//! Closed fixture runner: entirely absent from production. No caller SQL or catalogue.
use crate::{
    audit,
    backup::{Maintenance, VerifiedBackupReceipt},
    backup_upgrade_tests::{barrier, fault},
    records::storage,
    startup,
};
use mstrchf_contracts::{DomainError, bytes_digest};
use rusqlite::{Connection, TransactionBehavior};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UpgradeOutcome {
    Committed,
    CommittedAfterRecovery,
}
fn point(name: &str) -> Result<(), DomainError> {
    barrier(name);
    if fault(name) {
        Err(storage("injected transaction failure"))
    } else {
        Ok(())
    }
}
struct UncertainCommitCustody {
    old: audit::AuditedRows,
    expected_new: audit::AuditedRows,
    _backup: VerifiedBackupReceipt,
}

pub(crate) fn apply(
    m: &mut Maintenance<'_>,
    receipt: VerifiedBackupReceipt,
) -> Result<UpgradeOutcome, DomainError> {
    let backup = m.consume(receipt)?;
    m.database.verify_owner()?;
    let old = m.source.clone();
    let before_data = m.source_data_version();
    let tx = m
        .database
        .connection
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .map_err(storage)?;
    if crate::backup::data_version(&tx)? != before_data || m.catalogue.audit(&tx)? != old {
        return Err(storage("source changed at write lock"));
    }
    let run = (|| {
        point("ug_after_begin")?;
        crate::backup_upgrade_tests::count_migration();
        tx.execute_batch(
            "CREATE TABLE fixture_added(id INTEGER PRIMARY KEY, note TEXT NOT NULL) STRICT;",
        )
        .map_err(storage)?;
        point("ug_after_ddl")?;
        tx.execute_batch("INSERT INTO fixture_added VALUES(1,'upgrade-complete');")
            .map_err(storage)?;
        point("ug_after_insert")?;
        tx.execute_batch("UPDATE fixture_probe SET value=11 WHERE id=1;")
            .map_err(storage)?;
        point("ug_after_data")?;
        tx.execute(
            "INSERT INTO schema_migrations VALUES(2,?1,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            [bytes_digest(audit::FIXTURE_V1_TO_V2_SQL.as_bytes()).0],
        )
        .map_err(storage)?;
        point("ug_after_ledger")?;
        tx.pragma_update(None, "user_version", 2).map_err(storage)?;
        point("ug_after_version")?;
        let expected = audit::audit_fixture(&tx, audit::FixtureVersion::V2)?;
        point("ug_before_commit")?;
        Ok(expected)
    })();
    let expected_new = match run {
        Ok(expected) => expected,
        Err(e) => {
            tx.rollback().map_err(storage)?;
            m.poison();
            return Err(e);
        }
    };
    let custody = UncertainCommitCustody {
        old,
        expected_new,
        _backup: backup,
    };
    let simulated_before = fault("commit_before_error");
    let commit = if simulated_before {
        tx.rollback()
            .map_err(storage)
            .and(Err(storage("injected precommit error")))
    } else {
        tx.commit().map_err(storage)
    };
    let uncertain = commit.is_err() || fault("commit_after_error");
    if !uncertain {
        point("ug_after_commit")?;
    }
    m.poison();
    m.database.verify_owner()?;
    let old = std::mem::replace(
        &mut m.database.connection,
        Connection::open_in_memory().map_err(storage)?,
    );
    old.close().map_err(|(_, e)| storage(e))?;
    if fault("reopen_unavailable") {
        return Err(storage("recovery unavailable"));
    }
    m.database.connection = startup::writable_query_only(&m.database.path)?;
    let actual = m.catalogue.audit(&m.database.connection)?;
    if actual != custody.expected_new {
        // Exact old is an unsuccessful attempt. Neither old nor target is recovery-only.
        return Err(storage(if actual == custody.old {
            "rolled back"
        } else {
            "recovery mismatch"
        }));
    }
    startup::configure(&m.database.connection)?;
    m.database.recovery_only = false;
    m.finished();
    point("ug_before_ready")?;
    Ok(if uncertain {
        UpgradeOutcome::CommittedAfterRecovery
    } else {
        UpgradeOutcome::Committed
    })
}
