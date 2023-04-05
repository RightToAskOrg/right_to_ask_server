# config.toml

The configuration information for the server is kept in a file called `config.toml` that should
be in the directory the server is launched from.

This contains secret information. Do not check into git!

It should contain the following sections

## General configuration

Some general configuration for the server goes at the top. These are likely to change.

```toml
# Define the number of users' long searches that are cached. Should be O(number of users per hour)
search_cache_size=1000
# default false. If set to true, the user must have validated an email address to do most write operations.
require_validated_email=false
```


## Server signing key.

This should be an ECDSA key.

You can generate such a key in a variety of ways. One is using openssl:

```bash
openssl genpkey -algorithm Ed25519 -out priv.pem
cat priv.pem 
openssl pkey -in priv.pem -pubout -out pub.pem
cat pub.pem
```

This should produce a transcript like
```text
$ cat priv.pem 
-----BEGIN PRIVATE KEY-----
MC4CAQAwBQYDK2VwBCIEIC1g2fpTgYB8+pq7yKC+ZTxnRux0fgVqx2lJ5DqmTdom
-----END PRIVATE KEY-----
$ openssl pkey -in priv.pem -pubout -out pub.pem
$ cat pub.pem
-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEArp1jCQctgoBLmuUUphij24bFB8qwsm/45+9GVoRc4FI=
-----END PUBLIC KEY----- 
```
The base 64 encoded text is what is wanted. Insert it into `config.toml` like
```toml
[signing]
public = "MCowBQYDK2VwAyEArp1jCQctgoBLmuUUphij24bFB8qwsm/45+9GVoRc4FI="
private = "MC4CAQAwBQYDK2VwBCIEIC1g2fpTgYB8+pq7yKC+ZTxnRux0fgVqx2lJ5DqmTdom"
```

Naturally, do not use the example keys provided above!

Format note: After decoding from base-64, the private key is in PKCS#8 format, and the public key is in SubjectPublicKeyInfo (SPKI).

## Database URLs

Add a section to `config.toml` with database URLs based on where the database 
was set up and passwords like

```toml
[database]
rta = "mysql://RightToAsk:stick-the-rta-password-here@localhost:3306/RightToAsk"
bulletinboard = "mysql://bulletinboard:stick-the-bulletin-board-password-here@localhost:3306/bulletinboard"
```

## Email configuration (optional for development)

The server needs to send email to verify control of an email address. If the following is not set,
then the code will be written to the server console. If the following is included, it will be emailed
using an external SMTP server (connecting on the standard submission port 587 using STARTTLS and PLAIN authentication)

```toml
[email]
# The email address that emails come from
verification_from_email = "RightToAsk <verification@righttoask.org>"
# The email address that "reply" will send to.
verification_reply_to_email = "RightToAsk <verification@righttoask.org>"
relay = "mail.righttoask.org" # The mail server that will do the sending. This is not a real one
# optional, used for testing. If entered, then mail will be sent to this address instead of where it is supposed to go.
# Do not use in a production system.
testing_email_override = "tester@righttoask.org"

[email.smtp_credentials]
authentication_identity = "verification@righttoask.org"
secret = "stick-your-password-here"
```

There is an intention of (optionally) allowing other credentials such as for AWS email.


# Unit tests for database - test_config.toml

Some unit tests, when running, require a database to execute against. These use a file with the
same format called test_config.toml

The unit tests are normally run in the directory of the crate - the unit tests requiring test_config.toml
will first change directory to the parent directory if that contains a test_config.toml file but the 
main directory does not.

You will usually want to use a different database for the unit tests - most if not all such unit tests 
will erase and re-initialize the database.


