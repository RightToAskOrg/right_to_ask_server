/* Canonicalize electorates and reference USER.id, change badges to reference USER.id */


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
    CONSTRAINT FOREIGN KEY (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT,
    CONSTRAINT FOREIGN KEY (electorate_id) REFERENCES ElectorateDefinition (id) ON DELETE CASCADE ON UPDATE RESTRICT
) CHARACTER SET utf8;

INSERT IGNORE into ElectorateDefinition (Chamber,Electorate)
  select ELECTORATES.Chamber,IFNULL(ELECTORATES.Electorate,'') from ELECTORATES;

INSERT into UserElectorate (user_id,electorate_id)
  SELECT USERS.id,ElectorateDefinition.id from ELECTORATES
      inner join ElectorateDefinition on (ElectorateDefinition.Chamber=ELECTORATES.Chamber) and (ElectorateDefinition.Electorate=IFNULL(ELECTORATES.Electorate,''))
      inner join USERS on USERS.UID=ELECTORATES.UID;


DROP TABLE ELECTORATES;


ALTER TABLE BADGES add user_id INTEGER;
UPDATE BADGES inner join USERS ON USERS.UID=BADGES.UID SET BADGES.user_id=USERS.id;
ALTER TABLE BADGES modify user_id INTEGER NOT NULL;
ALTER TABLE BADGES add constraint foreign key (user_id) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE BADGES drop column UID;

ALTER TABLE QUESTIONS add CreatedById INTEGER;
UPDATE QUESTIONS inner join USERS ON USERS.UID=QUESTIONS.CreatedBy SET QUESTIONS.CreatedById=USERS.id;
ALTER TABLE QUESTIONS modify CreatedById INTEGER NOT NULL;
ALTER TABLE QUESTIONS add constraint foreign key (CreatedById) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE QUESTIONS drop column CreatedBy;

ALTER TABLE HAS_VOTED add VoterId INTEGER;
UPDATE HAS_VOTED inner join USERS ON USERS.UID=HAS_VOTED.Voter SET HAS_VOTED.VoterId=USERS.id;
ALTER TABLE HAS_VOTED modify VoterId INTEGER NOT NULL;
ALTER TABLE HAS_VOTED add constraint foreign key (VoterId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE HAS_VOTED drop column Voter;

ALTER TABLE PersonForQuestion add UserId INTEGER;
UPDATE PersonForQuestion inner join USERS ON USERS.UID=PersonForQuestion.UID SET PersonForQuestion.UserId=USERS.id;
ALTER TABLE PersonForQuestion add constraint foreign key (UserId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE PersonForQuestion drop column UID;

ALTER TABLE Answer add AuthorId INTEGER;
UPDATE Answer inner join USERS ON USERS.UID=Answer.author SET Answer.AuthorId=USERS.id;
ALTER TABLE Answer modify AuthorId INTEGER NOT NULL;
ALTER TABLE Answer add constraint foreign key (AuthorId) REFERENCES USERS (id) ON DELETE CASCADE ON UPDATE RESTRICT;
ALTER TABLE Answer drop column author;



alter table HAS_VOTED drop index QuestionId;
alter table HAS_VOTED add constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT;

alter table PersonForQuestion drop index QuestionId;
alter table PersonForQuestion add constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT;

alter table Answer drop index QuestionId;
alter table Answer add constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT;

alter table HansardLink drop index QuestionId;
alter table HansardLink add constraint foreign key (QuestionId) REFERENCES QUESTIONS (QuestionId) ON DELETE CASCADE ON UPDATE RESTRICT;


alter table Answer drop index MP;
alter table Answer add constraint foreign key (MP) REFERENCES MP_IDs (id) ON DELETE CASCADE ON UPDATE RESTRICT;


delete from SchemaVersion;
insert into SchemaVersion (version) values (7);
