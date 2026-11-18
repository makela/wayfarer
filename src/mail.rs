//! The emails to subscribers and to the owner, and how they are sent: by
//! SMTP, e.g. through Gmail, or, without `SMTP_HOST`, only logged.

use lettre::message::header::{HeaderName, HeaderValue};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::stub::AsyncStubTransport;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use maud::{Markup, html};
use std::fmt::Write;

use crate::posts::PostPage;
use crate::subscribers::Subscriber;

/// Who the emails are from, by default.
pub const DEFAULT_FROM: &str = "Tai Chi Wayfarer <taichiwayfarer@gmail.com>";

/// How the emails go out.
pub enum Mailer {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    /// They don't: they are only logged, e.g. in development.
    Log,
    /// For tests: keeps them, or fails as told.
    Stub(AsyncStubTransport),
}

/// How the SMTP connection is secured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    /// From the start, usually port 465, as Gmail's.
    Tls,
    /// Upgraded after connecting, usually port 587.
    StartTls,
    /// None, for a local relay.
    None,
}

impl Encryption {
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "tls" => Ok(Self::Tls),
            "starttls" => Ok(Self::StartTls),
            "none" => Ok(Self::None),
            other => Err(format!(
                "SMTP_ENCRYPTION should be tls, starttls or none, got {other}"
            )),
        }
    }
}

/// Whether a failed email is worth trying again.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// E.g. the connection failed or the daily limit is reached.
    Temporary(String),
    /// The server refused the email itself, e.g. its address.
    Permanent(String),
}

impl Mailer {
    pub fn smtp(
        host: &str,
        port: u16,
        encryption: Encryption,
        login: Option<(String, String)>,
    ) -> Result<Self, lettre::transport::smtp::Error> {
        let builder = match encryption {
            Encryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(host)?,
            Encryption::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)?,
            Encryption::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host),
        };
        let builder = builder.port(port);
        let builder = match login {
            Some((user, password)) => builder.credentials(Credentials::new(user, password)),
            None => builder,
        };
        Ok(Self::Smtp(builder.build()))
    }

    pub async fn send(&self, message: Message) -> Result<(), Failure> {
        match self {
            Self::Smtp(smtp) => smtp.send(message).await.map(drop).map_err(|error| {
                // Gmail answers a reached daily limit with a permanent code
                // (550 5.4.5), but it passes in a day.
                let text = error.to_string();
                if error.is_permanent() && !text.contains("5.4.5") {
                    Failure::Permanent(text)
                } else {
                    Failure::Temporary(text)
                }
            }),
            Self::Log => {
                let to = message
                    .headers()
                    .get_raw("To")
                    .unwrap_or_default()
                    .to_string();
                let subject = message
                    .headers()
                    .get_raw("Subject")
                    .unwrap_or_default()
                    .to_string();
                tracing::info!(%to, %subject, "Email not sent, as SMTP_HOST isn't set");
                tracing::debug!(email = %String::from_utf8_lossy(&message.formatted()));
                Ok(())
            }
            Self::Stub(stub) => stub
                .send(message)
                .await
                .map_err(|error| Failure::Temporary(error.to_string())),
        }
    }
}

/// What every email needs to know.
pub struct Context<'a> {
    pub from: &'a Mailbox,
    /// Public address of the site without a trailing slash.
    pub site_url: &'a str,
}

impl Context<'_> {
    fn link(&self, path: &str, token: &str) -> String {
        format!("{}{path}?token={token}", self.site_url)
    }

    fn unsubscribe_link(&self, subscriber: &Subscriber) -> String {
        self.link("/unsubscribe", &subscriber.token)
    }

    /// An email to a subscriber, with text and HTML versions of the same.
    fn to_subscriber(
        &self,
        subscriber: &Subscriber,
        subject: &str,
        text: &str,
        body: &Markup,
        unsubscribe: bool,
    ) -> Result<Message, String> {
        let to: Mailbox = subscriber
            .email
            .parse()
            .map_err(|error| format!("{}: {error}", subscriber.email))?;
        let mut builder = Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject);
        if unsubscribe {
            // One-click unsubscribe (RFC 8058), which Gmail and others ask of
            // bulk senders and show beside the sender.
            let link = self.unsubscribe_link(subscriber);
            builder = builder
                .raw_header(HeaderValue::new(
                    HeaderName::new_from_ascii_str("List-Unsubscribe"),
                    format!("<{link}>"),
                ))
                .raw_header(HeaderValue::new(
                    HeaderName::new_from_ascii_str("List-Unsubscribe-Post"),
                    "List-Unsubscribe=One-Click".to_string(),
                ));
        }
        let mut text = text.to_string();
        let html = html_page(
            subject,
            body,
            unsubscribe.then(|| self.unsubscribe_link(subscriber)),
        );
        if unsubscribe {
            let _ = write!(
                text,
                "\n\n--\nYou get this as you subscribed to new posts of Tai Chi Wayfarer.\n\
                 Unsubscribe: {}\n",
                self.unsubscribe_link(subscriber)
            );
        }
        builder
            .multipart(MultiPart::alternative_plain_html(text, html.into_string()))
            .map_err(|error| error.to_string())
    }

    /// The link to confirm a sign-up.
    pub fn confirm(&self, subscriber: &Subscriber) -> Result<Message, String> {
        let link = self.link("/confirm", &subscriber.token);
        let text = format!(
            "Hi!\n\nPlease confirm that you'd like to get the new posts of Tai Chi Wayfarer \
             by email:\n\n{link}\n\nIf you didn't ask for this, just ignore this email, \
             and you won't hear from me again.\n\nTimo"
        );
        let body = html! {
            p { "Hi!" }
            p { "Please confirm that you'd like to get the new posts of Tai Chi Wayfarer by email:" }
            (button(&link, "Confirm"))
            p { "If you didn't ask for this, just ignore this email, and you won't hear from me again." }
            p { "Timo" }
        };
        self.to_subscriber(
            subscriber,
            "Confirm your subscription to Tai Chi Wayfarer",
            &text,
            &body,
            false,
        )
    }

    /// To someone who asked for updates before the form: how it works now.
    pub fn welcome(&self, subscriber: &Subscriber) -> Result<Message, String> {
        let text = format!(
            "Hi!\n\nSome time ago you asked me to tell you when I publish new posts on \
             Tai Chi Wayfarer. Thank you! From now on, each new post comes to you by \
             email automatically. You don't need to do anything.\n\n\
             If you'd rather not get them, you can unsubscribe with the link below.\n\n\
             The posts are at {}\n\nTimo",
            self.site_url
        );
        let body = html! {
            p { "Hi!" }
            p {
                "Some time ago you asked me to tell you when I publish new posts on "
                a href=(self.site_url) { "Tai Chi Wayfarer" }
                ". Thank you! From now on, each new post comes to you by email automatically. You don't need to do anything."
            }
            p { "If you'd rather not get them, you can unsubscribe with the link below." }
            p { "Timo" }
        };
        self.to_subscriber(
            subscriber,
            "New Tai Chi Wayfarer posts now come by email",
            &text,
            &body,
            true,
        )
    }

    /// A new post.
    pub fn post(&self, subscriber: &Subscriber, page: &PostPage) -> Result<Message, String> {
        let link = format!("{}{}", self.site_url, page.post.path());
        let date = page.post.published_at.format("%B %e, %Y").to_string();
        let title = &page.post.title;
        let text = format!(
            "{title}\n{date}\n\n{}\n\nRead the post: {link}",
            page.description
        );
        let image = page.image.as_ref().map(|image| {
            let src = if image.src.starts_with("http") {
                image.src.clone()
            } else {
                format!("{}{}", self.site_url, image.src)
            };
            (src, image.alt.clone())
        });
        let body = html! {
            h1 style="font-size:24px;font-weight:normal;margin:0 0 4px" {
                a href=(link) style="color:#0f172b;text-decoration:none" { (title) }
            }
            p style="margin:0 0 16px;color:#62748e;font-style:italic" { (date) }
            @if let Some((src, alt)) = image {
                a href=(link) {
                    img src=(src) alt=(alt) width="560" style="width:100%;max-width:560px;height:auto;border-radius:8px";
                }
            }
            p { (page.description) }
            (button(&link, "Read the post"))
        };
        self.to_subscriber(subscriber, title, &text, &body, true)
    }

    /// To the owner: someone confirmed their subscription. Replies go to them.
    pub fn new_subscriber(
        &self,
        owner: &Mailbox,
        subscriber: &Subscriber,
    ) -> Result<Message, String> {
        let reply_to: Mailbox = subscriber
            .email
            .parse()
            .map_err(|error| format!("{}: {error}", subscriber.email))?;
        let note = if subscriber.note.is_empty() {
            "(They didn't write anything about themselves.)"
        } else {
            &subscriber.note
        };
        Message::builder()
            .from(self.from.clone())
            .reply_to(reply_to)
            .to(owner.clone())
            .subject(format!("New subscriber: {}", subscriber.email))
            .body(format!(
                "{} subscribed to new posts.\n\n{note}\n\nReply to write to them.\n",
                subscriber.email
            ))
            .map_err(|error| error.to_string())
    }
}

fn button(href: &str, label: &str) -> Markup {
    html! {
        p style="margin:24px 0" {
            a href=(href) style="background:#0f172b;color:#f8fafc;padding:10px 20px;border-radius:8px;text-decoration:none;display:inline-block" {
                (label)
            }
        }
    }
}

fn html_page(title: &str, body: &Markup, unsubscribe: Option<String>) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
            }
            body style="margin:0;padding:24px 16px;background:#f8fafc;color:#0f172b;font-family:Helvetica,Arial,sans-serif;font-size:16px;line-height:1.5" {
                div style="max-width:560px;margin:0 auto" {
                    (body)
                    @if let Some(link) = unsubscribe {
                        p style="margin-top:32px;font-size:13px;color:#62748e" {
                            "You get this as you subscribed to new posts of Tai Chi Wayfarer. "
                            a href=(link) style="color:#62748e" { "Unsubscribe" }
                        }
                    }
                }
            }
        }
    }
}
