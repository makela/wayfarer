//! The subscribers to new posts by email and the emails waiting for them, in
//! the site's database (see `db.rs`). Signing up is confirmed by email
//! (double opt-in); imported subscribers count as confirmed. The emails are
//! sent by `newsletter.rs`.

use chrono::{DateTime, Duration, Utc};
use sqlx::{Row, SqlitePool};

/// Longest note kept, in characters; the form allows as much.
pub const NOTE_LENGTH: usize = 2000;
/// Confirmation emails at most this often to one address, so the form can't
/// be used to flood someone's inbox.
const CONFIRMATION_INTERVAL: Duration = Duration::days(1);
/// Confirmation emails at most this many an hour in all, to stay well within
/// the sending limits of the mail account.
const CONFIRMATIONS_PER_HOUR: i64 = 30;
/// Sign-ups not confirmed in this time are deleted.
pub const UNCONFIRMED_KEPT: Duration = Duration::days(7);

pub type Result<T> = std::result::Result<T, sqlx::Error>;

/// A time as stored: UTC to the second, as text that sorts by time.
#[must_use]
pub fn timestamp(time: DateTime<Utc>) -> String {
    time.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// A new secret for the links of a subscriber's emails.
fn new_token() -> Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    Ok(bytes.iter().fold(String::new(), |mut text, byte| {
        use std::fmt::Write;
        let _ = write!(text, "{byte:02x}");
        text
    }))
}

/// An email address as typed, trimmed, if it is one.
#[must_use]
pub fn parse_email(text: &str) -> Option<String> {
    let email = text.trim();
    (email.len() <= 254 && email.parse::<lettre::Address>().is_ok()).then(|| email.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscriber {
    pub id: i64,
    pub email: String,
    pub note: String,
    pub token: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

/// What became of a sign-up. The page says the same in every case, so it
/// doesn't tell who is subscribed.
#[derive(Debug, PartialEq, Eq)]
pub enum Signup {
    /// A confirmation email is on its way.
    ConfirmationQueued,
    /// Already subscribed, or a confirmation was sent lately.
    Unchanged,
    /// Too many sign-ups lately; try again later.
    TooMany,
}

/// The kinds of emails, as in the outbox's `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The link to confirm a sign-up.
    Confirm,
    /// To an imported subscriber: new posts now come by email.
    Welcome,
    /// To the owner: someone confirmed, with their note.
    NewSubscriber,
    /// A new post.
    Post,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Confirm => "confirm",
            Self::Welcome => "welcome",
            Self::NewSubscriber => "new_subscriber",
            Self::Post => "post",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        [
            Self::Confirm,
            Self::Welcome,
            Self::NewSubscriber,
            Self::Post,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
    }
}

/// An email waiting in the outbox, with its subscriber.
#[derive(Debug, Clone)]
pub struct Pending {
    pub id: i64,
    /// None for a kind this version doesn't know.
    pub kind: Option<Kind>,
    pub post_url: Option<String>,
    pub created_at: String,
    pub attempts: i64,
    pub subscriber: Subscriber,
}

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

fn subscriber(row: &sqlx::sqlite::SqliteRow) -> Result<Subscriber> {
    Ok(Subscriber {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        note: row.try_get("note")?,
        token: row.try_get("token")?,
        created_at: row.try_get("created_at")?,
        confirmed_at: row.try_get("confirmed_at")?,
    })
}

/// The columns `subscriber` reads, as a literal to `concat!` into queries.
macro_rules! subscriber_columns {
    () => {
        "id, email, note, token, created_at, confirmed_at"
    };
}

impl Store {
    /// The subscribers in the site's database, see `db.rs`.
    #[must_use]
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// No subscribers, in an empty database in memory, for tests.
    pub async fn in_memory() -> Result<Self> {
        Ok(Self::new(crate::db::in_memory().await?))
    }

    /// Signs up `email`, which must be valid (see [`parse_email`]), with a
    /// note about themselves, and queues the email to confirm it.
    pub async fn subscribe(&self, email: &str, note: &str, now: DateTime<Utc>) -> Result<Signup> {
        let note: String = note.trim().chars().take(NOTE_LENGTH).collect();
        let mut tx = self.pool.begin().await?;
        let existing = sqlx::query(
            "SELECT id, confirmed_at, confirmation_sent_at FROM subscribers WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?;
        let id = match existing {
            Some(row) => {
                let id: i64 = row.try_get("id")?;
                let confirmed: Option<String> = row.try_get("confirmed_at")?;
                let sent: Option<String> = row.try_get("confirmation_sent_at")?;
                if confirmed.is_some()
                    || sent.is_some_and(|sent| sent > timestamp(now - CONFIRMATION_INTERVAL))
                {
                    return Ok(Signup::Unchanged);
                }
                if !note.is_empty() {
                    sqlx::query("UPDATE subscribers SET note = ? WHERE id = ?")
                        .bind(&note)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
                Some(id)
            }
            None => None,
        };
        let lately: i64 =
            sqlx::query_scalar("SELECT count(*) FROM subscribers WHERE confirmation_sent_at > ?")
                .bind(timestamp(now - Duration::hours(1)))
                .fetch_one(&mut *tx)
                .await?;
        if lately >= CONFIRMATIONS_PER_HOUR {
            return Ok(Signup::TooMany);
        }
        let id =
            match id {
                Some(id) => id,
                None => sqlx::query_scalar(
                    "INSERT INTO subscribers (email, note, token, created_at) VALUES (?, ?, ?, ?)
                     RETURNING id",
                )
                .bind(email)
                .bind(&note)
                .bind(new_token()?)
                .bind(timestamp(now))
                .fetch_one(&mut *tx)
                .await?,
            };
        sqlx::query("UPDATE subscribers SET confirmation_sent_at = ? WHERE id = ?")
            .bind(timestamp(now))
            .bind(id)
            .execute(&mut *tx)
            .await?;
        queue(&mut tx, id, Kind::Confirm, None, now).await?;
        tx.commit().await?;
        Ok(Signup::ConfirmationQueued)
    }

    pub async fn by_token(&self, token: &str) -> Result<Option<Subscriber>> {
        let row = sqlx::query(concat!(
            "SELECT ",
            subscriber_columns!(),
            " FROM subscribers WHERE token = ?"
        ))
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(subscriber).transpose()
    }

    /// Confirms the sign-up with `token` and tells the owner about it.
    /// Returns the subscriber, or None for an unknown token.
    pub async fn confirm(&self, token: &str, now: DateTime<Utc>) -> Result<Option<Subscriber>> {
        let mut tx = self.pool.begin().await?;
        let confirmed = sqlx::query(concat!(
            "UPDATE subscribers SET confirmed_at = ? WHERE token = ? AND confirmed_at IS NULL
             RETURNING ",
            subscriber_columns!()
        ))
        .bind(timestamp(now))
        .bind(token)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(row) = &confirmed {
            let subscriber = subscriber(row)?;
            queue(&mut tx, subscriber.id, Kind::NewSubscriber, None, now).await?;
            // The confirmation email has done its job, if it is still waiting.
            sqlx::query("DELETE FROM outbox WHERE subscriber_id = ? AND kind = ?")
                .bind(subscriber.id)
                .bind(Kind::Confirm.as_str())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        // Confirmed already: the link was used twice.
        self.by_token(token).await
    }

    /// Deletes the subscriber with `token` and their waiting emails. Returns
    /// whether there was one.
    pub async fn unsubscribe(&self, token: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM subscribers WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Deletes the subscriber with `email`. Returns whether there was one.
    pub async fn remove(&self, email: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM subscribers WHERE email = ?")
            .bind(email.trim())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Adds `email` as confirmed, e.g. someone who asked for updates by
    /// email before the form, and queues a welcome email telling how it
    /// works now. Returns false if the address is already on the list.
    pub async fn import(&self, email: &str, now: DateTime<Utc>) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let id: Option<i64> = sqlx::query_scalar(
            "INSERT INTO subscribers (email, token, created_at, confirmed_at) VALUES (?, ?, ?, ?)
             ON CONFLICT (email) DO NOTHING RETURNING id",
        )
        .bind(email)
        .bind(new_token()?)
        .bind(timestamp(now))
        .bind(timestamp(now))
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(id) = id {
            queue(&mut tx, id, Kind::Welcome, None, now).await?;
        }
        tx.commit().await?;
        Ok(id.is_some())
    }

    /// Everyone on the list, confirmed or not, oldest first.
    pub async fn list(&self) -> Result<Vec<Subscriber>> {
        sqlx::query(concat!(
            "SELECT ",
            subscriber_columns!(),
            " FROM subscribers ORDER BY created_at, id"
        ))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(subscriber)
        .collect()
    }

    /// Whether any post has been announced, i.e. whether the list of
    /// announced posts has been started.
    pub async fn has_announced(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM announced_posts")
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    pub async fn is_announced(&self, url: &str) -> Result<bool> {
        let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM announced_posts WHERE url = ?")
            .bind(url)
            .fetch_optional(&self.pool)
            .await?;
        Ok(found.is_some())
    }

    /// Marks the post as announced, and, if `send`, queues its email to every
    /// confirmed subscriber. Returns how many emails were queued.
    pub async fn announce(&self, url: &str, send: bool, now: DateTime<Utc>) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let new = sqlx::query(
            "INSERT INTO announced_posts (url, announced_at) VALUES (?, ?)
             ON CONFLICT (url) DO NOTHING",
        )
        .bind(url)
        .bind(timestamp(now))
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        let queued = if new && send {
            sqlx::query(
                "INSERT INTO outbox (subscriber_id, kind, post_url, created_at, next_attempt_at)
                 SELECT id, ?, ?, ?, ? FROM subscribers WHERE confirmed_at IS NOT NULL",
            )
            .bind(Kind::Post.as_str())
            .bind(url)
            .bind(timestamp(now))
            .bind(timestamp(now))
            .execute(&mut *tx)
            .await?
            .rows_affected()
        } else {
            0
        };
        tx.commit().await?;
        Ok(queued)
    }

    /// The emails due to be sent by `now`, oldest first.
    pub async fn due(&self, now: DateTime<Utc>) -> Result<Vec<Pending>> {
        sqlx::query(
            "SELECT o.id AS outbox_id, o.kind, o.post_url, o.created_at AS queued_at, o.attempts,
                    s.id, s.email, s.note, s.token, s.created_at, s.confirmed_at
             FROM outbox o JOIN subscribers s ON s.id = o.subscriber_id
             WHERE o.next_attempt_at <= ? ORDER BY o.id",
        )
        .bind(timestamp(now))
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| {
            Ok(Pending {
                id: row.try_get("outbox_id")?,
                kind: Kind::parse(row.try_get("kind")?),
                post_url: row.try_get("post_url")?,
                created_at: row.try_get("queued_at")?,
                attempts: row.try_get("attempts")?,
                subscriber: subscriber(row)?,
            })
        })
        .collect()
    }

    /// Removes a sent email, or one given up on, from the outbox.
    pub async fn done(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM outbox WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Tries the email again at `next`.
    pub async fn retry(&self, id: i64, next: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE outbox SET attempts = attempts + 1, next_attempt_at = ? WHERE id = ?")
            .bind(timestamp(next))
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes sign-ups that weren't confirmed in time. Returns how many.
    pub async fn delete_unconfirmed(&self, now: DateTime<Utc>) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM subscribers WHERE confirmed_at IS NULL AND created_at < ?")
                .bind(timestamp(now - UNCONFIRMED_KEPT))
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }
}

async fn queue(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    subscriber_id: i64,
    kind: Kind,
    post_url: Option<&str>,
    now: DateTime<Utc>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO outbox (subscriber_id, kind, post_url, created_at, next_attempt_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(subscriber_id)
    .bind(kind.as_str())
    .bind(post_url)
    .bind(timestamp(now))
    .bind(timestamp(now))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn noon() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 18, 12, 0, 0)
            .single()
            .unwrap_or_default()
    }

    async fn kinds(store: &Store, now: DateTime<Utc>) -> Result<Vec<(String, Option<Kind>)>> {
        Ok(store
            .due(now)
            .await?
            .into_iter()
            .map(|pending| (pending.subscriber.email, pending.kind))
            .collect())
    }

    #[test]
    fn emails_are_checked() {
        assert_eq!(parse_email(" a@example.com "), Some("a@example.com".into()));
        for bad in [
            "",
            "a",
            "a@",
            "@example.com",
            "a b@example.com",
            "a@example.com\nb",
        ] {
            assert_eq!(parse_email(bad), None, "{bad:?}");
        }
    }

    #[tokio::test]
    async fn signup_is_confirmed_once() -> TestResult {
        let store = Store::in_memory().await?;
        let now = noon();
        let signup = store
            .subscribe("a@example.com", "  My teacher is X ", now)
            .await?;
        assert_eq!(signup, Signup::ConfirmationQueued);
        assert_eq!(
            kinds(&store, now).await?,
            [("a@example.com".into(), Some(Kind::Confirm))]
        );
        let subscriber = store.list().await?.remove(0);
        assert_eq!(subscriber.note, "My teacher is X");
        assert_eq!(subscriber.confirmed_at, None);
        assert_eq!(subscriber.token.len(), 48);

        // Signing up again the same day sends nothing more, in any case.
        assert_eq!(
            store.subscribe("A@example.com", "", now).await?,
            Signup::Unchanged
        );
        assert_eq!(store.due(now).await?.len(), 1);

        let confirmed = store
            .confirm(&subscriber.token, now)
            .await?
            .ok_or("not found")?;
        assert_eq!(confirmed.confirmed_at, Some(timestamp(now)));
        // The owner hears of it, and the confirmation isn't needed anymore.
        assert_eq!(
            kinds(&store, now).await?,
            [("a@example.com".into(), Some(Kind::NewSubscriber))]
        );
        // A second click changes nothing.
        store.confirm(&subscriber.token, now).await?;
        assert_eq!(store.due(now).await?.len(), 1);
        assert!(store.confirm("no-such-token", now).await?.is_none());

        // Confirmed: signing up again sends nothing, even days later.
        let later = now + Duration::days(3);
        assert_eq!(
            store.subscribe("a@example.com", "", later).await?,
            Signup::Unchanged
        );
        Ok(())
    }

    #[tokio::test]
    async fn unconfirmed_signup_can_be_retried_a_day_later() -> TestResult {
        let store = Store::in_memory().await?;
        let now = noon();
        store.subscribe("a@example.com", "", now).await?;
        let next_day = now + Duration::days(1) + Duration::seconds(1);
        let signup = store.subscribe("a@example.com", "Hello", next_day).await?;
        assert_eq!(signup, Signup::ConfirmationQueued);
        assert_eq!(store.list().await?[0].note, "Hello");
        // Not confirmed in a week: gone, with its emails.
        assert_eq!(store.delete_unconfirmed(now + Duration::days(7)).await?, 0);
        assert_eq!(store.delete_unconfirmed(now + Duration::days(9)).await?, 1);
        assert_eq!(store.list().await?.len(), 0);
        assert_eq!(store.due(next_day).await?.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn signups_are_limited_per_hour() -> TestResult {
        let store = Store::in_memory().await?;
        let now = noon();
        for n in 0..CONFIRMATIONS_PER_HOUR {
            let signup = store
                .subscribe(&format!("{n}@example.com"), "", now)
                .await?;
            assert_eq!(signup, Signup::ConfirmationQueued);
        }
        assert_eq!(
            store.subscribe("x@example.com", "", now).await?,
            Signup::TooMany
        );
        let later = now + Duration::hours(1) + Duration::seconds(1);
        assert_eq!(
            store.subscribe("x@example.com", "", later).await?,
            Signup::ConfirmationQueued
        );
        Ok(())
    }

    #[tokio::test]
    async fn imported_are_confirmed_and_welcomed() -> TestResult {
        let store = Store::in_memory().await?;
        let now = noon();
        assert!(store.import("a@example.com", now).await?);
        assert!(!store.import("A@example.com", now).await?);
        assert_eq!(
            kinds(&store, now).await?,
            [("a@example.com".into(), Some(Kind::Welcome))]
        );
        assert!(store.list().await?[0].confirmed_at.is_some());
        // Already on the list: signing up doesn't ask to confirm again.
        assert_eq!(
            store.subscribe("a@example.com", "", now).await?,
            Signup::Unchanged
        );
        Ok(())
    }

    #[tokio::test]
    async fn posts_go_to_confirmed_subscribers_once() -> TestResult {
        let store = Store::in_memory().await?;
        let now = noon();
        store.import("a@example.com", now).await?;
        store.subscribe("b@example.com", "", now).await?;
        for pending in store.due(now).await? {
            store.done(pending.id).await?;
        }
        assert!(!store.has_announced().await?);
        assert_eq!(store.announce("old-post", false, now).await?, 0);
        assert!(store.has_announced().await? && store.is_announced("old-post").await?);
        assert_eq!(store.announce("new-post", true, now).await?, 1);
        assert_eq!(store.announce("new-post", true, now).await?, 0);
        let due = store.due(now).await?;
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].post_url.as_deref(), Some("new-post"));

        // Tried again later, not before.
        store.retry(due[0].id, now + Duration::minutes(5)).await?;
        assert_eq!(store.due(now).await?.len(), 0);
        let retried = store.due(now + Duration::minutes(5)).await?;
        assert_eq!(retried[0].attempts, 1);

        // Unsubscribing takes the waiting emails with it.
        assert!(store.unsubscribe(&retried[0].subscriber.token).await?);
        assert!(!store.unsubscribe(&retried[0].subscriber.token).await?);
        assert_eq!(store.due(now + Duration::days(1)).await?.len(), 0);
        Ok(())
    }
}
