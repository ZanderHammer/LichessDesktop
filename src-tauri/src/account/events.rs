//! What the lines of `/api/stream/event` mean to the user (ADR 0025). The field
//! names follow lichess's API spec and lila's `EventStream.scala`.

use std::collections::HashSet;

use serde::Deserialize;
use tauri::Url;

use crate::i18n::{Key, Strings};

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Event {
    Challenge {
        challenge: Challenge,
    },
    GameStart {
        game: Game,
    },
    /// Such as `gameFinish` or a type added later.
    #[serde(other)]
    Other,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Challenge {
    id: String,
    /// Shows the challenge, or its game once it is accepted.
    url: String,
    /// `None` for an anonymous challenger.
    challenger: Option<User>,
    /// `None` for an open challenge.
    dest_user: Option<User>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct User {
    id: String,
    name: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Game {
    game_id: String,
    /// The game ID plus the player's own part: the URL of the player's page.
    full_id: String,
    /// Whether the user has moved in this game.
    #[serde(default)]
    has_moved: bool,
    opponent: Opponent,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct Opponent {
    /// Also set for an AI, such as "Stockfish level 1".
    username: String,
}

/// Something worth telling the user.
#[derive(Debug, PartialEq, Eq)]
pub enum Alert {
    Challenge { from: Option<String>, url: String },
    GameStart { opponent: String, full_id: String },
}

impl Alert {
    pub fn body(&self, strings: &Strings) -> String {
        match self {
            Alert::Challenge { from, .. } => {
                let from = from
                    .as_deref()
                    .unwrap_or_else(|| strings.get(Key::Anonymous));
                strings.format(Key::NotifyChallenge, from)
            }
            Alert::GameStart { opponent, .. } => strings.format(Key::NotifyGameStart, opponent),
        }
    }

    /// The page a click opens.
    pub fn url(&self) -> Option<Url> {
        match self {
            Alert::Challenge { url, .. } => url.parse().ok(),
            Alert::GameStart { full_id, .. } => {
                format!("{}/{full_id}", crate::START_URL).parse().ok()
            }
        }
    }
}

/// Decides which lines of one or more connections are news. Lichess sends all
/// current challenges and games whenever a stream opens, with nothing that
/// marks them as old, so what was seen has to be remembered across reconnects.
#[derive(Debug)]
pub struct Seen {
    /// The ID of a user, which is the lowercase username.
    me: String,
    /// Separate, as an accepted challenge's game has the challenge's ID.
    challenges: HashSet<String>,
    games: HashSet<String>,
    /// Until the first keep-alive, which lila sends 7 seconds after the current
    /// state. What the app first sees of it isn't news: the page shows it too.
    replaying: bool,
}

impl Seen {
    pub fn new(username: &str) -> Self {
        Self {
            me: username.to_lowercase(),
            challenges: HashSet::new(),
            games: HashSet::new(),
            replaying: true,
        }
    }

    /// Takes in one line of the stream, in order. What arrives after a
    /// reconnect and wasn't seen yet happened during the gap, so it is news.
    pub fn line(&mut self, line: &str) -> Option<Alert> {
        if line.trim().is_empty() {
            self.replaying = false;
            return None;
        }
        match serde_json::from_str(line) {
            Ok(event) => self.event(event),
            Err(err) => {
                eprintln!("skipped an unreadable account event: {err}");
                None
            }
        }
    }

    fn event(&mut self, event: Event) -> Option<Alert> {
        match event {
            Event::Challenge { challenge } => {
                // The stream also reports the challenges the user sends.
                let incoming = challenge
                    .dest_user
                    .is_some_and(|dest_user| dest_user.id == self.me);
                let new = incoming && self.challenges.insert(challenge.id);
                (new && !self.replaying).then(|| Alert::Challenge {
                    from: challenge.challenger.map(|user| user.name),
                    url: challenge.url,
                })
            }
            Event::GameStart { game } => {
                let new = self.games.insert(game.game_id);
                // Whoever has moved in a game knows about it. This also covers
                // an old game that lila's queue of 32 events dropped earlier.
                (new && !self.replaying && !game.has_moved).then_some(Alert::GameStart {
                    opponent: game.opponent.username,
                    full_id: game.full_id,
                })
            }
            Event::Other => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    /// From lichess's API spec.
    fn challenge_line(id: &str) -> Value {
        json!({
            "type": "challenge",
            "challenge": {
                "id": id,
                "url": format!("https://lichess.org/game/{id}"),
                "status": "created",
                "challenger": {
                    "name": "Adriana",
                    "flair": "smileys.smiling-cat-with-heart-eyes",
                    "id": "adriana",
                    "rating": 627,
                    "provisional": true
                },
                "destUser": { "name": "Gabriela", "id": "gabriela", "rating": 1603, "online": true },
                "variant": { "key": "standard", "name": "Standard", "short": "Std" },
                "rated": false,
                "speed": "correspondence",
                "timeControl": { "type": "unlimited" },
                "color": "random",
                "finalColor": "black",
                "perf": { "icon": "", "name": "Correspondence" }
            },
            "compat": { "bot": false, "board": true }
        })
    }

    /// From lichess's API spec.
    fn game_line(id: &str) -> Value {
        json!({
            "type": "gameStart",
            "game": {
                "fullId": format!("{id}91cB"),
                "gameId": id,
                "fen": "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                "color": "white",
                "lastMove": "",
                "source": "friend",
                "status": { "id": 20, "name": "started" },
                "variant": { "key": "standard", "name": "Standard" },
                "speed": "blitz",
                "perf": "blitz",
                "rated": true,
                "hasMoved": false,
                "opponent": { "id": "aaron", "username": "Aaron", "rating": 879 },
                "isMyTurn": true,
                "secondsLeft": 300,
                "rating": 784,
                "compat": { "bot": false, "board": true },
                "id": id
            }
        })
    }

    fn challenge_alert(id: &str) -> Alert {
        Alert::Challenge {
            from: Some("Adriana".into()),
            url: format!("https://lichess.org/game/{id}"),
        }
    }

    fn game_alert(id: &str) -> Alert {
        Alert::GameStart {
            opponent: "Aaron".into(),
            full_id: format!("{id}91cB"),
        }
    }

    /// Past the replay of the first connection.
    fn live() -> Seen {
        let mut seen = Seen::new("Gabriela");
        seen.line("");
        seen
    }

    fn feed(seen: &mut Seen, line: &Value) -> Option<Alert> {
        seen.line(&line.to_string())
    }

    #[test]
    fn reads_the_events_of_the_api_spec() {
        let mut seen = live();
        assert_eq!(
            feed(&mut seen, &challenge_line("qUfdwN17")),
            Some(challenge_alert("qUfdwN17"))
        );
        assert_eq!(
            feed(&mut seen, &game_line("p8Jy2ayw")),
            Some(game_alert("p8Jy2ayw"))
        );
    }

    #[test]
    fn reads_an_ai_opponent() {
        let mut line = game_line("kUhShhgl");
        line["game"]["opponent"] = json!({ "id": null, "username": "Stockfish level 1", "ai": 1 });
        assert_eq!(
            feed(&mut live(), &line),
            Some(Alert::GameStart {
                opponent: "Stockfish level 1".into(),
                full_id: "kUhShhgl91cB".into(),
            })
        );
    }

    #[test]
    fn reads_an_anonymous_challenger() {
        let mut absent = challenge_line("qUfdwN17");
        absent["challenge"]
            .as_object_mut()
            .unwrap()
            .remove("challenger");
        let mut null = challenge_line("qUfdwN17");
        null["challenge"]["challenger"] = Value::Null;
        for line in [absent, null] {
            assert_eq!(
                feed(&mut live(), &line),
                Some(Alert::Challenge {
                    from: None,
                    url: "https://lichess.org/game/qUfdwN17".into(),
                })
            );
        }
    }

    #[test]
    fn other_lines_are_no_news() {
        let mut seen = live();
        for line in [
            r#"{"type":"gameFinish","game":{"gameId":"p8Jy2ayw"}}"#,
            r#"{"type":"challengeCanceled","challenge":{"id":"qUfdwN17"}}"#,
            r#"{"type":"challengeDeclined","challenge":{"id":"qUfdwN17"}}"#,
            r#"{"type":"somethingNew"}"#,
            r#"{"type":"gameStart","game":{}}"#,
            r#"{"no":"type"}"#,
            "not json",
        ] {
            assert_eq!(seen.line(line), None, "{line}");
        }
    }

    #[test]
    fn the_first_replay_gives_no_alerts_but_is_remembered() {
        let mut seen = Seen::new("Gabriela");
        assert_eq!(feed(&mut seen, &challenge_line("oldChall")), None);
        assert_eq!(feed(&mut seen, &game_line("oldGame1")), None);
        seen.line("");
        assert_eq!(feed(&mut seen, &challenge_line("oldChall")), None);
        assert_eq!(feed(&mut seen, &game_line("oldGame1")), None);
        assert_eq!(
            feed(&mut seen, &challenge_line("newChall")),
            Some(challenge_alert("newChall"))
        );
    }

    #[test]
    fn the_replay_after_a_reconnect_only_alerts_about_the_gap() {
        let mut seen = live();
        assert!(feed(&mut seen, &challenge_line("qUfdwN17")).is_some());
        assert!(feed(&mut seen, &game_line("p8Jy2ayw")).is_some());
        // The connection drops. The new one replays the state, with no marker.
        assert_eq!(feed(&mut seen, &challenge_line("qUfdwN17")), None);
        assert_eq!(feed(&mut seen, &game_line("p8Jy2ayw")), None);
        assert_eq!(
            feed(&mut seen, &game_line("inTheGap")),
            Some(game_alert("inTheGap"))
        );
    }

    #[test]
    fn a_connection_lost_during_the_first_replay_is_still_a_replay() {
        let mut seen = Seen::new("Gabriela");
        assert_eq!(feed(&mut seen, &game_line("oldGame1")), None);
        // No keep-alive yet. The next connection sends the rest of the state.
        assert_eq!(feed(&mut seen, &game_line("oldGame2")), None);
    }

    #[test]
    fn a_game_the_user_has_moved_in_is_no_news() {
        let mut line = game_line("p8Jy2ayw");
        line["game"]["hasMoved"] = json!(true);
        assert_eq!(feed(&mut live(), &line), None);
    }

    #[test]
    fn the_game_of_an_accepted_challenge_still_alerts() {
        let mut seen = live();
        assert!(feed(&mut seen, &challenge_line("sameId12")).is_some());
        assert_eq!(
            feed(&mut seen, &game_line("sameId12")),
            Some(game_alert("sameId12"))
        );
    }

    #[test]
    fn only_incoming_challenges_alert() {
        let mut outgoing = challenge_line("qUfdwN17");
        outgoing["challenge"]["challenger"] = json!({ "id": "gabriela", "name": "Gabriela" });
        outgoing["challenge"]["destUser"] = json!({ "id": "adriana", "name": "Adriana" });
        let mut open = challenge_line("qUfdwN18");
        open["challenge"]["challenger"] = json!({ "id": "gabriela", "name": "Gabriela" });
        open["challenge"]["destUser"] = Value::Null;

        let mut seen = live();
        assert_eq!(feed(&mut seen, &outgoing), None);
        assert_eq!(feed(&mut seen, &open), None);
    }

    #[test]
    fn the_text_names_the_other_player() {
        let strings = Strings::load(crate::i18n::BASE);
        assert_eq!(
            challenge_alert("qUfdwN17").body(&strings),
            "Challenge from Adriana"
        );
        let anonymous = Alert::Challenge {
            from: None,
            url: String::new(),
        };
        assert_eq!(anonymous.body(&strings), "Challenge from Anonymous");
        assert_eq!(
            game_alert("p8Jy2ayw").body(&strings),
            "Game started vs Aaron"
        );
    }

    #[test]
    fn a_click_opens_the_challenge_or_the_players_page() {
        assert_eq!(
            challenge_alert("qUfdwN17").url().unwrap().as_str(),
            "https://lichess.org/game/qUfdwN17"
        );
        assert_eq!(
            game_alert("p8Jy2ayw").url().unwrap().as_str(),
            "https://lichess.org/p8Jy2ayw91cB"
        );
    }
}
