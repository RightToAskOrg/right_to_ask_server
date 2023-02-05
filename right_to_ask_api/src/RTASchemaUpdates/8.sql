/* Canonicalize electorates and reference USER.id, change badges to reference USER.id */

ALTER TABLE Answer ADD CensorshipStatus  ENUM('NotFlagged','Flagged','Allowed','StructureChanged','StructureChangedThenFlagged','Censored') NOT NULL DEFAULT 'NotFlagged';
ALTER TABLE QUESTIONS ADD CensorshipStatus  ENUM('NotFlagged','Flagged','Allowed','StructureChanged','StructureChangedThenFlagged','Censored') NOT NULL DEFAULT 'NotFlagged', ADD INDEX (CensorshipStatus);

ALTER TABLE Answer DROP COLUMN censored;
ALTER TABLE QUESTIONS DROP COLUMN censored;

ALTER TABLE QUESTIONS ADD NumFlags INTEGER NOT NULL DEFAULT 0,ADD INDEX (NumFlags); /* number of flags of the question (or an answer in it) [since the last moderator approval] */

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


delete from SchemaVersion;
insert into SchemaVersion (version) values (8);
