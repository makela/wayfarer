-- Your SQL goes here
CREATE TABLE feedbacks (
  id SERIAL PRIMARY KEY,
  email VARCHAR NOT NULL,
  feedback TEXT NOT NULL,
  created_at timestamp NOT NULL DEFAULT NOW()
)
