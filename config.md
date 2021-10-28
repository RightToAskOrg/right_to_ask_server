# config.toml

The configuration information for the server is kept in a file called `config.toml` that should
be in the directory the server is launched from.

This contains secret information. Do not check into git!

It should contain the following sections

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




