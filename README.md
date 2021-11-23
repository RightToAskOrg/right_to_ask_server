# Right to Ask server.

## Compiling

This is a rust program. Install rust, change to the directory containing this file, and compile with 
```bash
cargo build --release
```

This will create several binary programs in the `target/release` directory.

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

### Loading the list of MPs

Download files and make `data/MP_source/MPs.json` by running `./target/release/update_mp_list_of_files`. This may be done by a server command later.

### Running the server

```bash
./target/release/right_to_ask_server
```
`./target/release/initialize_databases`.
This will create a new webserver which has a home page providing some test and diagnostic pages. Its url will
be printed. Stop with control-C.

## Copyright

This program is Copyright 2021 Thinking Cybersecurity Pty. Ltd. 

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
