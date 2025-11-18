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