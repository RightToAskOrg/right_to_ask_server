# Right to Ask server.

## Compiling

To compile this you currently need the git repository bulletin-board at the same level as this
directory.

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

### Telling the server how to connect.

Create a file in the directory that you intend to run the server from called database_url 
containing something like 
```text
mysql://RightToAsk:stick-the-rta-password-here@localhost:3306/RightToAsk
```
and similarly a file called bulletin_board_url containing something like 
```text
mysql://bulletinboard:stick-the-bulletin-board-password-here@localhost:3306/bulletinboard
```

### Creating the database schema

Initialize the two databases via the command `./target/release/initialize_databases`. 


## Copyright

This program is Copyright 2021 Thinking Cybersecurity Pty. Ltd. 

Licenses subject to change soon.

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
