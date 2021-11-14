
drop table if exists USERS;
drop table if exists ELECTORATES;
drop table if exists BADGES;

create table if not exists USERS
(
    UID         VARCHAR(30) PRIMARY KEY NOT NULL,
    DisplayName VARCHAR(60),
    AusState    VARCHAR(3),
    PublicKey   TEXT NOT NULL
);

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
                     'SA_Legislative_Assembly',
                     'SA_Legislative_Council',
                     'Vic_Legislative_Assembly',
                     'Vic_Legislative_Council',
                     'Tas_House_Of_Assembly',
                     'Tas_Legislative_Council',
                     'WA_Legislative_Assembly',
                     'WA_Legislative_Council') NOT NULL,
    Electorate  VARCHAR(50) NOT NULL,
    INDEX (UID)
);

create table if not exists BADGES
(
    UID         VARCHAR(30) NOT NULL,
    badge       ENUM('EmailDomain','MP','MPStaff') NOT NULL,
    what        VARCHAR(50) NOT NULL,
    INDEX (UID)
);

