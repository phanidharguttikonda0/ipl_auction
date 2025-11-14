# IPL Auction Front-End Specification

## Overview
This document translates the latest product notes into concrete UI/UX requirements for the React + Tailwind implementation. It assumes the backend described in `api_specs.md` and focuses on the screens, states, flows, and styling guidance needed to scaffold the front end (web-first, responsive down to tablet and mobile).

---

## 1. Authentication & Onboarding Flow
- **Entry point:** `POST /continue-with-google`.
- **Step 1 – Google Continue:** render a CTA “Continue with Google”. Once the user completes OAuth and we POST the returned payload:
  - If the backend responds with an `Authorization` header, store `Bearer <token>` in `localStorage` and route to the Home experience.
  - If the response lacks an `Authorization` header, the backend expects `favorite_team`. Keep the modal/page open and show a **team picker**.
- **Step 2 – Favorite Team Choice (only when needed):**
  - Display all 10 IPL teams in a selectable grid/dropdown (strings listed in `api_specs.md`).
  - Once the user picks a team and re-submits the same `/continue-with-google` request with `favorite_team`, capture the `Authorization` header, store it, and proceed.
- **State management:** show loading states during POST, show error toast if auth fails.
- **Persistence:** the stored token should be attached to all subsequent API calls and WebSocket handshakes.

---

## 2. Home Experience (Desktop-First Layout)

### 2.1 High-Level Layout
```
┌───────────────┬───────────────────────────────┐
│ Left Panel     │ Center Content                │
│ (Rules +       │ (User details + Auctions)     │
│ Upcoming feats)│                               │
├───────────────┼───────────────────────────────┤
│ Bottom/Right Utility Panel (Join/Create Room)  │
└───────────────────────────────────────────────┘
```
- **Laptop breakpoint:** three-column feel.
- **Tablet/Mobile:** collapse into sections stacked vertically; ensure cards scale gracefully.

### 2.2 Rules & Upcoming Features (Left Panel)
- Present a card titled “House Rules” with summarized bullet list (min 3 participants, 15-player squads, budget 100 Cr, etc.).
- Below it, list **Upcoming Features** exactly as provided, each prefixed with the arrow icon or “•”. Text to include verbatim:
  1. Introducing RTM soon.
  2. no retained players . in future implementing retained players.
  3. custom bid time.
  4. if every one clicked not interested, then we are going to make it unsold immediately.
  5. Managing the players into different pools.
  6. After all team brought 14 players, then they  can choose interested players from remaining , so those players will get.
  7. get unsold players list and stop current list
  8. WebRTC implementation in future
  9. loading same profile picture from the mail-id
  10. foreign player buying limits
  11. pausing the auction.
- Style as scrollable list if needed.

### 2.3 User Snapshot (Center – Top)
- Card showing:
  - Username
  - Gmail
  - Favorite team
  - Optional profile avatar (future: “loading same profile picture from the mail-id”).
- Include CTA to log out (clears localStorage token).

### 2.4 Auctions Overview (Center – Bottom)
- **API:** `GET /rooms/get-auctions-played`.
- Render a list/table of past rooms with `room_id` and `created_at`.
- Each row clickable:
  - On click, call `GET /rooms/get-participants/{room_id}`.
  - Show drawer/modal listing participants + team names.
  - Within that drawer, clicking a team name triggers:
    - `GET /players/get-team-details/{participant_id}`
    - `GET /players/get-team-players/{participant_id}`
  - Display stats (balance, counts) and the roster list.

### 2.5 Right Utility Panel
- **Join Room**
  - Text input for Room ID.
  - Button “Join Room”.
  - Flow:
    1. Call `GET /rooms/join-room-get-teams/{room-id}`.
    2. If response says “Already a participant”, jump to WebSocket connect page using returned participant_id.
    3. If “Join with the remaining teams”, show modal listing the `remaining_teams`; user picks team and submit `GET /rooms/join-room/{room-id}/{team_name}`.
    4. After success, open the auction room view and establish WS connection `/ws/{room_id}/{participant_id}`.
- **Create Room**
  - Button “Create Room”.
  - On click, open modal listing valid team names; once selected, hit `GET /rooms/create-room/{team_name}`.
  - On success, navigate to auction room with given `room_id` and `participant_id`, then connect via WebSocket.
- Provide inline states: loading, validation errors (invalid team, room closed, etc.) using responses from APIs.

---

## 3. Auction Room Screen

### 3.1 Layout (Desktop)
```
┌───────────────────────────────────────────────┐
│ Header: Room info + connection status + timer │
├───────────────┬───────────────────────────────┤
│ Left Sidebar  │ Main Stage                    │
│ (Team picker, │ - Center: Current player card │
│ stats cards)  │ - Right top: Last bid message │
│               │ - Right mid: Action buttons   │
│               │ - Right bottom: Activity feed │
└───────────────┴───────────────────────────────┘
```

### 3.2 Required Elements
- **Header:**
  - Room ID, participant/team name.
  - Connection indicator (WebSocket connected/disconnected).
  - Countdown timer reflecting `BID_EXPIRY`.
  - If current user is creator (via `is_room_creator` logic server side), show “Start Auction” button.
- **Current Player Card (center):**
  - Player name, role, country, base price.
  - Placeholder when auction not started yet.
- **Last Bid Panel (top-right):**
  - Show latest bid message (team + amount) broadcasted via WebSocket.
  - Update in real time using message type documented in `api_specs.md`.
- **Actions (right-mid):**
  - Buttons: “Bid”, “Not Interested”, “End Auction” (creator only). Buttons reflect availability rules (e.g., disabled when <3 participants).
  - Hook up to WS messages: sending `"start"`, `"bid"`, `"end"`, etc.
- **Activity Feed (right-bottom):**
  - Scrollable list of incoming WS messages (player sold, unsold, new joiners, warnings).
- **Left Sidebar:**
  - **Balance & Squad Card:** show remaining balance, players bought, role counts (from `/players/get-team-details/{participant_id}`).
  - **Team Dropdown:** list of all participants (obtained from `/rooms/get-participants/{room_id}`) with ability to view their stats + roster (same API pair as above).
  - **Team Roster Panel:** when a team is selected, show table of players (`GET /players/get-team-players/{participant_id}`).
  - **Upcoming player queue** (optional future) – placeholder to integrate “Managing players into different pools”.

### 3.3 WebSocket Interactions
- Connect using stored token in headers during handshake.
- Handle server messages:
  - JSON player payloads → update current player.
  - Bid outputs → update last bid panel + feed.
  - Sold/Unsold events → update feed, refresh team stats (hit REST APIs to keep in sync).
  - `"exit"` → show modal “Auction ended”; auto disconnect and route to summary.
  - Error strings (e.g., “Invalid Message”) → toast notifications.
- Auto attempt reconnect if connection drops.

### 3.4 Owner Controls
- Start button only visible if `is_room_creator` returns true (call dedicated API or infer from create-room response).
- End button gated the same way; show confirmation modal referencing validations (all teams >=15 players).

---

## 4. Styling & Branding Guidelines
- **Palette:** dominant dark reddish (#5c1f1f – adjust as needed) + black/charcoal backgrounds (#0f0f0f to #181818). Use gradients for hero panels.
- **Buttons:** white background, black text, subtle drop shadow, rounded corners (Tailwind `rounded-lg`). On hover invert colors or add border.
- **Typography:** Use a clean geometric font (e.g., Inter). Headlines uppercase with tracking; body text comfortable line-height.
- **Cards:** semi-transparent dark backgrounds with subtle borders (`border border-white/10`) and glassmorphism blur for elegance.
- **Icons:** Use minimal line icons (Heroicons) to keep interface clean.
- **Animations:** Soft transitions on hover, entry transitions for modals (Tailwind `transition-all`, `duration-300`).
- **Responsiveness:** 
  - Breakpoints: `lg` (desktop), `md` (tablet), `sm` (mobile).
  - Collapse side panels into accordions on mobile.
  - Ensure large tables become stacked cards.

---

## 5. Responsiveness Strategy
- **Desktop (≥1024px):** full layout described above.
- **Tablet (768–1023px):** 
  - Stack left panel above center content.
  - Auction room: convert to two columns (stats + stage).
- **Mobile (<768px):**
  - Use vertical sections with collapsible accordions for rules, upcoming features, team stats.
  - Action buttons fixed at bottom for easy reach.
  - Modal overlays full screen.
- **Testing:** plan to use Tailwind’s responsive utilities (`lg:flex`, `md:grid`, etc.).

---

## 6. API References Within UI
| Feature | API(s) |
| --- | --- |
| Auth flow | `POST /continue-with-google` |
| Room creation | `GET /rooms/create-room/{team_name}` |
| Room joining | `GET /rooms/join-room-get-teams/{room_id}` → `GET /rooms/join-room/{room_id}/{team_name}` |
| Auction history | `GET /rooms/get-auctions-played` |
| Participants list | `GET /rooms/get-participants/{room_id}` |
| Team stats | `GET /players/get-team-details/{participant_id}` |
| Team roster | `GET /players/get-team-players/{participant_id}` |
| WebSocket | `GET /ws/{room_id}/{participant_id}` (messages per `api_specs.md`) |

- Ensure every API request includes the stored Bearer token.
- On receiving WebSocket events that change balances/rosters, re-fetch the relevant REST data to keep UI consistent.

---

## 7. Open Questions / Assumptions
1. **Favorite team persistence:** assume backend returns favorite team after second auth; otherwise fetch via profile endpoint if available.
2. **Not Interested button:** currently not in API; treat as future placeholder.
3. **Unsold handling:** immediate unsold when everyone clicks “not interested” will require additional WS/REST support—mark button as disabled until backend added.
4. **Profile photo:** waiting on backend endpoint to fetch Gmail-based avatars.

Document updates should capture answers as they become available.

---

## 8. Next Steps for Front-End Generation
- Feed `api_specs.md` + this `frontend_spec.md` into AI tooling (e.g., Vercel AI) section by section.
- Start with Authentication flow, then Home layout, finally Auction Room.
- After code generation, manually wire WebSocket handlers and test against backend.
- Use this spec as acceptance criteria for QA.

---

**Last Updated:** 2025-11-14

