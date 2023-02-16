/* Have an index on UPPER(UID) for USERS. Done by computed columns as MariaDB doesn't support functional indices directly */


ALTER TABLE USERS ADD UPPER_CASE_UID VARCHAR(30) generated always as (UPPER(UID));
ALTER TABLE USERS ADD UNIQUE INDEX UPPER_CASE_UID (UPPER_CASE_UID);



delete from SchemaVersion;
insert into SchemaVersion (version) values (9);