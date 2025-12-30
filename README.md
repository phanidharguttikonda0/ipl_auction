# IPL Auction System

A real-time IPL (Indian Premier League) auction platform built with Rust, featuring WebSocket-based bidding, Redis event-driven architecture, and PostgreSQL for persistent storage.

## Features

- **Real-time Auction**: WebSocket-based bidding system for instant updates
- **Event-Driven Architecture**: Redis pub/sub for timer-based auction mechanics
- **Multi-Room Support**: Multiple auction rooms can run simultaneously
- **Smart Bid Validation**: Automatic validation ensuring teams maintain minimum squad requirements
- **Team Management**: Each participant selects and manages their own team
- **Budget Control**: Built-in balance tracking and spending limits
- **Background Task Processing**: Non-blocking database operations via Tokio channels
- **Observability**: Structured logging and Prometheus metrics
- **Redis State Management**: Fast, in-memory state management for active auctions
- **PostgreSQL Persistence**: Reliable storage for auction data and player information

## Tech Stack

- **Framework**: Axum 0.8.6 (async web framework)
- **Database**: PostgreSQL (via SQLx 0.8.6)
- **Cache/State**: Redis 0.32.7 with keyspace event notifications
- **Async Runtime**: Tokio 1.48.0
- **Serialization**: Serde 1.0.228 & Serde JSON 1.0.145
- **Logging**: Tracing 0.1.41 & Tracing-subscriber 0.3.20
- **Authentication**: JsonWebToken 10.2.0
- **Metrics**: Prometheus (exposed on port 9898)
- **Configuration**: dotenv 0.15.0

## Prerequisites

- Rust 1.91.0 or higher
- PostgreSQL database
- Redis server (with keyspace notifications enabled)

## Setup

### 1. Clone the repository
```bash
git clone <repository-url>
cd ipl_auction
```

### 2. Configure Redis

Enable keyspace event notifications in your Redis configuration:
```bash
# redis.conf or via redis-cli
CONFIG SET notify-keyspace-events Ex
```

### 3. Configure environment variables

Create a `.env` file in the project root:
```env
# Database
DATABASE_URL=postgresql://username:password@localhost/ipl_auction

# Redis
REDIS_URL=localhost

# Auction Settings
BID_EXPIRY=30

# Optional: Production flag
PROD=false

# Optional: IP Info API
IP_INFO_API_KEY=your_api_key_here
```

### 4. Setup database

Run migrations to create the database schema:
```bash
sqlx migrate run
```

### 5. Build and run

```bash
# Development
cargo run

# Production
cargo build --release
./target/release/ipl_auction
```

The application will start on `http://localhost:4545` (or the port specified in `PORT` env var).

## Project Structure

```
ipl_auction/
├── src/
│   ├── main.rs                          # Application entry point, server setup
│   ├── auction.rs                       # WebSocket handler and message routing
│   ├── controllers/                     # HTTP request handlers
│   │   ├── admin.rs                     # Admin operations
│   │   ├── authentication.rs            # Google OAuth authentication
│   │   ├── player.rs                    # Player data endpoints
│   │   ├── profile.rs                   # User profile management
│   │   ├── rooms.rs                     # Room CRUD operations
│   │   └── others.rs                    # Miscellaneous endpoints (feedback, etc.)
│   ├── services/                        # Business logic layer
│   │   ├── auction.rs                   # Database access layer
│   │   ├── auction_room.rs              # Redis operations, event listener
│   │   ├── auction_logic_executor.rs    # Core auction logic (bid, start, RTM)
│   │   ├── background_db_tasks_runner.rs # Background task processors
│   │   ├── llm_call.rs                  # External API integrations
│   │   └── other.rs                     # Utility functions
│   ├── models/                          # Data structures
│   │   ├── app_state.rs                 # Application state definition
│   │   ├── auction_models.rs            # Auction-related models
│   │   ├── authentication_models.rs     # Auth data structures
│   │   ├── background_db_tasks.rs       # Background task types
│   │   ├── player_models.rs             # Player data models
│   │   ├── room_models.rs               # Room data models
│   │   ├── webRTC_models.rs             # WebRTC signaling models
│   │   └── others.rs                    # Miscellaneous models
│   ├── routes/                          # Route definitions
│   │   ├── admin_routes.rs              # Admin route registration
│   │   ├── players_routes.rs            # Player route registration
│   │   └── rooms_routes.rs              # Room route registration
│   ├── middlewares/                     # HTTP middleware
│   │   └── authentication.rs            # JWT validation middleware
│   └── observability/                   # Monitoring and logging
│       ├── tracing.rs                   # Structured logging setup
│       ├── metrics.rs                   # Prometheus metrics
│       └── http_tracing.rs              # HTTP request tracing
├── migrations/                          # Database migrations (SQLx)
│   ├── 0000001_up.sql
│   ├── 0000002_up.sql
│   └── ipl_auction_schema.sql
├── clean_up_scripts/                    # Utility scripts
├── Cargo.toml                           # Rust dependencies
├── Cargo.lock                           # Locked dependencies
├── Dockerfile                           # Container configuration
├── .env                                 # Environment variables (not in repo)
├── api_specs.md                         # Complete API documentation
├── frontend_spec.md                     # Frontend integration guide
├── DESIGN.md                            # Architecture documentation
└── README.md                            # This file
```

## How It Works

### Auction Flow

1. **Room Creation**: A user creates an auction room and becomes the room creator
2. **Team Selection**: Participants join the room and select their teams (10 IPL teams)
3. **Auction Start**: Room creator initiates the auction (minimum 3 participants required)
4. **Player Bidding**: 
   - Players are presented one by one
   - Participants bid in real-time via WebSocket
   - Bid timer starts/resets on each bid (default: 30 seconds)
   - When timer expires, player is sold to highest bidder or marked unsold
5. **Bid Validation**: System validates bids based on:
   - Available balance
   - Minimum squad requirements (15 players)
   - Reserved funds for remaining slots
   - Strict mode rules (if enabled)
6. **Auction End**: Creator can end auction (validates all teams have 15+ players)

### Key WebSocket Messages

**Client → Server:**
- `start` - Begin the auction (creator only)
- `bid` - Place a bid on the current player
- `skip` - Skip current player (if allowed)
- `rtm` - Use Right to Match
- `end` - End the auction (creator only)
- `pause` - Pause the auction (creator only)

**Server → Client:**
- Player details (JSON) - New player up for auction
- Bid updates (JSON) - Real-time bid information
- Player sold/unsold notifications
- Participant join/reconnect events
- Auction completion signals

For complete WebSocket and HTTP API documentation, see [api_specs.md](api_specs.md).

## Architecture Highlights

### Event-Driven Design

The system uses **Redis keyspace expiry events** to handle bid timers:
- When a bid is placed, a Redis key with TTL is set
- Redis publishes an expiry event when the timer runs out
- A dedicated listener processes expiry events and determines player outcomes
- This ensures timers work even if the server restarts

### Background Task Processing

Database writes are offloaded to background workers:
- WebSocket handlers enqueue tasks to Tokio channels
- Dedicated worker tasks process DB operations asynchronously
- Keeps auction responses fast and non-blocking
- Task types: PlayerSold, BalanceUpdate, RoomStatus, etc.

### Three-Layer State

1. **In-Memory (AppState)**: Active WebSocket connections
2. **Redis**: Current auction state, timers, bids
3. **PostgreSQL**: Persistent storage, historical data

For detailed architecture documentation, see [DESIGN.md](DESIGN.md).

## API Endpoints

### Authentication
- `POST /continue-with-google` - Google OAuth login

### Room Management
- `GET /rooms/create-room/{team_name}` - Create auction room
- `GET /rooms/join-room-get-teams/{room_id}` - Get available teams
- `GET /rooms/join-room/{room_id}/{team_name}` - Join room
- `GET /rooms/get-auctions-played` - List user's auction history
- `GET /rooms/get-participants/{room_id}` - Get room participants

### Player Data
- `GET /players/get-team-details/{participant_id}` - Get team statistics
- `GET /players/get-team-players/{participant_id}` - Get team roster

### WebSocket
- `GET /ws/{room_id}/{participant_id}` - Establish WebSocket connection

For complete API documentation with request/response examples, see [api_specs.md](api_specs.md).

## Observability

### Ports

| Port | Service | Purpose |
|------|---------|---------|
| 4545 | IPL Auction App | Main application server |
| 9898 | Metrics | Prometheus metrics endpoint |
| 9090 | Prometheus | Metrics collection (if running) |
| 3000 | Grafana | Visualization (if running) |

### Logging

Structured logs are written asynchronously in JSON format. Configure log level via environment variables or code.

### Metrics

Prometheus metrics available at `http://localhost:9898/metrics` including:
- Custom bid/auction event counters
- HTTP request metrics
- WebSocket connection counts

## Documentation

- **[DESIGN.md](DESIGN.md)** - Comprehensive architecture documentation including:
  - Event-Driven Architecture details
  - Background task execution patterns
  - WebSocket implementation details
  - Redis data structures
  - State management strategy
  - Complete system diagrams

- **[api_specs.md](api_specs.md)** - Complete HTTP and WebSocket API reference

- **[frontend_spec.md](frontend_spec.md)** - Frontend integration guidelines

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

Copyright © 2025 Guttikonda Phanidhar Reddy. All Rights Reserved.

This software is proprietary. Unauthorized copying, modification, distribution, 
or use of this software is strictly prohibited without explicit written permission 
from the author.

For licensing or usage permissions, please contact: phanidharguttikonda0@gmail.com