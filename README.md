 # Right to Ask server.

## Compiling

This is a rust program. Install rust, change to the directory containing this file, and compile with 
```bash
cargo build --release
```

This will create several binary programs in the `target/release` directory.

## Documentation

Optionally, if you want to read the API and other documentation, compile it with
```
cargo doc
```
The docs are then available in `target/doc`. For example, `/target/doc/right_to_ask_api/question` describes the question API data structures.

## Databases

The server uses MySQL/MariaDB for the database. Two databases are needed, one for the
RightToAsk data, one for the bulletin board data. These may be in the same database
or may be separate.

### Installing MariaDB on Ubuntu

To install MariaDB on Ubuntu, run
```bash
sudo apt install mariadb-server
sudo mysql_secure_installation
```

Setting up a root password is problematic see
https://www.digitalocean.com/community/tutorials/how-to-install-mariadb-on-ubuntu-20-04

Then set up databases, and limited access users via

```bash
sudo mariadb
```

Then create the database and user bulletinboard. Put in (and don't forget) your own passwords.

```sql
CREATE DATABASE IF NOT EXISTS bulletinboard;
GRANT ALL PRIVILEGES ON bulletinboard.* TO 'bulletinboard'@'localhost' IDENTIFIED BY 'stick-the-bulletin-board-password-here';
CREATE DATABASE IF NOT EXISTS RightToAsk;
GRANT ALL PRIVILEGES ON RightToAsk.* TO 'RightToAsk'@'localhost' IDENTIFIED BY 'stick-the-rta-password-here';
FLUSH PRIVILEGES;
EXIT
```

### Setting up configuration

Create a file `config.toml` in the directory that you intend to run the server
as described in [config.md]


### Creating the database schema

Initialize the two databases via the command `./target/release/initialize_databases`. 

### Loading the list of MPs, Committees, and Hearings.

You need three files to serve which need regular updating:
* `data/MP_source/MPs.json` listing information about current MPs.
* `data/upcoming_hearings/committees.json` listing information about current committees.
* `data/upcoming_hearings/hearings.json` listing information about current hearings.

The first of these can be created by running `./target/release/update_mp_list_of_files`. The other two by
running `./target/release/update_upcoming_hearings`. Note that these will download new data from websites,
and may fail if their format or URLs has changed.

Alternatively, copy these files in from somewhere else.

### Setting up word comparison datafiles

In the right_to_ask_server directory, you need two files
* `GeneralVocabulary.bin` providing an indexed list of synonyms and word frequency
* `ListedKeywords.csv` providing domain specific data (e.g. nicknames for prominent politicians)

See [WordComparison.md](WordComparison.md) for how to create these files, or alternatively just copy them in.
Creating GeneralVocabulary.bin is a multi-hour process and it is easier to just copy the ~70mb file.

*Note that `GeneralVocabulary.bin` is memory mapped for speed, and modifying it while one of the
programs here is running will result in undefined behaviour, such as permanently corrupting the databases.
Don't do it*.

### Setting up word comparison database

The word comparison database is designed to be easy to reconstruct by deleting it, and recreating
from the RTA database. This should not result in any information loss. To do this run
`./target/release/recreate_word_comparison_database`.

This in principle needs to be done whenever
* `GeneralVocabulary.bin` or `ListedKeywords.csv` have changed. (although it doesn't matter with the current textfile implementation, it will with more advanced backends)
* The RTA database has been recreated by running `./target/release/initialize_databases`.
* The word comparison database schema changes. (fairly rare).

### Running the server

```bash
./target/release/right_to_ask_server
```

This will create a new webserver which has a home page providing some test and diagnostic pages. Its url will
be printed. Stop with control-C.  You can check that it is working by visiting the url (probably localhost:8099) in your web browser.

## Subsequent runs
After you have set all this up the first time, you should only need to run
```
./target/release/right_to_ask_server
```

Depending on your system, some people also find they need to restart the mysql server every time. If the above command complains that it can't access the database, you might need
```
sudo service mysql start
```
first, or the equivalent systemctl command. Or you can make this go away entirely by enabling mysql to start automatically at boot time.

Optionally, you can execute
`./target/release/initialize_databases` and `./target/release/recreate_word_comparison_database`
between runs. This will reinitialize (i.e. wipe) the database contents.

## Copyright

This program is Copyright 2022 Democracy Developers Ltd. 

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
