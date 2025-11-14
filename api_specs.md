# IPL Auction System - API Specifications

## Table of Contents
1. [HTTP API Routes](#http-api-routes)
2. [WebSocket API](#websocket-api)
3. [Project Motto](#project-motto)

---

## HTTP API Routes

All routes except `/` and `/continue-with-google` require authentication via `Authorization` header with Bearer token.

### Base URL
```
http://localhost:4545
```

---

### 1. Root Endpoint

**Route:** `GET /`

**Description:** Simple health check endpoint

**Authentication:** Not required

**Response:**
```
Status Code: 200 OK
Body: "Hello, World!"
```

---

### 2. Google Authentication

**Route:** `POST /continue-with-google`

**Description:** Authenticates user via Google OAuth and returns JWT token in Authorization header

**Authentication:** Not required

**Request Body (Form Data):**
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

### 3. Create Room

**Route:** `GET /rooms/create-room/{team_name}`

**Description:** Creates a new auction room and adds the creator as the first participant

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `team_name` (String): Name of the team selected by the room creator

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

### 4. Get Remaining Teams

**Route:** `GET /rooms/join-room-get-teams/{room-id}`

**Description:** Returns remaining available teams for a room. If user is already a participant, returns their participant details instead.

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room-id` (String): The room ID to join

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

### 5. Join Room

**Route:** `GET /rooms/join-room/{room-id}/{team_name}`

**Description:** Adds a participant to an existing room with the selected team

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `room-id` (String): The room ID to join
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

### 6. Get Auctions Played

**Route:** `GET /rooms/get-auctions-played`

**Description:** Returns list of all auction rooms participated by the authenticated user

**Authentication:** Required (Bearer token)

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

---

### 7. Get Participants in Room

**Route:** `GET /rooms/get-participants/{room_id}`

**Note:** The route definition in code shows `/get-participants` without path parameter, but the controller expects `room_id` as a path parameter. The route should be `/get-participants/{room_id}` to match the controller implementation.

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

### 8. Get Team Details

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

### 9. Get Team Players

**Route:** `GET /players/get-team-players/{participant_id}`

**Description:** Returns list of all players bought by a team with their details

**Authentication:** Required (Bearer token)

**Path Parameters:**
- `participant_id` (Integer): The participant ID

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
- System ensures: `(balance - bid_amount) >= (15 - players_bought) × 0.30`
- This guarantees teams can complete their minimum squad

**Server Response:**
- On success: Broadcasts bid details (JSON) to all participants
- On failure: Sends error message only to the requester

**Error Messages:**
- `"You are already the highest bidder"`
- `"Min of 3 participants should be in the room to bid"`
- `"Technical Issue"` (if bid processing fails)

---

#### 3. End Auction

**Message:** `"end"`

**Description:** Ends the auction. Only the room creator can end the auction.

**Requirements:**
- Only room creator can execute
- All participants must have at least 15 players in their squad

**Server Response:**
- On success: Broadcasts `"exit"` message to all participants (clients should disconnect)
- On failure: Broadcasts error message to all participants

**Error Messages:**
- `"You will not having permissions"` (if not room creator)
- `"Not enough players brought by each team"` (if validation fails)
- `"Till all participants brought at least 15 player"` (if room cannot be fetched)
- `"Technical Issue"` (if room creator check fails)

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
  "balance": 100.00
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
  "total_players_brought": 5
}
```

**When Sent:**
- When a participant reconnects to a room they were already part of
- Broadcasted to all participants

---

#### 3. Player Up for Auction

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

#### 4. Bid Placed

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

#### 5. Player Sold

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

#### 6. Player Unsold

**Message Type:** Text

**Message:** `"UnSold"`

**Description:** Sent when a player goes unsold (no valid bids or timer expired with no bids)

**When Sent:**
- After bid timer expires with no valid bids (bid_amount = 0.0)
- Broadcasted to all participants
- Followed by next player message or "Auction Completed"

---

#### 7. Auction Completed

**Message Type:** Text

**Message:** `"Auction Completed"`

**Description:** Sent when all players have been auctioned

**When Sent:**
- When there are no more players to auction
- Broadcasted to all participants

---

#### 8. Exit Signal

**Message Type:** Text

**Message:** `"exit"`

**Description:** Sent when auction ends successfully via "end" command

**When Sent:**
- After "end" command is executed and all validations pass
- Broadcasted to all participants
- **Client Action:** Client should disconnect WebSocket connection upon receiving this message

---

#### 9. Auction Stopped Temporarily

**Message Type:** Text

**Message:** `"Auction Stopped Temporarily, Due to no min players"`

**Description:** Sent when auction is paused due to insufficient participants

**When Sent:**
- When bid timer expires but room has less than 3 participants
- Broadcasted to all participants
- Room creator can restart auction with "start" command

---

#### 10. Error Messages

**Connection Errors:**

- `"Server Side Error, Unable to create connection"` - Server error during connection setup
- `"Your not in the room"` - Participant not found in room
- `"Auction was completed, Room was Closed"` - Attempting to connect to completed auction

**General Errors:**

- `"Invalid Message"` - Unknown or malformed message from client
- `"Technical Issue"` - Internal server error
- `"Technical Glitch"` - Player fetch error
- `"Error Occurred while getting players from redis"` - Redis error when fetching next player

**All error messages are sent only to the requester (not broadcasted), except:**
- Auction end validation errors (broadcasted to all)
- Auction stopped temporarily (broadcasted to all)

---

### WebSocket Message Flow Example

1. **Client connects** → Server validates and sends participant details or error
2. **New participant joins** → Server broadcasts `NewJoiner` JSON to all
3. **Creator sends "start"** → Server broadcasts first `Player` JSON to all
4. **Participant sends "bid"** → Server broadcasts `BidOutput` JSON to all, resets timer
5. **Timer expires with bid** → Server broadcasts `SoldPlayer` JSON, then next `Player` JSON
6. **Timer expires without bid** → Server broadcasts `"UnSold"`, then next `Player` JSON
7. **Creator sends "end"** → Server validates, broadcasts `"exit"` to all
8. **Clients disconnect** → Connection closed

---

### Bid Timer Mechanism

- Timer duration: Configurable via `BID_EXPIRY` environment variable (default: 30 seconds)
- Timer key: `auction:timer:{room_id}`
- Timer resets: Every time a new bid is placed
- Timer expiry: Triggers player sale/unsold logic and moves to next player

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

---

**Document Version:** 1.0  
**Last Updated:** January 2025

