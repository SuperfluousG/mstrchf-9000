//! Closed fixture runner: entirely absent from production. No caller SQL or catalogue.
use crate::{
    audit,
    backup::{Maintenance, VerifiedBackupReceipt},
    backup_upgrade_tests::{barrier, fault},
    records::storage,
    startup,
};
use mstrchf_contracts::{DomainError, bytes_digest};
use rusqlite::Connection;

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
fn rollback_active(connection: &Connection) -> Result<(), DomainError> {
    if !connection.is_autocommit() {
        connection.execute_batch("ROLLBACK").map_err(storage)?;
    }
    if !connection.is_autocommit() {
        return Err(storage("transaction cleanup unresolved"));
    }
    Ok(())
}

pub(crate) fn apply(
    m: &mut Maintenance<'_>,
    receipt: VerifiedBackupReceipt,
) -> Result<UpgradeOutcome, DomainError> {
    let backup = m.consume(receipt)?;
    // From here every fallible exit leaves the surviving owner disabled and the
    // actual verified receipt in that owner, even after Maintenance is dropped.
    m.poison();
    m.database.retain_upgrade_backup(backup)?;
    m.database.verify_owner()?;
    let old = m.source.clone();
    let before_data = m.source_data_version();
    // Use explicit SQL so no Transaction destructor can hide a failed rollback.
    let tx = &m.database.connection;
    if let Err(error) = tx.execute_batch("BEGIN EXCLUSIVE") {
        rollback_active(tx)?;
        return Err(storage(error));
    }
    let run = (|| {
        if crate::backup::data_version(tx)? != before_data || m.catalogue.audit(tx)? != old {
            return Err(storage("source changed at write lock"));
        }
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
        let expected = audit::audit_fixture(tx, audit::FixtureVersion::V2)?;
        point("ug_before_commit")?;
        Ok(expected)
    })();
    let expected_new = match run {
        Ok(expected) => expected,
        Err(e) => {
            rollback_active(tx)?;
            m.database.verify_owner()?;
            if m.catalogue.audit(&m.database.connection)? != old {
                return Err(storage("rollback state mismatch"));
            }
            return Err(e);
        }
    };
    let simulated_before = fault("commit_before_error");
    let recovery_unreadable = fault("recovery_unreadable");
    let recovery_mismatch = fault("recovery_mismatch");
    let recovery_lost_identity = fault("recovery_lost_identity");
    let recovery_lost_lock = fault("recovery_lost_lock");
    let commit = if simulated_before {
        rollback_active(tx).and(Err(storage("injected precommit error")))
    } else {
        tx.execute_batch("COMMIT").map_err(storage)
    };
    let uncertain = commit.is_err()
        || fault("commit_after_error")
        || recovery_unreadable
        || recovery_mismatch
        || recovery_lost_identity
        || recovery_lost_lock;
    if !uncertain {
        point("ug_after_commit")?;
    }
    if recovery_lost_identity {
        m.database.live.invalidate_identity_for_test();
    }
    if recovery_lost_lock {
        m.database.invalidate_lock_for_test();
    }
    m.database.verify_owner()?;
    // A failed COMMIT may still own a live transaction. Explicitly check its
    // cleanup before attempting close; failure retains this exact connection.
    rollback_active(&m.database.connection)?;
    if recovery_mismatch {
        // A valid ordinary fixture row changes after commit. Reopened data is
        // readable but equals neither captured old nor intended new contents.
        m.database
            .connection
            .execute(
                "UPDATE app_meta SET snapshot_revision='4' WHERE singleton=1",
                [],
            )
            .map_err(storage)?;
    }
    point("recovery_before_placeholder")?;
    let previous = std::mem::replace(
        &mut m.database.connection,
        Connection::open_in_memory().map_err(storage)?,
    );
    match previous.close() {
        Ok(()) => (),
        Err((previous, error)) => {
            // Preserve the returned live connection instead of dropping it on
            // an unresolved close. Backup seals remain owned by Database.
            m.database.connection = previous;
            return Err(storage(error));
        }
    }
    if recovery_unreadable || fault("reopen_unavailable") {
        return Err(storage("recovery unavailable"));
    }
    m.database.connection = startup::writable_query_only(&m.database.path)?;
    crate::backup_upgrade_tests::arm_recovery_authorizer(&m.database.connection)?;
    point("recovery_before_audit")?;
    let actual = m.catalogue.audit(&m.database.connection)?;
    if actual != expected_new {
        // Exact old is an unsuccessful attempt. Neither old nor target is recovery-only.
        return Err(storage(if actual == old {
            "rolled back"
        } else {
            "recovery mismatch"
        }));
    }
    point("recovery_before_configure")?;
    startup::configure(&m.database.connection)?;
    point("ug_before_ready")?;
    m.database.release_resolved_upgrade();
    m.finished();
    Ok(if uncertain {
        UpgradeOutcome::CommittedAfterRecovery
    } else {
        UpgradeOutcome::Committed
    })
}
