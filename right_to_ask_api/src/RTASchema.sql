
drop table if exists USERS;

create table if not exists USERS
(
    UID         VARCHAR(30) PRIMARY KEY NOT NULL,
    DisplayName VARCHAR(60),
    AusState    VARCHAR(3),
    PublicKey   TEXT NOT NULL
);

