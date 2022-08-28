
drop table if exists USERS;
drop table if exists ELECTORATES;
drop table if exists BADGES;
drop table if exists QUESTIONS;
drop table if exists MP_IDs;
drop table if exists Organisations;
drop table if exists PersonForQuestion;
drop table if exists Answer;

create table if not exists USERS
(
    UID         VARCHAR(30) PRIMARY KEY NOT NULL,
    DisplayName VARCHAR(60),
    AusState    VARCHAR(3),
    PublicKey   TEXT NOT NULL
) CHARACTER SET utf8;

create table if not exists ELECTORATES
(
    UID         VARCHAR(30) NOT NULL,
    Chamber     ENUM('ACT_Legislative_Assembly',
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
                     'WA_Legislative_Council') NOT NULL,
    Electorate  VARCHAR(50),
    INDEX (UID)
);

create table if not exists BADGES
(
    UID         VARCHAR(30) NOT NULL,
    badge       ENUM('EmailDomain','MP','MPStaff') NOT NULL,
    what        TEXT NOT NULL,
    INDEX (UID)
);

create table if not exists QUESTIONS
(
    QuestionId  BINARY(32) PRIMARY KEY NOT NULL, /* The hash of the question defining fields */
    Question    VARCHAR(280) NOT NULL,
    CreatedTimestamp BIGINT UNSIGNED NOT NULL,
    LastModifiedTimestamp BIGINT UNSIGNED NOT NULL,
    Version     BINARY(32) NULL, /* a bulletin board reference. Will only briefly be null. */
    CreatedBy   VARCHAR(30) NOT NULL, /* reference to UID in Users table */
    Background  TEXT NULL,
    CanOthersSetWhoShouldAsk BOOLEAN NOT NULL,
    CanOthersSetWhoShouldAnswer BOOLEAN NOT NULL,
    AnswerAccepted  BOOLEAN NOT NULL,
    FollowUpTo  BINARY(32) NULL,
    censored BOOLEAN NOT NULL DEFAULT FALSE,
    INDEX(LastModifiedTimestamp),
    INDEX(CreatedBy)
) CHARACTER SET utf8;

create table if not exists MP_IDs(
     id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
     Chamber     ENUM('ACT_Legislative_Assembly',
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
       'WA_Legislative_Council') NOT NULL,
    Electorate  TEXT NULL,
    FirstName   TEXT,
    LastName    TEXT,
    Alias   INT, /* A prior name for the same person */
    INDEX(Electorate(30)),
    INDEX(FirstName(30)),
    INDEX(LastName(30))
) CHARACTER SET utf8;


create table if not exists Committee_IDs(
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

create table if not exists Organisations(
    id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    OrgID TEXT NOT NULL,
    INDEX(OrgId(50))
) CHARACTER SET utf8;

create table if not exists PersonForQuestion
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    ROLE CHAR(1), /* Q = ask question, A = answer question */
    UID VARCHAR(30) NULL,/* reference to UID in Users table, if it is a user */
    MP INT NULL, /* reference to an MP in MP_IDs table, if it is an MP */
    ORG INT NULL, /* reference to an organisation in Organisations table, if it is an organisation */
    Committee INT NULL, /* reference to a committee in Committee_IDs table, if it is a committee */
    INDEX(QuestionId)
) CHARACTER SET utf8;

create table if not exists Answer
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    author      VARCHAR(30) NOT NULL, /* reference to UID in Users table */
    MP          INT NOT NULL, /* reference to an MP in MP_IDs table */
    timestamp   BIGINT UNSIGNED NOT NULL,
    answer      TEXT NOT NULL,
    version     BINARY(32), /* when the answer was created. Used as a key for censoring */
    censored    BOOLEAN NOT NULL DEFAULT FALSE,
    INDEX(version),
    INDEX(QuestionId),
    INDEX(MP)
) CHARACTER SET utf8;

create table HansardLink
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    url         TEXT, /* The URL */
    INDEX(QuestionId)
) CHARACTER SET utf8;

create table SchemaVersion
(
    version INT
);

insert into SchemaVersion (version) values (4);

