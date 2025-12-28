Password: 
--
-- PostgreSQL database dump
--

\restrict 24ccD5b6cPJoK4XfRdnPxkyvOTvEHgZdMaBdypiiv44ODW9t0aEUODEN8YCLHow

-- Dumped from database version 16.9
-- Dumped by pg_dump version 18.1 (Debian 18.1-1.pgdg13+2)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: feedback_type_enum; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.feedback_type_enum AS ENUM (
    'bug',
    'rating',
    'improvements'
);


--
-- Name: room_status; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.room_status AS ENUM (
    'not_started',
    'in_progress',
    'completed'
);


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: participants; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.participants (
    id integer NOT NULL,
    user_id integer NOT NULL,
    room_id uuid NOT NULL,
    team_selected character varying(100),
    purse_remaining real DEFAULT 100.00,
    remaining_rtms smallint DEFAULT 3 NOT NULL
);


--
-- Name: participants_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.participants_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: participants_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.participants_id_seq OWNED BY public.participants.id;


--
-- Name: players; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.players (
    id integer NOT NULL,
    name character varying(100) NOT NULL,
    base_price real NOT NULL,
    country character varying(100),
    role character varying(100),
    previous_team text,
    is_indian boolean DEFAULT true,
    profile_url text,
    pool_no smallint
);


--
-- Name: players_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.players_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: players_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.players_id_seq OWNED BY public.players.id;


--
-- Name: rooms; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.rooms (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    creator_id integer NOT NULL,
    status public.room_status DEFAULT 'not_started'::public.room_status,
    created_at timestamp with time zone DEFAULT now(),
    completed_at timestamp with time zone,
    strict_mode boolean DEFAULT false
);


--
-- Name: sold_players; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.sold_players (
    player_id integer NOT NULL,
    participant_id integer NOT NULL,
    room_id uuid NOT NULL,
    amount real NOT NULL,
    id integer NOT NULL
);


--
-- Name: sold_players_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.sold_players_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: sold_players_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.sold_players_id_seq OWNED BY public.sold_players.id;


--
-- Name: unsold_players; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.unsold_players (
    player_id integer NOT NULL,
    room_id uuid NOT NULL,
    id integer NOT NULL
);


--
-- Name: unsold_players_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.unsold_players_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: unsold_players_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.unsold_players_id_seq OWNED BY public.unsold_players.id;


--
-- Name: user_feedback; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_feedback (
    id bigint NOT NULL,
    feedback_type public.feedback_type_enum NOT NULL,
    rating_value smallint,
    title character varying(255) NOT NULL,
    description text NOT NULL,
    created_at timestamp without time zone DEFAULT now(),
    user_id integer,
    CONSTRAINT user_feedback_rating_value_check CHECK (((rating_value >= 1) AND (rating_value <= 5)))
);


--
-- Name: user_feedback_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.user_feedback_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: user_feedback_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.user_feedback_id_seq OWNED BY public.user_feedback.id;


--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id integer NOT NULL,
    username character varying(100) NOT NULL,
    mail_id character varying(255) NOT NULL,
    google_sid character varying(255),
    created_at timestamp with time zone DEFAULT now(),
    favorite_team character varying(50),
    location text
);


--
-- Name: users_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.users_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: users_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.users_id_seq OWNED BY public.users.id;


--
-- Name: participants id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants ALTER COLUMN id SET DEFAULT nextval('public.participants_id_seq'::regclass);


--
-- Name: players id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players ALTER COLUMN id SET DEFAULT nextval('public.players_id_seq'::regclass);


--
-- Name: sold_players id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sold_players ALTER COLUMN id SET DEFAULT nextval('public.sold_players_id_seq'::regclass);


--
-- Name: unsold_players id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.unsold_players ALTER COLUMN id SET DEFAULT nextval('public.unsold_players_id_seq'::regclass);


--
-- Name: user_feedback id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_feedback ALTER COLUMN id SET DEFAULT nextval('public.user_feedback_id_seq'::regclass);


--
-- Name: users id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users ALTER COLUMN id SET DEFAULT nextval('public.users_id_seq'::regclass);


--
-- Name: participants participants_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants
    ADD CONSTRAINT participants_pkey PRIMARY KEY (id);


--
-- Name: participants participants_user_id_room_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants
    ADD CONSTRAINT participants_user_id_room_id_key UNIQUE (user_id, room_id);


--
-- Name: players players_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.players
    ADD CONSTRAINT players_pkey PRIMARY KEY (id);


--
-- Name: rooms rooms_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_pkey PRIMARY KEY (id);


--
-- Name: sold_players sold_players_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sold_players
    ADD CONSTRAINT sold_players_pkey PRIMARY KEY (player_id, room_id);


--
-- Name: participants unique_room_team; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants
    ADD CONSTRAINT unique_room_team UNIQUE (room_id, team_selected);


--
-- Name: unsold_players unsold_players_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.unsold_players
    ADD CONSTRAINT unsold_players_pkey PRIMARY KEY (player_id, room_id);


--
-- Name: user_feedback user_feedback_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_feedback
    ADD CONSTRAINT user_feedback_pkey PRIMARY KEY (id);


--
-- Name: users users_google_sid_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_google_sid_key UNIQUE (google_sid);


--
-- Name: users users_mail_id_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_mail_id_key UNIQUE (mail_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: users users_username_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_username_key UNIQUE (username);


--
-- Name: idx_participants_room_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_participants_room_id ON public.participants USING btree (room_id);


--
-- Name: idx_participants_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_participants_user_id ON public.participants USING btree (user_id);


--
-- Name: idx_rooms_created_at_id_desc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_rooms_created_at_id_desc ON public.rooms USING btree (created_at DESC, id DESC);


--
-- Name: idx_rooms_creator_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_rooms_creator_id ON public.rooms USING btree (creator_id);


--
-- Name: idx_sold_players_room_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_sold_players_room_id ON public.sold_players USING btree (room_id);


--
-- Name: idx_unsold_players_room_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_unsold_players_room_id ON public.unsold_players USING btree (room_id);


--
-- Name: participants participants_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants
    ADD CONSTRAINT participants_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id) ON DELETE CASCADE;


--
-- Name: participants participants_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.participants
    ADD CONSTRAINT participants_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: rooms rooms_creator_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.rooms
    ADD CONSTRAINT rooms_creator_id_fkey FOREIGN KEY (creator_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: sold_players sold_players_participant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sold_players
    ADD CONSTRAINT sold_players_participant_id_fkey FOREIGN KEY (participant_id) REFERENCES public.participants(id) ON DELETE CASCADE;


--
-- Name: sold_players sold_players_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sold_players
    ADD CONSTRAINT sold_players_player_id_fkey FOREIGN KEY (player_id) REFERENCES public.players(id) ON DELETE CASCADE;


--
-- Name: sold_players sold_players_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.sold_players
    ADD CONSTRAINT sold_players_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id) ON DELETE CASCADE;


--
-- Name: unsold_players unsold_players_player_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.unsold_players
    ADD CONSTRAINT unsold_players_player_id_fkey FOREIGN KEY (player_id) REFERENCES public.players(id) ON DELETE CASCADE;


--
-- Name: unsold_players unsold_players_room_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.unsold_players
    ADD CONSTRAINT unsold_players_room_id_fkey FOREIGN KEY (room_id) REFERENCES public.rooms(id) ON DELETE CASCADE;

ALTER TABLE public.user_feedback
    ADD CONSTRAINT fk_user_feedback_user
        FOREIGN KEY (user_id)
            REFERENCES public.users(id)
            ON DELETE SET NULL
            ON UPDATE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict 24ccD5b6cPJoK4XfRdnPxkyvOTvEHgZdMaBdypiiv44ODW9t0aEUODEN8YCLHow

