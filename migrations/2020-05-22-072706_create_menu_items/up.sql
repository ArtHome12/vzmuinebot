-- Your SQL goes here

CREATE TABLE menu_items (
  id SERIAL PRIMARY KEY,
  title VARCHAR NOT NULL,
  price smallint NOT NULL,
  category VARCHAR NOT NULL
)