-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ==================================================
-- ENUMS
-- ==================================================
-- Room status enum: clearer, professional names
CREATE TYPE room_status AS ENUM ('not_started', 'in_progress', 'completed');

-- ==================================================
-- USERS TABLE
-- ==================================================
CREATE TABLE users (
                       id SERIAL PRIMARY KEY,
                       username VARCHAR(100) UNIQUE NOT NULL,
                       mail_id VARCHAR(255) UNIQUE NOT NULL,
                       google_sid VARCHAR(255) UNIQUE,
                       created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ==================================================
-- ROOMS TABLE
-- ==================================================
CREATE TABLE rooms (
                       id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                       participated_count INT DEFAULT 0,
                       creator_id INT NOT NULL,
                       status room_status DEFAULT 'not_started',
                       created_at TIMESTAMPTZ DEFAULT NOW(),
                       completed_at TIMESTAMPTZ,
                       FOREIGN KEY (creator_id) REFERENCES users(id) ON DELETE CASCADE
);

-- ==================================================
-- PARTICIPANTS TABLE
-- ==================================================
CREATE TABLE participants (
                              id SERIAL PRIMARY KEY,
                              user_id INT NOT NULL,
                              room_id UUID NOT NULL,
                              team_selected VARCHAR(100),
                              purse_remaining NUMERIC(10,2) DEFAULT 100.00, -- in crores (₹)
                              FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                              FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE,
                              UNIQUE (user_id, room_id)
);

-- ==================================================
-- PLAYERS TABLE
-- ==================================================
CREATE TABLE players (
                         id SERIAL PRIMARY KEY,
                         name VARCHAR(100) NOT NULL,
                         base_price NUMERIC(10,2) NOT NULL,
                         country VARCHAR(100),
                         role VARCHAR(100)
);

-- ==================================================
-- UNSOLD PLAYERS TABLE
-- ==================================================
CREATE TABLE unsold_players (
                                player_id INT NOT NULL,
                                room_id UUID NOT NULL,
                                PRIMARY KEY (player_id, room_id),
                                FOREIGN KEY (player_id) REFERENCES players(id) ON DELETE CASCADE,
                                FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
);

-- ==================================================
-- SOLD PLAYERS TABLE
-- ==================================================
CREATE TABLE sold_players (
                              player_id INT NOT NULL,
                              participant_id INT NOT NULL,
                              room_id UUID NOT NULL,
                              amount NUMERIC(10,2) NOT NULL,
                              PRIMARY KEY (player_id, room_id),
                              FOREIGN KEY (player_id) REFERENCES players(id) ON DELETE CASCADE,
                              FOREIGN KEY (participant_id) REFERENCES participants(id) ON DELETE CASCADE,
                              FOREIGN KEY (room_id) REFERENCES rooms(id) ON DELETE CASCADE
);

-- ==================================================
-- INDEXES (for performance)
-- ==================================================
CREATE INDEX idx_participants_user_id ON participants(user_id);
CREATE INDEX idx_participants_room_id ON participants(room_id);
CREATE INDEX idx_sold_players_room_id ON sold_players(room_id);
CREATE INDEX idx_unsold_players_room_id ON unsold_players(room_id);
CREATE INDEX idx_rooms_creator_id ON rooms(creator_id);
