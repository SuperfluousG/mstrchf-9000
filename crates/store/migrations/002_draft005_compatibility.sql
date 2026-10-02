CREATE TABLE store_format(
 singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton=1)
   REFERENCES app_meta(singleton),
 live_profile TEXT NOT NULL COLLATE BINARY CHECK(live_profile='B1Draft005')
) STRICT;
CREATE TABLE receipt_formats(
 command_id TEXT PRIMARY KEY NOT NULL COLLATE BINARY
   REFERENCES command_receipts(command_id),
 profile TEXT NOT NULL COLLATE BINARY
   CHECK(profile IN ('B1RetainedV1','B1Draft005'))
) STRICT;
