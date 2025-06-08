struct OAuth2ImapAuthenticator {
    user: String,
    access_token: AccessToken,
}

impl imap::Authenticator for OAuth2ImapAuthenticator {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user,
            self.access_token.secret(),
        )
    }
}

/*
   // imap is blocking, we wrap it here to avoid blocking the runtime
   let imap_client = spawn_blocking(|| {
       return imap::ClientBuilder::new("imap.gmail.com", 993)
           .connect()
           .unwrap();
   })
   .await
   .unwrap();

   let authenticator = OAuth2ImapAuthenticator {
       user: "duarte.gmj@gmail.com".to_string(),
       access_token: token_result.access_token().to_owned(),
   };

   let result =
       spawn_blocking(
           move || match imap_client.authenticate("XOAUTH2", &authenticator) {
               Ok(mut session) => {
                   session.select("INBOX").unwrap();

                   // fetch message number 1 in this mailbox, along with its RFC822 field.
                   // RFC 822 dictates the format of the body of e-mails
                   let messages = session.fetch("1", "RFC822").unwrap();
                   messages.iter().for_each(|msg| {
                       println!("{:?}", msg);

                       // extract the message's body
                       let body = msg.body().expect("message did not have a body!");
                       let body = std::str::from_utf8(body)
                           .expect("message was not valid utf-8")
                           .to_string();

                       println!("{}", body);
                   });

                   // be nice to the server and log out
                   session.logout().unwrap();
               }
               Err((err, client)) => {
                   tracing::error!("Failed to authenticate to server with error: {err}");
                   panic!()
               }
           },
       )
       .await
       .unwrap();
*/
