# IPL Auction System

A real-time IPL (Indian Premier League) auction platform built with Rust, featuring WebSocket-based bidding, Redis for state management, and PostgreSQL for persistent storage.

## Features

- **Real-time Auction**: WebSocket-based bidding system for instant updates
- **Multi-Room Support**: Multiple auction rooms can run simultaneously
- **Smart Bid Validation**: Automatic validation ensuring teams maintain minimum squad requirements
- **Team Management**: Each participant selects and manages their own team
- **Budget Control**: Built-in balance tracking and spending limits
- **Redis State Management**: Fast, in-memory state management for active auctions
- **PostgreSQL Persistence**: Reliable storage for auction data and player information

## Tech Stack

- **Framework**: Axum 0.8.6 (async web framework)
- **Database**: PostgreSQL (via SQLx 0.8.6)
- **Cache/State**: Redis 0.32.7
- **Async Runtime**: Tokio 1.48.0
- **Serialization**: Serde 1.0.228 & Serde JSON 1.0.145
- **Logging**: Tracing 0.1.41 & Tracing-subscriber 0.3.20
- **Authentication**: JsonWebToken 10.2.0
- **Configuration**: dotenv 0.15.0

## Prerequisites

- Rust 1.91.0 or higher
- PostgreSQL database
- Redis server

## Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd ipl_auction
   ```

2. **Configure environment variables**

   Create a `.env` file in the project root:
   ```env
   DATABASE_URL=postgresql://username:password@localhost/ipl_auction
   REDIS_URL=redis://localhost:6379
   BID_EXPIRY=30
   ```

3. **Run database migrations**
   ```bash
   sqlx migrate run
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run
   ```

## How It Works

### Auction Flow

1. **Room Creation**: A user creates an auction room
2. **Team Selection**: Participants join the room and select their teams
3. **Auction Start**: Room creator initiates the auction
4. **Player Bidding**: Players are presented one by one, participants bid in real-time
5. **Bid Validation**: System validates bids based on:
    - Available balance
    - Minimum squad requirements (15 players)
    - Reserved funds for remaining slots
6. **Auction End**: Ends when all participants have minimum 15 players

### WebSocket Messages

- `start` - Begin the auction (creator only)
- `bid` - Place a bid on the current player
- `end` - End the auction (creator only, with validation)

### Bid Allowance Logic

The system ensures teams can complete their squad by reserving funds:
- Each team must acquire at least 15 players
- Minimum reserve: `(15 - players_bought) × 0.30` lakhs
- Bids are rejected if they would leave insufficient funds

## Project Structure

## API Endpoints

### WebSocket Connection
WS /auction/{room_id}/{participant_id}

## Contributing
### Contributions are welcome! Please feel free to submit a Pull Request.

This README provides a comprehensive overview of your IPL auction system, including its features, technology stack, setup instructions, and how the auction flow works. You can customize it further by adding specific API endpoint documentation, screenshots, or additional sections as needed.

## License

Copyright © 2025 Guttikonda Phanidhar Reddy. All Rights Reserved.

This software is proprietary. Unauthorized copying, modification, distribution, 
or use of this software is strictly prohibited without explicit written permission 
from the author.

For licensing or usage permissions, please contact: [phanidharguttikonda0@gmail.com]