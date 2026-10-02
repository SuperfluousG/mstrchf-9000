CREATE TABLE fixture_added(id INTEGER PRIMARY KEY, note TEXT NOT NULL) STRICT;
INSERT INTO fixture_added VALUES(1,'upgrade-complete');
UPDATE fixture_probe SET value=11 WHERE id=1;
