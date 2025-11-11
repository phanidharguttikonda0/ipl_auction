ALTER TABLE players ADD COLUMN previous_team TEXT;
ALTER TABLE users ADD COLUMN favorite_team varchar(50);
ALTER TABLE players
ALTER COLUMN base_price TYPE REAL;
ALTER TABLE ROOMS DROP COLUMN participated_count;