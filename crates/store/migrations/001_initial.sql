CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY CHECK(version>0), checksum TEXT NOT NULL, applied_utc TEXT NOT NULL) STRICT;
CREATE TABLE app_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1), lineage TEXT NOT NULL UNIQUE,
 snapshot_revision TEXT NOT NULL CHECK(snapshot_revision NOT GLOB '*[^0-9]*' AND substr(snapshot_revision,1,1) BETWEEN '1' AND '9' AND length(snapshot_revision)<=20 AND (length(snapshot_revision)<20 OR snapshot_revision<='18446744073709551615')),
 settings_revision TEXT NOT NULL CHECK(settings_revision NOT GLOB '*[^0-9]*' AND substr(settings_revision,1,1) BETWEEN '1' AND '9' AND length(settings_revision)<=20 AND (length(settings_revision)<20 OR settings_revision<='18446744073709551615')),
 history_revision TEXT NOT NULL CHECK(history_revision='1'), catalogue_revision TEXT NOT NULL CHECK(catalogue_revision='1')) STRICT;
CREATE TABLE household(singleton INTEGER PRIMARY KEY REFERENCES app_meta(singleton) CHECK(singleton=1), household_id TEXT NOT NULL UNIQUE,
 draft_revision TEXT NOT NULL CHECK(draft_revision NOT GLOB '*[^0-9]*' AND substr(draft_revision,1,1) BETWEEN '1' AND '9' AND length(draft_revision)<=20 AND (length(draft_revision)<20 OR draft_revision<='18446744073709551615')),
 draft BLOB NOT NULL CHECK(length(draft)<=8388608),
 privacy_revision TEXT NOT NULL CHECK(privacy_revision NOT GLOB '*[^0-9]*' AND substr(privacy_revision,1,1) BETWEEN '1' AND '9' AND length(privacy_revision)<=20 AND (length(privacy_revision)<20 OR privacy_revision<='18446744073709551615')),
 privacy BLOB NOT NULL CHECK(length(privacy)<=8388608),
 profile_revision TEXT CHECK(profile_revision NOT GLOB '*[^0-9]*' AND substr(profile_revision,1,1) BETWEEN '1' AND '9' AND length(profile_revision)<=20 AND (length(profile_revision)<20 OR profile_revision<='18446744073709551615')),
 profile BLOB CHECK(length(profile)<=8388608), CHECK((profile IS NULL)=(profile_revision IS NULL))) STRICT;
CREATE TABLE jobs(job_id TEXT PRIMARY KEY, creation_sequence INTEGER NOT NULL UNIQUE CHECK(creation_sequence>0), terminal INTEGER NOT NULL CHECK(terminal IN (0,1)), summary BLOB NOT NULL CHECK(length(summary)<=8388608)) STRICT;
CREATE TABLE resource_quarantine(resource_key TEXT PRIMARY KEY, job_id TEXT NOT NULL REFERENCES jobs(job_id), quarantined INTEGER NOT NULL CHECK(quarantined IN (0,1))) STRICT;
CREATE TABLE command_receipts(command_id TEXT PRIMARY KEY, command_digest TEXT NOT NULL, command BLOB NOT NULL CHECK(length(command)<=8388608), response BLOB NOT NULL CHECK(length(response)<=8388608), response_digest TEXT NOT NULL,
 snapshot_revision TEXT NOT NULL UNIQUE CHECK(snapshot_revision NOT GLOB '*[^0-9]*' AND substr(snapshot_revision,1,1) BETWEEN '1' AND '9' AND length(snapshot_revision)<=20 AND (length(snapshot_revision)<20 OR snapshot_revision<='18446744073709551615'))) STRICT;
