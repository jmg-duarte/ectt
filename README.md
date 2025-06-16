# eCTT

> Short for eletronic-[CTT](https://en.wikipedia.org/wiki/CTT_Correios_de_Portugal).

Read and send emails from your CLI.

## Build

```
cargo b -r
```

## Run

```
cargo r -r -- run
```

You can download the binary from the [GitHub Releases](https://github.com/jmg-duarte/ectt/releases) and run it with:

```
ectt run
```

## Configuration

eCTT's configuration is based on `himalaya`'s configuration, however, we're using JSON instead of TOML and we support less options.

The configuration can be placed under `<OS configuration folder>/ectt/config.json`,
the directory from which eCTT is launched, or specified using the `--config <file>` flag.

> NOTE: eCTT will output some logs to the `<OS configuration folder>/ectt/config.json`,
> these should only contain ERROR logs, however you should be able to change this using

### IMAP

The IMAP configuration supports both password and OAuth based authentication.
The main part of the configuration is declared with a `read` key:

```json
{
    "read": {
        "type": "imap",
        "host": "imap.gmail.com",
        "port": 993,
        "login": "<you@email.com>",
        "auth": { /* see below */ }
    },
}
```

> The only supported `read.type` is currently `imap`

<details>
<summary><h4>Authentication</h4></summary>

##### OAuth

OAuth setup will slightly vary from provider to provider, below you can find a list of OAuth setup guides from tested providers:

* [Gmail OAuth instructions](https://developers.google.com/identity/protocols/oauth2#1.-obtain-oauth-2.0-credentials-from-the-dynamic_data.setvar.console_name-.)

The `auth` field for OAuth will resemble the following:

```json
"auth": {
    "type": "oauth",
    "client_id": "<CLIENT_ID>",
    "auth_uri": "<AUTH_URL>",
    "token_uri": "<TOKEN_URL>",
    "client_secret": "<CLIENT_SECRET>",
    "access_token": "<ACCESS_TOKEN>",
    "refresh_token": "<REFRESH_TOKEN>"
}
```


> eCTT will automatically refresh your access token if it has expired, *however*
> it will *not* update the configuration file.

<details>
<summary>Putting it all together</summary>

```
{
    "read": {
        "type": "imap",
        "host": "imap.gmail.com",
        "port": 993,
        "login": "<you@email.com>",
        "auth": {
            "type": "oauth",
            "client_id": "<CLIENT_ID>",
            "auth_uri": "<AUTH_URL>",
            "token_uri": "<TOKEN_URL>",
            "client_secret": "<CLIENT_SECRET>",
            "access_token": "<ACCESS_TOKEN>",
            "refresh_token": "<REFRESH_TOKEN>"
        }
    },
}
```

</details>


##### Password

The password based authentication is much simpler, you simply declare the `type` to be `password` and provide it under `raw`:

```json
"auth": {
    "type": "password",
    "raw": "<YOUR_PASSWORD>"
}
```

> If you're using Gmail, you will need to setup [App Passwords](https://support.google.com/accounts/answer/185833?hl=en).


<details>
<summary>Putting it all together</summary>

```
{
    "read": {
        "type": "imap",
        "host": "imap.gmail.com",
        "port": 993,
        "login": "<you@email.com>",
        "auth": {
            "type": "password",
            "raw": "<YOUR_PASSWORD>"
        }
    }
}
```

</details>


</details>


### SMTP

Like IMAP, the SMTP configuration supports both password and OAuth based authentication.
The main part of the configuration is declared with a `send` key:

```json
{
    "send": {
        "type": "smtp",
        "host": "smtp.gmail.com",
        "port": 465,
        "login": "<you@email.com>",
        "auth": { /* see below */ }
    },
}
```

> The only supported `send.type` is currently `smtp`

<details>
<summary><h4>Authentication</h4></summary>

The SMTP authentication configuration is very similar and it even provides the same parameters!

##### OAuth

OAuth setup will slightly vary from provider to provider, below you can find a list of OAuth setup guides from tested providers:

* [Gmail OAuth instructions](https://developers.google.com/identity/protocols/oauth2#1.-obtain-oauth-2.0-credentials-from-the-dynamic_data.setvar.console_name-.)

The `auth` field for OAuth will resemble the following:

```json
"auth": {
    "type": "oauth",
    "client_id": "<CLIENT_ID>",
    "auth_uri": "<AUTH_URL>",
    "token_uri": "<TOKEN_URL>",
    "client_secret": "<CLIENT_SECRET>",
    "access_token": "<ACCESS_TOKEN>",
    "refresh_token": "<REFRESH_TOKEN>"
}
```


> eCTT will automatically refresh your access token if it has expired, *however*
> it will *not* update the configuration file.

<details>
<summary>Putting it all together</summary>

```
{
    "read": {
        "type": "smtp",
        "host": "smtp.gmail.com",
        "port": 465,
        "login": "<you@email.com>",
        "auth": {
            "type": "oauth",
            "client_id": "<CLIENT_ID>",
            "auth_uri": "<AUTH_URL>",
            "token_uri": "<TOKEN_URL>",
            "client_secret": "<CLIENT_SECRET>",
            "access_token": "<ACCESS_TOKEN>",
            "refresh_token": "<REFRESH_TOKEN>"
        }
    },
}
```

</details>


##### Password

The password based authentication is much simpler, you simply declare the `type` to be `password` and provide it under `raw`:

```json
"auth": {
    "type": "password",
    "raw": "<YOUR_PASSWORD>"
}
```

> If you're using Gmail, you will need to setup [App Passwords](https://support.google.com/accounts/answer/185833?hl=en).


<details>
<summary>Putting it all together</summary>

```
{
    "read": {
        "type": "smtp",
        "host": "smtp.gmail.com",
        "port": 465,
        "login": "<you@email.com>",
        "auth": {
            "type": "password",
            "raw": "<YOUR_PASSWORD>"
        }
    }
}
```

</details>

</details>

After setting both IMAP and SMTP, your file should look like this:

```json
{
    "read": {
        "type": "imap",
        "host": "imap.gmail.com",
        "port": 993,
        "login": "you@email.com",
        "auth": {
            <YOUR AUTH SETUP>
        }
    },
    "send": {
        "type": "smtp",
        "host": "smtp.gmail.com",
        "port": 465,
        "login": "you@email.com",
        "auth": {
            <YOUR AUTH SETUP>
        }
    }
}

```
