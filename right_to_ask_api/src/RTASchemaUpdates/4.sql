

create table HansardLink
(
    QuestionId  BINARY(32), /* reference to QuestionID in QUESTIONS table */
    url         TEXT, /* The URL */
    INDEX(QuestionId)
) CHARACTER SET utf8;

delete from SchemaVersion;
insert into SchemaVersion (version) values (4);
