
drop table if exists USERS;
drop table if exists ELECTORATES;
drop table if exists BADGES;

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
    Electorate  VARCHAR(50),
    FirstName   VARCHAR(50),
    LastName    VARCHAR(50),
    Alias   INT, /* A prior name for the same person */
    INDEX(Electorate),
    INDEX(FirstName),
    INDEX(LastName),
) CHARACTER SET utf8;

create table if not exists Organisations(
    id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    OrgID VARCHAR(50) NOT NULL;
    INDEX(OrgId);
) CHARACTER SET utf8;

create table if not exists PersonForQuestion
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    ROLE CHAR(1), /* Q = ask question, A = answer question */
    UID VARCHAR(30) NULL,/* reference to UID in Users table, if it is a user */
    MP INT NULL, /* reference to an MP in MP_IDs table, if it is an MP */
    ORG INT NULL, /* reference to an organisation in Organisations table, if it is an organisation */
    INDEX(QuestionId),
) CHARACTER SET utf8;

create table if not exists Answer
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    author      VARCHAR(30) NOT NULL, /* reference to UID in Users table */
    timestamp   BIGINT UNSIGNED NOT NULL,
    INDEX(QuestionId),
) CHARACTER SET utf8;
