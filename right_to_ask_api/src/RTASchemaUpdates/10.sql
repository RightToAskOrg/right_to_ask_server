/* Have an index on UPPER(UID) for USERS. Done by computed columns as MariaDB doesn't support functional indices directly */


ALTER TABLE USERS ADD VerifiedEmail TEXT NULL;
ALTER TABLE USERS ADD VerifiedEmailTimestamp BIGINT UNSIGNED NULL;
ALTER TABLE USERS ADD Blocked BOOLEAN NOT NULL DEFAULT FALSE;



delete from SchemaVersion;
insert into SchemaVersion (version) values (10);