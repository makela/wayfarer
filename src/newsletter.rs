//! Sends the emails: each new post to the subscribers when its time comes,
//! and the emails of signing up. Runs in the server, waking up every minute
//! and when someone signs up.
//!
//! The first time, the posts published so far are only marked as announced,
//! so that subscribers don't get every old post. After that a post is sent
//! when it is published, if it was published within a week: a post added
//! with an older date is not sent.

use chrono::{DateTime, Duration, Utc};
use lettre::message::Mailbox;
use std::sync::Arc;
use tokio::sync::Notify;

use crate::mail::{Context, Failure, Mailer};
use crate::posts::PostRepository;
use crate::subscribers::{self, Kind, Pending, Store, timestamp};

/// How often to look for newly published posts and emails to send.
const POLL: std::time::Duration = std::time::Duration::from_secs(60);
/// Posts published longer ago than this aren't sent.
const ANNOUNCE_WITHIN: Duration = Duration::days(7);
/// An email that fails for longer than this is given up on.
const GIVE_UP_AFTER: Duration = Duration::days(3);
/// The longest wait between tries of an email.
const LONGEST_RETRY: Duration = Duration::hours(1);

pub struct Newsletter {
    pub store: Store,
    pub mailer: Mailer,
    pub from: Mailbox,
    /// Who hears of new subscribers.
    pub owner: Mailbox,
    /// Public address of the site without a trailing slash.
    pub site_url: Arc<str>,
    pub posts: Arc<PostRepository>,
    /// The time that decides which posts are published, as the site's.
    pub clock: fn() -> DateTime<Utc>,
    /// A pause after each email, to go easy on the mail server.
    pub pause: std::time::Duration,
}

/// What a round did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub queued: u64,
    pub sent: usize,
    pub failed: usize,
}

impl Newsletter {
    /// Works until the server stops.
    pub async fn run(self, wake: Arc<Notify>) {
        loop {
            if let Err(error) = self.tick(Utc::now()).await {
                tracing::error!(%error, "Newsletter failed, trying again in a minute");
            }
            tokio::select! {
                () = tokio::time::sleep(POLL) => {}
                () = wake.notified() => {}
            }
        }
    }

    /// One round: announces newly published posts, deletes sign-ups left
    /// unconfirmed and sends what's due.
    pub async fn tick(&self, now: DateTime<Utc>) -> subscribers::Result<Report> {
        let queued = self.announce(now).await?;
        let deleted = self.store.delete_unconfirmed(now).await?;
        if deleted > 0 {
            tracing::info!(deleted, "Deleted sign-ups that weren't confirmed");
        }
        let (sent, failed) = self.send_due(now).await?;
        Ok(Report {
            queued,
            sent,
            failed,
        })
    }

    /// Queues the emails of posts published since the last round.
    async fn announce(&self, now: DateTime<Utc>) -> subscribers::Result<u64> {
        let first = !self.store.has_announced().await?;
        let mut queued = 0;
        // Oldest first, so several at once arrive in order.
        let published: Vec<_> = self.posts.published((self.clock)()).collect();
        for post in published.into_iter().rev() {
            if self.store.is_announced(&post.url).await? {
                continue;
            }
            let recent = post
                .published_utc()
                .is_some_and(|published| published > now - ANNOUNCE_WITHIN);
            let send = !first && recent;
            let count = self.store.announce(&post.url, send, now).await?;
            queued += count;
            if send {
                tracing::info!(post = %post.url, emails = count, "Sending a new post");
            } else if !first {
                tracing::warn!(post = %post.url, "Not sending a post published over a week ago");
            }
        }
        if first {
            tracing::info!("Marked the posts so far as sent, without sending them");
        }
        Ok(queued)
    }

    async fn send_due(&self, now: DateTime<Utc>) -> subscribers::Result<(usize, usize)> {
        let context = Context {
            from: &self.from,
            site_url: &self.site_url,
        };
        let (mut sent, mut failed) = (0, 0);
        for pending in self.store.due(now).await? {
            let message = match self.message(&context, &pending) {
                Ok(Some(message)) => message,
                Ok(None) => {
                    self.store.done(pending.id).await?;
                    continue;
                }
                Err(error) => {
                    tracing::error!(%error, email = pending.id, "Could not write an email");
                    self.store.done(pending.id).await?;
                    failed += 1;
                    continue;
                }
            };
            match self.mailer.send(message).await {
                Ok(()) => {
                    self.store.done(pending.id).await?;
                    sent += 1;
                }
                Err(Failure::Permanent(error)) => {
                    tracing::error!(%error, to = %pending.subscriber.email, "Email refused, not trying again");
                    self.store.done(pending.id).await?;
                    failed += 1;
                }
                Err(Failure::Temporary(error)) => {
                    failed += 1;
                    if pending.created_at < timestamp(now - GIVE_UP_AFTER) {
                        tracing::error!(%error, to = %pending.subscriber.email, "Email failed for days, giving up");
                        self.store.done(pending.id).await?;
                    } else {
                        // 1, 2, 4… minutes, at most an hour.
                        let minutes = 1i64 << pending.attempts.clamp(0, 6);
                        let wait = Duration::minutes(minutes).min(LONGEST_RETRY);
                        tracing::warn!(%error, to = %pending.subscriber.email, retry_in = %wait, "Email failed");
                        self.store.retry(pending.id, now + wait).await?;
                        // The others would likely fail too; on the next round.
                        break;
                    }
                }
            }
            tokio::time::sleep(self.pause).await;
        }
        Ok((sent, failed))
    }

    /// The email to send, or None if there's nothing to send anymore.
    fn message(
        &self,
        context: &Context,
        pending: &Pending,
    ) -> Result<Option<lettre::Message>, String> {
        let subscriber = &pending.subscriber;
        match pending.kind {
            Some(Kind::Confirm) if subscriber.confirmed_at.is_none() => {
                context.confirm(subscriber).map(Some)
            }
            Some(Kind::Welcome) => context.welcome(subscriber).map(Some),
            Some(Kind::NewSubscriber) => context.new_subscriber(&self.owner, subscriber).map(Some),
            Some(Kind::Post) => pending
                .post_url
                .as_ref()
                .and_then(|url| self.posts.post_map.get(url))
                .map(|page| context.post(subscriber, page))
                .transpose(),
            Some(Kind::Confirm) | None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use lettre::transport::stub::AsyncStubTransport;
    use std::path::Path;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn at(month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, month, day, 12, 0, 0)
            .single()
            .unwrap_or_default()
    }

    /// Before "Don't Think — Just Do" of October 24.
    fn october_14() -> DateTime<Utc> {
        at(10, 14)
    }
    fn october_25() -> DateTime<Utc> {
        at(10, 25)
    }
    fn november_1() -> DateTime<Utc> {
        at(11, 1)
    }

    fn newsletter(
        store: &Store,
        stub: &AsyncStubTransport,
        clock: fn() -> DateTime<Utc>,
    ) -> Result<Newsletter, Box<dyn std::error::Error>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        Ok(Newsletter {
            store: store.clone(),
            mailer: Mailer::Stub(stub.clone()),
            from: crate::mail::DEFAULT_FROM.parse()?,
            owner: "owner@example.com".parse()?,
            site_url: "https://www.example.com".into(),
            posts: Arc::new(PostRepository::new(
                &root.join("posts"),
                &root.join("dist"),
            )?),
            clock,
            pause: std::time::Duration::ZERO,
        })
    }

    async fn sent(stub: &AsyncStubTransport) -> Vec<(String, String)> {
        stub.messages()
            .await
            .into_iter()
            .map(|(envelope, message)| {
                let to = envelope.to().iter().map(ToString::to_string).collect();
                // Undo the quoted-printable line breaks and escapes of the body.
                (to, message.replace("=\r\n", "").replace("=3D", "="))
            })
            .collect()
    }

    fn subject(message: &str) -> &str {
        message
            .lines()
            .find_map(|line| line.strip_prefix("Subject: "))
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn new_posts_go_out_once_and_old_ones_never() -> TestResult {
        let store = Store::in_memory().await?;
        store.import("reader@example.com", october_14()).await?;
        let stub = AsyncStubTransport::new_ok();

        // The first round: only the welcome email, not the posts so far.
        let report = newsletter(&store, &stub, october_14)?
            .tick(october_14())
            .await?;
        assert_eq!(
            report,
            Report {
                queued: 0,
                sent: 1,
                failed: 0
            }
        );
        let emails = sent(&stub).await;
        assert_eq!(emails[0].0, "reader@example.com");
        assert_eq!(
            subject(&emails[0].1),
            "New Tai Chi Wayfarer posts now come by email"
        );
        assert!(store.is_announced("learn-by-watching").await?);

        // "Don't Think — Just Do" is published: it goes out, once.
        let newsletter = newsletter(&store, &stub, october_25)?;
        let report = newsletter.tick(october_25()).await?;
        assert_eq!(
            report,
            Report {
                queued: 1,
                sent: 1,
                failed: 0
            }
        );
        assert_eq!(newsletter.tick(october_25()).await?, Report::default());
        let emails = sent(&stub).await;
        let post = &emails[1].1;
        // The title, with the dash encoded as it isn't plain ASCII.
        assert_eq!(subject(post), "Don't Think =?utf-8?b?4oCU?= Just Do");
        let token = &store.list().await?[0].token;
        assert!(post.contains(&format!(
            "List-Unsubscribe: <https://www.example.com/unsubscribe?token={token}>"
        )));
        assert!(post.contains("List-Unsubscribe-Post: List-Unsubscribe=One-Click"));
        assert!(post.contains("https://www.example.com/post/dont-think-just-do"));
        Ok(())
    }

    #[tokio::test]
    async fn posts_over_a_week_old_are_not_sent() -> TestResult {
        let store = Store::in_memory().await?;
        store.import("reader@example.com", october_14()).await?;
        let stub = AsyncStubTransport::new_ok();
        newsletter(&store, &stub, october_14)?
            .tick(october_14())
            .await?;
        // Eight days after "Don't Think — Just Do" before anything ran again.
        let report = newsletter(&store, &stub, november_1)?
            .tick(november_1())
            .await?;
        assert_eq!(report.queued, 0);
        assert!(store.is_announced("dont-think-just-do").await?);
        Ok(())
    }

    #[tokio::test]
    async fn signup_emails_go_to_the_right_people() -> TestResult {
        let store = Store::in_memory().await?;
        let stub = AsyncStubTransport::new_ok();
        let newsletter = newsletter(&store, &stub, november_1)?;
        let now = november_1();
        store
            .subscribe("new@example.com", "My teacher is Y", now)
            .await?;
        newsletter.tick(now).await?;
        let token = store.list().await?[0].token.clone();
        let emails = sent(&stub).await;
        assert_eq!(emails[0].0, "new@example.com");
        assert_eq!(
            subject(&emails[0].1),
            "Confirm your subscription to Tai Chi Wayfarer"
        );
        assert!(
            emails[0]
                .1
                .contains(&format!("https://www.example.com/confirm?token={token}"))
        );
        // Not a list email yet: nothing to unsubscribe from.
        assert!(!emails[0].1.contains("List-Unsubscribe"));

        store.confirm(&token, now).await?;
        newsletter.tick(now).await?;
        let emails = sent(&stub).await;
        assert_eq!(emails[1].0, "owner@example.com");
        assert!(emails[1].1.contains("Reply-To: new@example.com"));
        assert!(emails[1].1.contains("My teacher is Y"));
        Ok(())
    }

    #[tokio::test]
    async fn failed_emails_are_tried_again_later() -> TestResult {
        let store = Store::in_memory().await?;
        let now = november_1();
        store.import("a@example.com", now).await?;
        store.import("b@example.com", now).await?;
        let failing = newsletter(&store, &AsyncStubTransport::new_error(), november_1)?;
        let report = failing.tick(now).await?;
        // The first failed; the second waits for the next round.
        assert_eq!(
            report,
            Report {
                queued: 0,
                sent: 0,
                failed: 1
            }
        );
        assert_eq!(store.due(now).await?.len(), 1);

        let stub = AsyncStubTransport::new_ok();
        let working = newsletter(&store, &stub, november_1)?;
        working.tick(now + Duration::minutes(1)).await?;
        assert_eq!(sent(&stub).await.len(), 2);

        // Failing for days: given up.
        store.import("c@example.com", now).await?;
        let later = now + GIVE_UP_AFTER + Duration::minutes(1);
        failing.tick(later).await?;
        assert_eq!(store.due(later + Duration::days(1)).await?.len(), 0);
        Ok(())
    }
}
