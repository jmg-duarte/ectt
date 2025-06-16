# eCTT

> Short for eletronic-[CTT](https://en.wikipedia.org/wiki/CTT_Correios_de_Portugal).

Read and send emails from your CLI.

## Build

```
cargo b -r
```

## Configuration

eCTT's configuration is based on `himalaya`'s configuration, however, we're using JSON instead of TOML and we support less options.

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


#### Authentication

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

> [!NOTE]
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

> [!WARNING]
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
            "raw": <YOUR_PASSWORD>
        }
    }
}
```

</details>


</details>
