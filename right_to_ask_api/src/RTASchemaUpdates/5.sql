

alter table QUESTIONS add TotalVotes INT NOT NULL DEFAULT 0;
alter table QUESTIONS add NetVotes INT NOT NULL DEFAULT 0;

create table if not exists HAS_VOTED
(
    QuestionId  BINARY(32) NOT NULL, /* The hash of the question defining fields */
    Voter       VARCHAR(30) NOT NULL, /* reference to UID in Users table */
    INDEX(QuestionId),
    INDEX(Voter)
) CHARACTER SET utf8;


create table if not exists Minister_IDs(
                                           id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
                                           Jurisdiction     ENUM('ACT_Legislative_Assembly',
                                               'Australian_House_Of_Representatives',
                                               'Australian_Senate',
                                               'NSW_Legislative_Assembly',
                                               'NSW_Legislative_Council',
                                               'NT_Legislative_Assembly',
                                               'Qld_Legislative_Assembly',
                                               'SA_House_Of_Assembly',
                                               'SA_Legislative_Council',
                                               'Vic_Legislative_Assembly',
                                               'Vic_Legislative_Council',
                                               'Tas_House_Of_Assembly',
                                               'Tas_Legislative_Council',
                                               'WA_Legislative_Assembly',
                                               'WA_Legislative_Council',
                                               'ACT','NSW','NT','QLD','SA','TAS','VIC','WA',
                                               'Federal') NOT NULL,
                                           Name   TEXT NOT NULL,
                                           INDEX(Jurisdiction),
                                           INDEX(Name(30))
) CHARACTER SET utf8;

alter table PersonForQuestion add Minister INT NULL default NULL;

delete from SchemaVersion;
insert into SchemaVersion (version) values (5);
