/* Update the schema to version to 2.

   This takes an unversioned version of the database (the version for the couple of months prior to 26 July 2022 and modifies the schema without erasing existing data.
   As this only works with reasonably recent versions of the database, the update will not be done automatically.
   To do this manually, do commands like this:
     1) open a connection to your database.
        suppose in your config.toml file you have a line like
          rta="mysql://RightToAsk:yourpasswordhere@localhost:3306/RightToAsk"
        This is typically done by a command like "mariadb -p -u RightToAsk RightToAsk"
        where the first RightToAsk is the username (first RightToAsk in config.toml line, and the second is the database name, second RightToAsk in config.toml.
        You will then be prompted for the password, then get a shell.
     2) execute this file via a command like
           source right_to_ask_api/src/RTASchemaUpdates/2.sql;
        depending on your current directory you may need to change the relative path to this file.
        You should get something that looks like this:
            Query OK, 0 rows affected (0.026 sec)

            Query OK, 1 row affected (0.001 sec)

            Query OK, 0 rows affected (0.003 sec)
            Records: 0  Duplicates: 0  Warnings: 0

        If not, run initialise_databases, although this will delete existing data.
      3) get out of the mysql/mariadb shell by the command
            QUIT;

   */

create table SchemaVersion
(
    version INT
);

insert into SchemaVersion (version) values (2);


alter table QUESTIONS add censored BOOLEAN NOT NULL DEFAULT FALSE;
