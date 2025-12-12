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
                               user_id INT NULL,
                               feedback_type feedback_type_enum NOT NULL,
                               rating_value SMALLINT NULL CHECK (rating_value BETWEEN 1 AND 5),
                               title VARCHAR(255) NOT NULL,
                               description TEXT NOT NULL,
                               created_at TIMESTAMP DEFAULT NOW()
); -- we are going to take this request and add that into the database via a background task only
CREATE INDEX idx_rooms_created_at_id_desc
    ON rooms (created_at DESC, id DESC);
ALTER TABLE rooms ADD COLUMN strict_mode BOOLEAN default false;