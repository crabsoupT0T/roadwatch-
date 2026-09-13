-- Requires a Postgres instance with the PostGIS extension available.
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE potholes (
    id             UUID PRIMARY KEY,
    location       geography(Point, 4326) NOT NULL,
    photo          TEXT NOT NULL,
    note           TEXT NOT NULL DEFAULT '',
    votes          INTEGER NOT NULL DEFAULT 1,
    first_reported TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Spatial index: makes "anything within 10m of this point" a fast index lookup
-- instead of a full-table distance scan. This is the main reason a real
-- database beats the browser-storage prototype as the number of reports grows.
CREATE INDEX potholes_location_idx ON potholes USING GIST (location);

CREATE TABLE votes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pothole_id  UUID NOT NULL REFERENCES potholes(id) ON DELETE CASCADE,
    phone_hash  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (pothole_id, phone_hash)
);

CREATE INDEX votes_pothole_idx ON votes (pothole_id);
