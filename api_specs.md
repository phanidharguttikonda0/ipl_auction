# IPL Auction System - API Specifications

## Table of Contents
1. [HTTP API Routes](#http-api-routes)
2. [WebSocket API](#websocket-api)
3. [Project Motto](#project-motto)

---

## HTTP API Routes

All routes except `/health` and `/continue-with-google` require authentication via `Authorization` header with Bearer token.

### Base URL
```
http://localhost:4545
```

---

### 1. Health Check Endpoint

**Route:** `GET /health`

**Description:** Simple health check endpoint

**Authentication:** Not required

**Response:**
```
Status Code: 200 OK
Body: "Health check passed"
```

---

### 2. Google Authentication

**Route:** `POST /continue-with-google`

**Description:** Authenticates user via Google OAuth and returns JWT token in Authorization header

**Authentication:** Not required

**Request Body (JSON):**
```json
{
  "gmail": "user@example.com",
  "google_sid": "google_session_id",
  "favorite_team": "Mumbai Indians" // Optional, can be null for new users
}
```

**Valid Team Names:**
- "Mumbai Indians"
- "Chennai Super Kings"
- "Sun Risers Hyderabad"
- "Punjab Kings"
- "Rajasthan Royals"
- "Royal Challengers Bangalore"
- "Kolkata Knight Riders"
- "Delhi Capitals"
- "Lucknow Super Gaints"
- "Gujarat Titans"

**Success Response:**
```
Status Code: 200 OK
Headers:
  Authorization: Bearer <jwt_token>
Body:
{
  "message": "Login successful"
}
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "error": "Authorization Failed"
}
```

---

### 3. Update Favorite Team

**Route:** `GET /update-favorite-team/{new_team}`

**Description:** Updates the user's favorite team and returns a new JWT token

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `new_team` (String): Name of the new favorite team

**Success Response:**
```
Status Code: 200 OK
Headers:
  Authorization: Bearer <new_jwt_token>
```

**Error Response:**
```
Status Code: 500 Internal Server Error
```

---

### 4. Submit Feedback

**Route:** `POST /feedback`

**Description:** Submit user feedback (bug report, rating, or improvement suggestion)

**Authentication:** Required (Bearer token)

**Request Body (JSON):**
```json
{
  "feedback_type": "bug|rating|improvements",
  "rating_value": 4.5,  // Optional, required only for "rating" type
  "title": "Feedback title",  // Optional for rating type
  "description": "Detailed feedback"  // Optional for rating type
}
```

**Success Response:**
```
Status Code: 200 OK
Body:
{
  "message": "Feedback submitted successfully"
}
```

**Error Responses:**

Invalid feedback_type:
```
Status Code: 400 Bad Request
Body:
{
  "error": "Invalid feedback_type"
}
```

Missing rating_value:
```
Status Code: 400 Bad Request
Body:
{
  "error": "rating_value is required for rating feedback"
}
```

Missing title/description:
```
Status Code: 400 Bad Request
Body:
{
  "error": "title is required" // or "description is required"
}
```

Server Error:
```
Status Code: 500 Internal Server Error
Body:
{
  "error": "Failed to submit feedback"
}
```

---

## Room Management Routes

### 5. Create Room

**Route:** `GET /rooms/create-room/{team_name}/{is_strict_mode}`

**Description:** Creates a new auction room and adds the creator as the first participant

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `team_name` (String): Name of the team selected by the room creator
- `is_strict_mode` (Boolean): Enable strict bidding mode with stricter balance constraints

**Success Response:**
```
Status Code: 200 OK
Body:
{
  "room_id": "uuid-string",
  "team_name": "Mumbai Indians",
  "participant_id": 123,
  "message": "Room Created Successfully"
}
```

**Error Responses:**

Invalid Team Name:
```
Status Code: 400 Bad Request
Body:
{
  "message": "Invalid Team Name"
}
```

Server Error:
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "Internal Server Error"
}
```

**Note:** `is_strict_mode` when `true` enables advanced bidding constraints that enforce minimum balance requirements per player segment (0-4, 5-9, 10-14 players).

---

### 6. Get Remaining Teams

**Route:** `GET /rooms/join-room-get-teams/{room_id}`

**Description:** Returns remaining available teams for a room. If user is already a participant, returns their participant details instead.

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room_id` (String): The room ID to join

**Success Response - New Participant:**
```
Status Code: 200 OK
Body:
{
  "remaining_teams": ["Mumbai Indians", "Chennai Super Kings", ...],
  "message": "Join with the remaining teams"
}
```

**Success Response - Already a Participant:**
```
Status Code: 200 OK
Body:
{
  "room_id": "uuid-string",
  "team_name": "Mumbai Indians",
  "participant_id": 123,
  "message": "Already a participant"
}
```

**Success Response - Room Closed (Completed):**
```
Status Code: 200 OK
Body:
{
  "message": "Room Closed"
}
```

**Success Response - Room Closed (Auction Started):**
```
Status Code: 200 OK
Body:
{
  "message": "Room Closed, Auction Started"
}
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "Internal Server Error"
}
```

---

### 7. Join Room

**Route:** `GET /rooms/join-room/{room_id}/{team_name}`

**Description:** Adds a participant to an existing room with the selected team

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room_id` (String): The room ID to join
- `team_name` (String): Name of the team selected by the participant

**Success Response:**
```
Status Code: 200 OK
Body:
{
  "room_id": "uuid-string",
  "team_name": "Mumbai Indians",
  "participant_id": 123,
  "message": "Room Created Successfully"
}
```

**Error Responses:**

Invalid Team Name:
```
Status Code: 400 Bad Request
Body:
{
  "message": "Invalid Team Name"
}
```

Server Error:
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "Internal Server Error"
}
```

---

### 8. Get Auctions Played

**Route:** `GET /rooms/get-auctions-played/{per_page}/{room_id}/{last_record_time_stamp}`

**Description:** Returns paginated list of auction rooms participated by the authenticated user

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `per_page` (Integer): Number of rooms to return per page
- `room_id` (String): Room ID for cursor-based pagination (use empty string "" for first request)
- `last_record_time_stamp` (String): Base64-encoded timestamp of last record for pagination (use empty string "" for first request)

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "room_id": "uuid-string",
    "created_at": "2025-01-15T10:30:00Z"
  },
  {
    "room_id": "uuid-string-2",
    "created_at": "2025-01-16T14:20:00Z"
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "error in getting rooms"
}
```

**Pagination Example:**
First request: `/rooms/get-auctions-played/10//`  
Subsequent requests: `/rooms/get-auctions-played/10/last-room-id/base64-encoded-timestamp`

---

### 9. Get Participants in Room

**Route:** `GET /rooms/get-participants/{room_id}`

**Description:** Returns list of all participants in a specific room with their participant IDs and team names

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room_id` (String): The room ID

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "participant_id": "123",
    "team_name": "Mumbai Indians"
  },
  {
    "participant_id": "124",
    "team_name": "Chennai Super Kings"
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "error in getting rooms"
}
```

---

## Player Operations Routes

### 10. Get Team Details

**Route:** `GET /players/get-team-details/{participant_id}`

**Description:** Returns team statistics including balance, total players, and role-wise player counts

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `participant_id` (Integer): The participant ID

**Success Response:**
```
Status Code: 200 OK
Body:
{
  "remaining_balance": 45.50,
  "total_players": 8,
  "total_batsmans": 3,
  "total_bowlers": 2,
  "all_rounders": 3
}
```

**Error Responses:**

Balance Fetch Error:
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching balance"
}
```

Team Details Fetch Error:
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching team details"
}
```

---

### 11. Get Team Players

**Route:** `GET /players/get-team-players/{participant_id}/{status}`

**Description:** Returns list of all players bought by a team with their details

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `participant_id` (Integer): The participant ID
- `status` (String): Room status - "in_progress", "not_started", or "completed"

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "player_id": 1,
    "player_name": "Virat Kohli",
    "role": "Batsman",
    "brought_price": 15.50
  },
  {
    "player_id": 2,
    "player_name": "Jasprit Bumrah",
    "role": "Bowler",
    "brought_price": 12.00
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching team players"
}
```

**Note:** The `status` parameter determines which table to query:
- "completed" queries `completed_rooms_sold_players`
- Other statuses query `sold_players`

---

### 12. Get Sold Players

**Route:** `GET /players/get-sold-players/{room_id}/{page_no}/{offset}`

**Description:** Returns paginated list of all sold players in a room with team and price details

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room_id` (String): The room ID
- `page_no` (Integer): Page number for pagination
- `offset` (Integer): Number of records to skip

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "player_id": 1,
    "player_name": "Virat Kohli",
    "role": "Batsman",
    "team_name": "Mumbai Indians",
    "sold_price": 15.50,
    "participant_id": 123
  },
  {
    "player_id": 2,
    "player_name": "Jasprit Bumrah",
    "role": "Bowler",
    "team_name": "Chennai Super Kings",
    "sold_price": 12.00,
    "participant_id": 124
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching sold players"
}
```

**Pagination:** Returns up to 10 players per request. Use `offset` to paginate through results.

---

### 13. Get Unsold Players

**Route:** `GET /players/get-unsold-players/{room_id}/{page_no}/{offset}`

**Description:** Returns paginated list of all unsold players in a room

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room_id` (String): The room ID
- `page_no` (Integer): Page number for pagination
- `offset` (Integer): Number of records to skip

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "player_id": 50,
    "player_name": "Player Name",
    "role": "All Rounder",
    "base_price": 2.00
  },
  {
    "player_id": 51,
    "player_name": "Another Player",
    "role": "Batsman",
    "base_price": 1.50
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching unsold players"
}
```

**Pagination:** Returns up to 10 players per request. Use `offset` to paginate through results.

---

### 14. Get Players from Pool

**Route:** `GET /players/get-pool/{pool_id}`

**Description:** Returns all players from a specific pool

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `pool_id` (Integer): The pool number (i16)

**Success Response:**
```
Status Code: 200 OK
Body:
[
  {
    "id": 1,
    "name": "Virat Kohli",
    "role": "Batsman",
    "country": "India",
    "base_price": 2.0,
    "pool_no": 1
  },
  {
    "id": 2,
    "name": "Jasprit Bumrah",
    "role": "Bowler",
    "country": "India",
    "base_price": 2.0,
    "pool_no": 1
  }
]
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body:
{
  "message": "server error while fetching players from pool"
}
```

**Note:** This endpoint retrieves player data from Redis cache for fast pool-based player browsing.

---

## Admin Routes

### 15. Get Redis Player

**Route:** `GET /admin/get-redis-player/{player_id}`

**Description:** Retrieves a player's data directly from Redis cache (for debugging/admin purposes)

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `player_id` (Integer): The player ID

**Success Response:**
```
Status Code: 200 OK
Body:
{
  "id": 1,
  "name": "Virat Kohli",
  "role": "Batsman",
  "country": "India",
  "base_price": 2.0,
  "pool_no": 1
}
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body: "Error while fetching player from redis"
```

---

### 16. Execute Auction Cleanup Tasks

**Route:** `POST /admin/auction_completed_tasks_execution`

**Description:** Manually triggers auction completion tasks (moves data to completed tables, cleanup)

**Authentication:** Required (Bearer token)

**Request Body (JSON):**
```json
{
  "room_id": "uuid-string",
  "password": "admin_password"
}
```

**Success Response:**
```
Status Code: 200 OK
Body: "Successfully Executed"
```

**Error Response:**
```
Status Code: 500 Internal Server Error
Body: "Invalid Password"
```

**Note:** Requires `ADMIN_PASSWORD` environment variable to be set. This endpoint is for administrative cleanup of completed auctions.

---

## WebSocket API

### Connection Endpoint

**Route:** `GET /ws/{room_id}/{participant_id}`

**Description:** Establishes a WebSocket connection for real-time auction communication

**Authentication:** Required (Bearer token in Authorization header during initial HTTP handshake)

**Path Parameters:**
- `room_id` (String): The room ID to connect to
- `participant_id` (Integer): The participant ID

**Connection Flow:**
1. Client sends HTTP GET request with Authorization header
2. Server upgrades connection to WebSocket
3. Server validates room status and participant
4. Server sends initial connection confirmation or error
5. Client and server exchange messages in real-time

---

### Client to Server Messages

All client messages are sent as **text** messages.

#### 1. Start Auction

**Message:** `"start"`

**Description:** Initiates the auction. Only the room creator can start the auction.

**Requirements:**
- Minimum 3 participants must be in the room
- Room status must be "not_started"
- Only room creator can execute this command

**Server Response:**
- On success: Broadcasts first player details (JSON) to all participants
- On failure: Sends error message only to the requester

**Error Messages:**
- `"Min of 3 participants should be in the room to start auction"`
- `"Technical Glitch"` (if player cannot be fetched)

---

#### 2. Place Bid

**Message:** `"bid"`

**Description:** Places a bid on the current player being auctioned

**Requirements:**
- Minimum 3 participants must be in the room
- Room status must be "in_progress"
- Participant must have sufficient balance
- Participant must not be the current highest bidder
- Bid must allow participant to complete minimum squad (15 players)

**Bid Increment Logic:**
- If current bid = 0: Next bid = base_price
- If current bid < 1.0: Increment by 0.05
- If current bid < 10.0: Increment by 0.10
- If current bid >= 10.0: Increment by 0.25

**Bid Allowance Validation:**
- **Free Mode:** `(balance - bid_amount) >= (15 - players_bought) × 0.30`
- **Strict Mode:** Segment-based balance requirements with buffer per player

**Server Response:**
- On success: Broadcasts bid details (JSON) to all participants
- On failure: Sends error message only to the requester

**Error Messages:**
- `"You are already the highest bidder"`
- `"Min of 3 participants should be in the room to bid"`
- `"Bid not allowed due to insufficient balance for remaining players"`
- `"Technical Issue"` (if bid processing fails)

---

#### 3. Skip

**Message:** `"skip"`

**Description:** Skip the current player (mark as not interested)

**Requirements:**
- Participant can skip current player
- Tracked per player per participant

**Server Response:**
- Broadcasts skip notification to all participants
- Updates skip count for current player

---

#### 4. RTM (Right to Match)

**Message:** `"rtm"`

**Description:** Use RTM to match the current bid and acquire the player

**Requirements:**
- Participant must have remaining RTMs
- Player must be currently bid on by another team
- Participant must have sufficient balance
- RTM can only be used for specific scenarios (ex-team players)

**Server Response:**
- Initiates RTM timer
- Broadcasts RTM usage notification
- Allows bidding team to counter or cancel

---

#### 5. RTM Accept

**Message:** `"rtm-accept"`

**Description:** Accept the RTM and award player at current bid

**Requirements:**
- RTM timer must be active
- Participant using RTM must have sufficient balance

**Server Response:**
- Awards player to RTM user
- Updates balance and player count
- Moves to next player

---

#### 6. RTM Cancel

**Message:** `"rtm-cancel"`

**Description:** Cancel the RTM without countering

**Requirements:**
- RTM timer must be active

**Server Response:**
- Returns bidding rights to original bidder
- Continues auction for current player

---

#### 7. Instant RTM Cancel

**Message:** `"instant-rtm-cancel"`

**Description:** Immediately cancel RTM without waiting for timer

**Requirements:**
- RTM timer must be active
- Original bidder can instantly cancel

**Server Response:**
- Cancels RTM immediately
- Resumes normal bidding

---

#### 8. End Auction

**Message:** `"end"`

**Description:** Ends the auction. Only the room creator can end the auction.

**Requirements:**
- Only room creator can execute
- All participants must have at least 15 players in their squad (optional validation)

**Server Response:**
- On success: Broadcasts `"exit"` message to all participants (clients should disconnect)
- On failure: Broadcasts error message to all participants

**Error Messages:**
- `"Only Creator can have permission"` (if not room creator)
- `"During RTM You cannot End the Auction"` (if RTM timer active)
- `"Unable to End Auction, Due to Technical Problem"` (if cleanup fails)
- `"Technical Issue"` (if room creator check fails)

---

#### 9. Pause

**Message:** `"pause"`

**Description:** Pause the auction (creator only)

**Requirements:**
- Only room creator can execute

**Server Response:**
- `"After the Current Bid Auction will be Paused"` to creator
- Pauses after current bid timer expires

---

#### 10. Skip Current Pool

**Message:** `"skip-current-pool"`

**Description:** Vote to skip the entire current pool

**Requirements:**
- All participants must vote to skip
- Tracked per participant per pool

**Server Response:**
- Broadcasts skip pool vote notification
- If all vote, jumps to next pool

---

#### 11. Get Is Skipped Pool

**Message:** `"get-is-skipped-pool"`

**Description:** Check if current user has voted to skip current pool

**Server Response:**
- `"is_skipped:true"` or `"is_skipped:false"`

---

#### 12. Mute/Unmute

**Message:** `"mute"` or `"unmute"`

**Description:** Toggle audio state for WebRTC

**Server Response:**
- Broadcasts mute/unmute state to all participants
- Updates participant audio state in Redis

---

#### 13. Ping

**Message:** `"ping"`

**Description:** Heartbeat/keepalive message

**Server Response:**
- Sends Pong frame

---

#### 14. Chat

**Message:** `"chat-{message}"`

**Description:** Send chat message to all participants

**Format:** `"chat-Hello everyone"`

**Server Response:**
- Broadcasts chat message with team name to all participants

---

#### 15. WebRTC Signaling

**Messages:** JSON formatted WebRTC signaling messages

**Types:**
- Offer: `{"from": participant_id, "to": participant_id, "payload": sdp_offer}`
- Answer: `{"from": participant_id, "to": participant_id, "payload": sdp_answer}`
- ICE Candidate: `{"from": participant_id, "to": participant_id, "payload": ice_candidate}`

**Server Response:**
- Forwards signaling message to target participant

---

### Server to Client Messages

All server messages are sent as **text** messages (JSON strings for structured data).

#### 1. New Participant Joined

**Message Type:** JSON

**Description:** Sent to all participants when a new team joins the room

**Message Format:**
```json
{
  "participant_id": 123,
  "team_name": "Mumbai Indians",
  "balance": 100.00,
  "total_players_brought": 0,
  "remaining_rtms": 3,
  "is_unmuted": true,
  "foreign_players_brought": 0
}
```

**When Sent:**
- When a new participant joins a room with status "not_started"
- Broadcasted to all existing participants

---

#### 2. Participant Reconnected

**Message Type:** JSON

**Description:** Sent when a participant reconnects to an existing room

**Message Format:**
```json
{
  "id": 123,
  "team_name": "Mumbai Indians",
  "balance": 85.50,
  "total_players_brought": 5,
  "remaining_rtms": 2,
  "is_unmuted": true,
  "foreign_players_brought": 1
}
```

**When Sent:**
- When a participant reconnects to a room they were already part of
- Broadcasted to all participants

---

#### 3. All Participants List

**Message Type:** JSON Array

**Description:** Sent to newly connected participant with list of all active participants

**Message Format:**
```json
[
  {
    "id": 123,
    "team_name": "Mumbai Indians",
    "balance": 85.50,
    "total_players_brought": 5,
    "remaining_rtms": 2,
    "is_unmuted": true,
    "foreign_players_brought": 1
  },
  {
    "id": 124,
    "team_name": "Chennai Super Kings",
    "balance": 90.00,
    "total_players_brought": 3,
    "remaining_rtms": 3,
    "is_unmuted": false,
    "foreign_players_brought": 0
  }
]
```

**When Sent:**
- Immediately after successful WebSocket connection
- Only sent to the connecting participant

---

#### 4. Room Mode

**Message Type:** Text

**Message:** `"strict-mode"`

**Description:** Notifies client if room is in strict bidding mode

**When Sent:**
- Immediately after connection if room has strict mode enabled

---

#### 5. Player Up for Auction

**Message Type:** JSON

**Description:** Sent when a new player is up for bidding

**Message Format:**
```json
{
  "id": 1,
  "name": "Virat Kohli",
  "base_price": 2.0,
  "country": "India",
  "role": "Batsman"
}
```

**When Sent:**
- When auction starts (after "start" command)
- After a player is sold/unsold and next player is available
- Broadcasted to all participants

---

#### 6. Bid Placed

**Message Type:** JSON

**Description:** Sent when a participant places a valid bid

**Message Format:**
```json
{
  "team": "Mumbai Indians",
  "bid_amount": 2.10
}
```

**When Sent:**
- Immediately after a valid bid is placed
- Broadcasted to all participants
- Timer resets when bid is placed

---

#### 7. Player Sold

**Message Type:** JSON

**Description:** Sent when a player is sold to a team after bid timer expires

**Message Format:**
```json
{
  "team_name": "Mumbai Indians",
  "sold_price": 2.10,
  "remaining_balance": 97.90
}
```

**When Sent:**
- After bid timer expires and player has a valid bid
- Broadcasted to all participants
- Followed by next player message or "Auction Completed"

---

#### 8. Player Unsold

**Message Type:** Text

**Message:** `"UnSold"`

**Description:** Sent when a player goes unsold (no valid bids or timer expired with no bids)

**When Sent:**
- After bid timer expires with no valid bids (bid_amount = 0.0)
- Broadcasted to all participants
- Followed by next player message or "Auction Completed"

---

#### 9. Auction Completed

**Message Type:** Text

**Message:** `"Auction Completed"`

**Description:** Sent when all players have been auctioned

**When Sent:**
- When there are no more players to auction
- Broadcasted to all participants

---

#### 10. Exit Signal

**Message Type:** Text

**Message:** `"exit"`

**Description:** Sent when auction ends successfully via "end" command

**When Sent:**
- After "end" command is executed and all validations pass
- Broadcasted to all participants
- **Client Action:** Client should disconnect WebSocket connection upon receiving this message

---

#### 11. Participant Disconnected

**Message Type:** JSON

**Description:** Sent when a participant disconnects

**Message Format:**
```json
{
  "participant_id": 123,
  "team_name": "Mumbai Indians"
}
```

**When Sent:**
- When a participant closes WebSocket connection
- Broadcasted to remaining participants

---

#### 12. Participant Audio State

**Message Type:** JSON

**Description:** Sent when a participant mutes/unmutes

**Message Format:**
```json
{
  "participant_id": 123,
  "is_unmuted": false
}
```

**When Sent:**
- After mute/unmute command
- Broadcasted to all participants

---

#### 13. Chat Message

**Message Type:** JSON

**Description:** Chat message from a participant

**Message Format:**
```json
{
  "team_name": "Mumbai Indians",
  "message": "Hello everyone"
}
```

**When Sent:**
- After a participant sends a chat message
- Broadcasted to all participants

---

#### 14. Auction Stopped Temporarily

**Message Type:** Text

**Message:** `"Auction Stopped Temporarily, Due to no min players"`

**Description:** Sent when auction is paused due to insufficient participants

**When Sent:**
- When bid timer expires but room has less than 3 participants
- Broadcasted to all participants
- Room creator can restart auction with "start" command

---

#### 15. Error Messages

**Connection Errors:**

- `"Server Side Error, Unable to create connection"` - Server error during connection setup
- `"Server Side Error, Unable to get room-mode"` - Failed to retrieve room mode
- `"Your not in the room"` - Participant not found in room
- `"Auction Started Room was close"` - Attempting to join after auction started
- `"Auction was completed, Room was Closed"` - Attempting to connect to completed auction

**General Errors:**

- `"Invalid Message"` - Unknown or malformed message from client
- `"Technical Issue"` - Internal server error
- `"Technical Glitch"` - Player fetch error
- `"Error Occurred while getting players from redis"` - Redis error when fetching next player
- `"Unable to parse what you have sent"` - Invalid JSON in WebRTC signaling

**Permission Errors:**

- `"Only Creator can have permission"` - Non-creator trying to use creator-only command
- `"During RTM You cannot End the Auction"` - Trying to end during RTM timer

**All error messages are sent only to the requester (not broadcasted), except:**
- Auction end validation errors (broadcasted to all)
- Auction stopped temporarily (broadcasted to all)

---

### WebSocket Message Flow Example

1. **Client connects** → Server validates and sends participant details or error
2. **Server sends all participants list** → Client displays current room members
3. **New participant joins** → Server broadcasts `NewJoiner` JSON to all
4. **Creator sends "start"** → Server broadcasts first `Player` JSON to all
5. **Participant sends "bid"** → Server broadcasts `BidOutput` JSON to all, resets timer
6. **Timer expires with bid** → Server broadcasts `SoldPlayer` JSON, then next `Player` JSON
7. **Timer expires without bid** → Server broadcasts `"UnSold"`, then next `Player` JSON
8. **Creator sends "end"** → Server validates, broadcasts `"exit"` to all
9. **Clients disconnect** → Connection closed

---

### Bid Timer Mechanism

- Timer duration: Configurable via `BID_EXPIRY` environment variable (default: 30 seconds)
- Timer key: `auction:timer:{room_id}`
- Timer resets: Every time a new bid is placed
- Timer expiry: Triggers player sale/unsold logic and moves to next player
- Implementation: Redis keyspace expiry events listened by dedicated background task

---

## Project Motto

**"Bringing the Thrill of IPL Auctions to Your Fingertips"**

This IPL Auction System is designed to recreate the excitement and strategic gameplay of the Indian Premier League player auctions in a real-time, interactive digital environment. The platform enables cricket enthusiasts to:

- **Experience Real-Time Bidding:** Participate in live auctions with instant updates and competitive bidding
- **Build Your Dream Team:** Strategically select players while managing budgets and squad requirements
- **Compete with Friends:** Create private auction rooms and compete with multiple participants
- **Make Strategic Decisions:** Balance between star players and budget constraints to build a winning squad

The system emphasizes:
- **Fair Play:** Automated bid validation ensures all teams can complete their squads
- **Real-Time Experience:** WebSocket-based architecture provides instant updates and seamless interaction
- **Strategic Depth:** Budget management and minimum squad requirements add tactical elements
- **Reliability:** Redis for fast state management and PostgreSQL for persistent data storage
- **Event-Driven:** Redis pub/sub for timer-based auction mechanics

Whether you're a cricket fanatic, a fantasy sports enthusiast, or someone who enjoys strategic gameplay, this platform brings the auction house experience directly to you, making every bid count and every decision matter.

---

## Notes

- All monetary values are in **Crores (Cr)** of Indian Rupees
- Minimum squad size: **15 players** per team
- Starting balance: **100 Cr** per team
- Minimum participants required: **3 teams** to start an auction
- Maximum participants: **10 teams** (one for each IPL franchise)
- Bid increments are automatically calculated based on current bid amount
- All timestamps are in UTC format
- Room statuses: `"not_started"`, `"in_progress"`, `"completed"`
- Room modes: Normal (default) and Strict Mode (with enhanced balance constraints)

---

**Document Version:** 2.0  
**Last Updated:** December 30, 2025

