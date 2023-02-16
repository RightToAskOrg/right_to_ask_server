
drop table if exists USERS;
drop table if exists ELECTORATES;
drop table if exists BADGES;
drop table if exists QUESTIONS;
drop table if exists HAS_VOTED;
drop table if exists MP_IDs;
drop table if exists Organisations;
drop table if exists Committee_IDs;
drop table if exists Minister_IDs;
drop table if exists PersonForQuestion;
drop table if exists Answer;

create table if not exists USERS
(
    id          INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL, /* The real permanent unique id for a person. UID rarely changes but sometimes does. */
    UID         VARCHAR(30) NOT NULL,
    DisplayName VARCHAR(60),
    AusState    VARCHAR(3),
    PublicKey   TEXT NOT NULL,
    UPPER_CASE_UID VARCHAR(30) generated always as (UPPER(UID)),
    UNIQUE INDEX(UID),
    UNIQUE INDEX UPPER_CASE_UID (UPPER_CASE_UID)
) CHARACTER SET utf8;

/* The definition of an electorate, referred to in UserElectorate by the id */
create table if not exists ElectorateDefinition
(
    id  INTEGER PRIMARY KEY AUTO_INCREMENT NOT NULL,
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
    Electorate  VARCHAR(50) NOT NULL, /* If a chamber doesn't have electorates this is blank, as unique nulls are not enforced in MariaDB */
    UNIQUE INDEX indexCE (Chamber,Electorate)
) CHARACTER SET utf8;

/* The electorates that a particular user is in */
create table if not exists UserElectorate
(
    user_id INTEGER NOT NULL,
    electorate_id INTEGER NOT NULL,
    INDEX(user_id),
    CONSTRAINT FOREIGN KEY (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT,
    CONSTRAINT FOREIGN KEY (electorate_id) REFERENCES ElectorateDefinition (id) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

create table if not exists BADGES
(
    badge       ENUM('EmailDomain','MP','MPStaff') NOT NULL,
    what        TEXT NOT NULL,
    user_id     INTEGER NOT NULL,
    foreign key (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT
);

create table if not exists QUESTIONS
(
    QuestionId  BINARY(32) PRIMARY KEY NOT NULL, /* The hash of the question defining fields */
    Question    VARCHAR(280) NOT NULL,
    CreatedTimestamp BIGINT UNSIGNED NOT NULL,
    LastModifiedTimestamp BIGINT UNSIGNED NOT NULL,
    Version     BINARY(32) NULL, /* a bulletin board reference. Will only briefly be null. */
    CreatedById INTEGER NOT NULL,
    Background  TEXT NULL,
    CanOthersSetWhoShouldAsk BOOLEAN NOT NULL,
    CanOthersSetWhoShouldAnswer BOOLEAN NOT NULL,
    AnswerAccepted  BOOLEAN NOT NULL,
    FollowUpTo  BINARY(32) NULL,
    TotalVotes  INT NOT NULL DEFAULT 0,
    NetVotes    INT NOT NULL DEFAULT 0,
    CensorshipStatus  ENUM('NotFlagged','Flagged','Allowed','StructureChanged','StructureChangedThenFlagged','Censored') NOT NULL DEFAULT 'NotFlagged',
    NumFlags INTEGER NOT NULL DEFAULT 0, /* number of flags of the question (or an answer in it) [since the last moderator approval] */
    INDEX(LastModifiedTimestamp),
    INDEX(NumFlags),
    foreign key (CreatedById) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

create table if not exists HAS_VOTED
(
    QuestionId  BINARY(32) NOT NULL, /* The hash of the question defining fields */
    VoterId INTEGER NOT NULL, /* reference to id in Users table */
    constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT,
    constraint foreign key (VoterId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT
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

create table if not exists Organisations(
    id INT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    OrgID TEXT NOT NULL,
    INDEX(OrgId(50))
) CHARACTER SET utf8;

create table if not exists PersonForQuestion
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    ROLE CHAR(1), /* Q = ask question, A = answer question */
    UserId INTEGER NULL,/* reference to id in Users table, if it is a user */
    MP INT NULL, /* reference to an MP in MP_IDs table, if it is an MP */
    ORG INT NULL, /* reference to an organisation in Organisations table, if it is an organisation */
    Committee INT NULL, /* reference to a committee in Committee_IDs table, if it is a committee */
    Minister INT NULL, /* reference to a minister in Minister_IDs table, if it is a minister */
    foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT,
    constraint foreign key (UserId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

create table if not exists Answer
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    AuthorId    INTEGER NOT NULL, /* reference to id in Users table */
    MP          INT NOT NULL, /* reference to an MP in MP_IDs table */
    timestamp   BIGINT UNSIGNED NOT NULL,
    answer      TEXT NOT NULL,
    version     BINARY(32), /* when the answer was created. Used as a key for censoring */
    CensorshipStatus  ENUM('NotFlagged','Flagged','Allowed','StructureChanged','StructureChangedThenFlagged','Censored') NOT NULL DEFAULT 'NotFlagged',
    INDEX(version),
    foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT,
    INDEX(MP),
    constraint foreign key (AuthorId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

create table HansardLink
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    url         TEXT, /* The URL */
    foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

/**
  This is a list of emails that the server should never send email to.
  Typically this is because they have been targetted by malicious thirs parties, and the
  email address owner has asked for their email to be restricted.
 */
create table DoNotEmail(
    email   TEXT,
    INDEX(email(20))
) CHARACTER SET utf8;

/* To prevent RTA from being used to send too many emails maliciously to a third party, there
   is a rate limit associated with a particular address. This may be over different periods.
   Periodically, all rates with a given timescale are deleted.
 */
create table EmailRateLimitHistory(
                                      email   TEXT,
                                      timescale  INT NOT NULL, /* 0 means today, 1 means this month */
                                      sent    INT NOT NULL, /* The number of times this email has been sent on this timescale */
                                      INDEX(email(20)),
                                      INDEX(timescale)
) CHARACTER SET utf8;

CREATE TABLE QuestionReportedReasons (
                                         QuestionId BINARY (32) NOT NULL, /* The hash of the question defining fields */
                                         reason ENUM('NotAQuestion','ThreateningViolence','IncludesPrivateInformation','IncitesHatredOrDiscrimination','EncouragesHarm','TargetedHarassment','DefamatoryInsinuation','Illegal','Impersonation','Spam') NOT NULL,
                                         user_id INTEGER NOT NULL,
                                         CONSTRAINT FOREIGN KEY (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT,
                                         constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT,
                                         constraint qru unique (QuestionId,reason,user_id)
)  CHARACTER SET utf8;

CREATE TABLE AnswerReportedReasons (
                                       QuestionId BINARY (32) NOT NULL, /* The hash of the question defining fields */
                                       reason ENUM('NotAQuestion','ThreateningViolence','IncludesPrivateInformation','IncitesHatredOrDiscrimination','EncouragesHarm','TargetedHarassment','DefamatoryInsinuation','Illegal','Impersonation','Spam') NOT NULL,
                                       answer BINARY (32) NOT NULL,
                                       user_id INTEGER NOT NULL,
                                       CONSTRAINT FOREIGN KEY (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT,
                                       constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT,
                                       constraint foreign key (answer) REFERENCES Answer (version) ON DELETE CASCADE ON UPDATE RESTRICT,
                                       constraint qrau unique (QuestionId,reason,answer,user_id)
)  CHARACTER SET utf8;



create table SchemaVersion
(
    version INT
);

insert into SchemaVersion (version) values (9);

