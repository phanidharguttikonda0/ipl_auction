ALTER TABLE players ADD COLUMN previous_team TEXT;
ALTER TABLE users ADD COLUMN favorite_team varchar(50);
ALTER TABLE players
ALTER COLUMN base_price TYPE REAL;
ALTER TABLE ROOMS DROP COLUMN participated_count;
ALTER TABLE SOLD_PLAYERS ALTER COLUMN amount TYPE REAL;
ALTER SEQUENCE players_id_seq RESTART WITH 1;
ALTER TABLE participants
ALTER COLUMN purse_remaining TYPE REAL;
ALTER TABLE sold_players
    ADD COLUMN id SERIAL;
ALTER TABLE unsold_players
    ADD COLUMN id SERIAL;
ALTER TABLE participants
    ADD COLUMN remaining_rtms smallint DEFAULT 3 NOT NULL ; -- 3 RTMs per team by default
-- ALTER TABLE players
--     ADD COLUMN player_rating INTEGER
--         CHECK (player_rating >= 0 AND player_rating <= 100);
ALTER TABLE players ADD COLUMN is_indian BOOLEAN default true;

-- Create ENUM for feedback type
CREATE TYPE feedback_type_enum AS ENUM ('bug', 'rating', 'improvements');

-- Create table
CREATE TABLE user_feedback (
                               id BIGSERIAL PRIMARY KEY,
                               user_id INT not NULL,
                               feedback_type feedback_type_enum NOT NULL,
                               rating_value SMALLINT NULL CHECK (rating_value BETWEEN 1 AND 5),
                               title VARCHAR(255) NOT NULL,
                               description TEXT NOT NULL,
                               created_at TIMESTAMP DEFAULT NOW()
); -- we are going to take this request and add that into the database via a background task only
CREATE INDEX idx_rooms_created_at_id_desc
    ON rooms (created_at DESC, id DESC);
ALTER TABLE rooms ADD COLUMN strict_mode BOOLEAN default false;
ALTER TABLE users ADD COLUMN location Text;
ALTER table players ADD COLUMN profile_url Text;
ALTER TABLE players ADD COLUMN pool_no SMALLINT;
ALTER TABLE user_feedback
    ADD CONSTRAINT fk_user_feedback_user
        FOREIGN KEY (user_id)
            REFERENCES users(id)
            ON DELETE SET NULL
            ON UPDATE CASCADE;

-- WE ARE GOING TO ADD TIME STAMP FROM EACH AND EVERY RECORD OF PLAYER SOLD AND PLAYER UNSOLD
ALTER TABLE sold_players
    ADD COLUMN created_at TIMESTAMP NOT NULL DEFAULT NOW();

ALTER TABLE unsold_players
    ADD COLUMN created_at TIMESTAMP NOT NULL DEFAULT NOW();


-- THESE ARE THE NEW TABLES FROM NOW AFTER AUCTION COMPLETION EVERY SOLD AND UNSOLD PLAYER MOVES TO THESE TABLES
CREATE TABLE COMPLETED_ROOMS_UNSOLD_PLAYERS (
  player_id INT NOT NULL,
  room_id UUID NOT NULL,
  created_at TIMESTAMP NOT NULL,
  PRIMARY KEY (player_id, room_id),
  FOREIGN KEY (player_id) REFERENCES players(id) ON DELETE CASCADE,
  FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
);

CREATE TABLE COMPLETED_ROOMS_SOLD_PLAYERS (
                                              player_id INT NOT NULL,
                                              participant_id INT NOT NULL,
                                              room_id UUID NOT NULL,
                                              amount REAL NOT NULL,
                                              created_at TIMESTAMP NOT NULL,
                                              PRIMARY KEY (player_id, room_id),
                                              FOREIGN KEY (player_id) REFERENCES players(id) ON DELETE CASCADE,
                                              FOREIGN KEY (participant_id) REFERENCES participants(id) ON DELETE CASCADE,
                                              FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
);
-- Fast lookup by room
CREATE INDEX idx_completed_sold_room_id
    ON COMPLETED_ROOMS_SOLD_PLAYERS (room_id);

-- Fast lookup by participant (buyer history)
CREATE INDEX idx_completed_sold_participant_id
    ON COMPLETED_ROOMS_SOLD_PLAYERS (participant_id);

-- Optional: time-based queries ( not adding now , in future if needed we will add it )
CREATE INDEX idx_completed_sold_created_at
    ON COMPLETED_ROOMS_SOLD_PLAYERS (created_at);

-- Fast lookup by room
CREATE INDEX idx_completed_unsold_room_id
    ON COMPLETED_ROOMS_UNSOLD_PLAYERS (room_id);

-- Optional: time-based queries ( not adding now , in future if needed we will add it )
CREATE INDEX idx_completed_unsold_created_at
    ON COMPLETED_ROOMS_UNSOLD_PLAYERS (created_at);