







ALTER TABLE USERS DROP PRIMARY KEY, ADD id INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL FIRST, ADD UNIQUE INDEX(UID);

create table DoNotEmail(
                           email   TEXT,
                           INDEX(email(20))
) CHARACTER SET utf8;


create table EmailRateLimitHistory(
        email   TEXT,
        timescale  INT NOT NULL, /* 0 means today, 1 means this month */
        sent    INT NOT NULL, /* The number of times this email has been sent on this timescale */
        INDEX(email(20)),
        INDEX(timescale)
) CHARACTER SET utf8;






delete from SchemaVersion;
insert into SchemaVersion (version) values (6);
