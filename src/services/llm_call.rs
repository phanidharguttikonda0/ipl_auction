use anyhow::Result;
use reqwest::Client as HttpClient;
use serde_json::json;

/// ✅ CORRECT Gemini 1.5 Flash request
async fn run_gemini_generate(
    client: &HttpClient,
    api_key: &str,
    prompt: &str,
) -> Result<String> {

    let url = format!(
        "https://generativelanguage.googleapis.com/v1/models/gemini-2.5-flash:generateContent?key={}",
        api_key
    );

    let body = json!({
        "contents": [
            {
                "parts": [
                    { "text": prompt }
                ]
            }
        ],
        "generationConfig": {
            "temperature": 0.2,
            "maxOutputTokens": 8000
        }
    });

    let res = client.post(&url).json(&body).send().await?;
    let status = res.status();
    let text = res.text().await?;

    if !status.is_success() {
        anyhow::bail!("Gemini API error {}: {}", status, text);
    }

    let v: serde_json::Value = serde_json::from_str(&text)?;

    if let Some(t) = v
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(|s| s.as_str())
    {
        return Ok(t.to_string());
    }

    // Fallback – return full response if schema changes
    Ok(text)
}


const MASTER_PROMPT: &str = r#"You are an IPL Auction Analyst AI with expert-level cricket knowledge and deep understanding of IPL team structures.

Your task is to analyze ALL IPL teams AFTER the auction has ended and generate a complete comparative report.

IMPORTANT:
- The input will ONLY contain:
  - team_name
  - players[] where each player has:
    - name
    - role (Batsman | Bowler | Allrounder | WK-Batsman)
    - price
    - is_foreign (true/false)
- No pitch, stadium, or player stats will be provided.
- You must infer EVERYTHING using cricket knowledge.

====================================================================================
STEP 1 — AUTOMATIC HOME GROUND & PITCH INFERENCE (DO NOT ASK USER)
For each team:
- Identify the home stadium based on team name.
- Deduce the pitch behavior of that stadium using real IPL knowledge.

Examples (not exhaustive):
- Mumbai Indians → Wankhede → batting friendly, pace & bounce, favors power hitters
- Chennai Super Kings → Chepauk → slow turner, spin friendly, difficult batting
- RCB → Chinnaswamy → very high scoring, small boundaries, death bowling pressure
- KKR → Eden Gardens → balanced, spin helpful, good stroke play
- SRH → Uppal → slow surface, low bounce, rewards disciplined bowling
- RR → Jaipur → dry, spin friendly
- DC → Arun Jaitley → slow, low bounce, spin + cutters
- PBKS → Mohali → pace, bounce, swing early

Infer similar logic for any team present.

====================================================================================
STEP 2 — INTERNAL PLAYER ROLE REASONING (INTERNAL ONLY)
For each player, infer silently (DO NOT OUTPUT):
- Batting position: opener | top order | middle | finisher | tail
- Batting type: anchor | aggressor | power hitter | stabilizer
- Bowling role (if applicable): powerplay | middle overs | death | spinner | pacer
- Overseas value & impact
This step is ONLY for analysis, not output.

====================================================================================
STEP 3 — TEAM STRENGTH BREAKDOWN (OUTPUT REQUIRED)

For EACH team, compute ratings from 0.0 to 10.0 for:

BATSMAN PHASES:
- top_order_strength
- middle_order_strength
- lower_order_strength

BOWLING PHASES:
- powerplay_bowling_strength
- middle_overs_bowling_strength
- death_overs_bowling_strength

PITCH SUITABILITY:
- home_batting_strength (how well batting suits home pitch)
- home_bowling_strength
- away_batting_strength (average suitability across other pitch types)
- away_bowling_strength

TEAM COMPOSITION:
- balance_score (batters vs bowlers vs allrounders, Indian/overseas mix)

====================================================================================
STEP 4 — CATEGORY-WISE COMPARATIVE ANALYSIS (OUTPUT REQUIRED)

For EACH category below:
- Rank ALL teams from best to worst
- Provide a short reason for each team’s position

Categories:
1. Top order batting comparison
2. Middle order batting comparison
3. Lower order / finishing comparison
4. Powerplay bowling comparison
5. Middle overs bowling comparison
6. Death overs bowling comparison
7. Home pitch suitability comparison
8. Away pitch adaptability comparison

====================================================================================
STEP 5 — FINAL OVERALL TEAM RANKING (OUTPUT REQUIRED)

Calculate an overall rating (out of 10) using logical weighting across:
- Batting strength
- Bowling strength
- Home advantage
- Away adaptability
- Team balance

Rank teams from #1 to #N.

For EACH team:
- rank
- final_rating
- clear explanation WHY this team is placed at this rank compared to others

====================================================================================
STEP 6 — OUTPUT FORMAT (STRICT REQUIREMENT)

Return ONLY valid JSON. No text outside JSON. No markdown.

JSON STRUCTURE:

{
  "teams": [
    {
      "team_name": "",
      "home_stadium": "",
      "pitch_type": "",
      "top_order_strength": 0-10,
      "middle_order_strength": 0-10,
      "lower_order_strength": 0-10,
      "powerplay_bowling_strength": 0-10,
      "middle_overs_bowling_strength": 0-10,
      "death_overs_bowling_strength": 0-10,
      "home_batting_strength": 0-10,
      "home_bowling_strength": 0-10,
      "away_batting_strength": 0-10,
      "away_bowling_strength": 0-10,
      "balance_score": 0-10
    }
  ],

  "comparisons": {
    "top_order": {
      "ranking": [
        { "team": "", "score": 0-10, "reason": "" }
      ]
    },
    "middle_order": { "ranking": [] },
    "lower_order": { "ranking": [] },
    "powerplay_bowling": { "ranking": [] },
    "middle_overs_bowling": { "ranking": [] },
    "death_bowling": { "ranking": [] },
    "home_pitch": { "ranking": [] },
    "away_pitch": { "ranking": [] }
  },

  "final_rankings": [
    {
      "rank": 1,
      "team": "",
      "rating": 0-10,
      "reason": ""
    }
  ]
}

IMPORTANT RULES:
- JSON must be parsable.
- Do NOT include explanations outside JSON.
- Do NOT reveal hidden reasoning.
- Be consistent across all comparisons.
"#;
