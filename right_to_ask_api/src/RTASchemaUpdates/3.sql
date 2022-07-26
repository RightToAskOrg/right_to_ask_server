

alter table Answer add censored BOOLEAN NOT NULL DEFAULT FALSE;
alter table Answer add version BINARY(32);
CREATE INDEX idx_version ON Answer (version);


delete from SchemaVersion;
insert into SchemaVersion (version) values (3);
