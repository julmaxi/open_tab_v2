use std::{collections::{HashMap, HashSet}, sync::{Weak, Arc}, convert::Infallible, pin::Pin, time::Duration};

use open_tab_entities::{EntityGroup, schema, domain::{tournament, self, entity::LoadEntity, ballot::SpeechRole}};
use sea_orm::{prelude::Uuid, DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ConnectionTrait};
use tokio::{pin, sync::{broadcast::{Receiver, Sender}, Mutex, RwLock}};
use tokio_stream::{Stream, StreamMap, wrappers::BroadcastStream, StreamExt};

use axum::{extract::{Path, State}, response::{Sse, sse::Event}, Json, Router, routing::get};
use serde::{Serialize, Deserialize};
use tracing::Subscriber;
use weak_table::WeakValueHashMap;

use crate::{state::AppState, response::{handle_error, APIError}, auth::ExtractAuthenticatedUser};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParticipantEventType {
    DebateMotionReleaseUpdated{debate_id: Uuid},
    ReleaseTimeUpdated{round_id: Uuid, new_time: Option<chrono::NaiveDateTime>, time: ReleaseTime},
    SpeechTimeUpdate{
        speech_role: SpeechRole,
        speech_position: i32,
        is_response: bool,
        start: Option<chrono::NaiveDateTime>,
        end: Option<chrono::NaiveDateTime>,
        pause_milliseconds: i32,
    },
    ActiveSpeechUpdate {
        speech: Option<DebateCurrentSpeech>
    },
}


impl ParticipantEventType {
    fn channel(&self) -> &str {
        match self {
            ParticipantEventType::DebateMotionReleaseUpdated{..} => "participant",
            ParticipantEventType::ReleaseTimeUpdated{..} => "participant",
            ParticipantEventType::SpeechTimeUpdate{..} => "timer",
            ParticipantEventType::ActiveSpeechUpdate { .. } => "timer"
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DebateCurrentSpeech {
    speech_role: SpeechRole,
    speech_position: u8,
    is_response: bool
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum ReleaseTime {
    Draw,
    MotionForTeams,
    DebateStart,
    MotionForAll,
    RoundClose
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantEvent {
    pub event: ParticipantEventType,
}

pub struct TournamentBroadcastState {
    pub round_times: HashMap<Uuid, HashMap<ReleaseTime, Option<chrono::NaiveDateTime>>>,
}

pub struct ParticipantNotificationManager {
    pub participant_broadcast_senders: HashMap<Uuid, Sender<ParticipantEvent>>,
    pub tournament_broadcast_senders: HashMap<Uuid, Sender<ParticipantEvent>>,
    pub tournament_broadcast_states: WeakValueHashMap<Uuid, Weak<Mutex<TournamentBroadcastState>>>,
}

impl ParticipantNotificationManager {
    pub fn new() -> Self {
        Self {
            participant_broadcast_senders: HashMap::new(),
            tournament_broadcast_senders: HashMap::new(),
            tournament_broadcast_states: WeakValueHashMap::new(),
        }
    }
    
    pub async fn subscribe_to_tournament<C>(&mut self, db: &C, tournament_id: Uuid) -> Result<Pin<Box<dyn Stream<Item=Result<Event, Infallible>> + Send>>, anyhow::Error> where C: sea_orm::ConnectionTrait {
        let receiver = self.tournament_broadcast_senders
            .entry(tournament_id)
            .or_insert_with(|| Sender::new(100))
            .subscribe();

        let stream = BroadcastStream::new(receiver);

        let state = if let Some(state) = self.tournament_broadcast_states.get(&tournament_id) {
            state
        } else {
            let rounds = domain::round::TournamentRound::get_all_in_tournament(db, tournament_id).await?;
            let state = Arc::new(Mutex::new(TournamentBroadcastState {
                round_times: rounds.into_iter().map(|round| {
                    let mut map = HashMap::new();
                    map.insert(ReleaseTime::Draw, round.draw_release_time);
                    map.insert(ReleaseTime::MotionForTeams, round.team_motion_release_time);
                    map.insert(ReleaseTime::DebateStart, round.debate_start_time);
                    map.insert(ReleaseTime::MotionForAll, round.full_motion_release_time);
                    map.insert(ReleaseTime::RoundClose, round.round_close_time);
                    (round.uuid, map)
                }).collect()
            }));
            self.tournament_broadcast_states.insert(tournament_id, state.clone());
            state
        };

        let stream = stream.filter_map(move |e| {
            //Me move state to the stream, so it will be dropped when the stream is dropped
            let _ = &state;
            e.ok()
        });

        let stream = stream.map(|e| Ok(Event::default().event(
            e.event.channel()
        ).data(serde_json::to_string(&e).unwrap())));

        Ok(Box::pin(stream))
    }

    pub async fn subscribe_to_participant(&mut self, participant_id: Uuid) -> Result<Pin<Box<dyn Stream<Item=Result<Event, Infallible>> + Send>>, anyhow::Error> {
        let receiver = self.participant_broadcast_senders
            .entry(participant_id)
            .or_insert_with(|| Sender::new(100))
            .subscribe();

        let stream = BroadcastStream::new(receiver);
        let stream = stream.filter_map(|e| e.ok());

        let stream = stream.map(|e| Ok(Event::default().event(
            e.event.channel()
        ).data(serde_json::to_string(&e).unwrap())));

        Ok(Box::pin(stream))
    }

    pub async fn notify_debate_non_aligned_motion_release_state<C>(&self, db: &C, debate_id: Uuid) -> Result<(), anyhow::Error> where C: ConnectionTrait {
        self.notify_debate(db, debate_id, ParticipantEvent {
            event: ParticipantEventType::DebateMotionReleaseUpdated {
                debate_id
            }
        }).await   
    }

    pub async fn notify_debate<C>(&self, db: &C, debate_id: Uuid, event: ParticipantEvent) -> Result<(), anyhow::Error> where C: ConnectionTrait {
        let debate = schema::tournament_debate::Entity::find_by_id(debate_id).one(db).await?;
        if let Some(debate) = debate {
            let ballot = domain::ballot::Ballot::get(db, debate.ballot_id).await?;

            let mut team_ids = vec![];
            if let Some(gov_team_id) = ballot.government.team {
                team_ids.push(gov_team_id);
            }
            if let Some(opp_team_id) = ballot.opposition.team {
                team_ids.push(opp_team_id);
            }

            let mut all_participants = HashSet::new();

            let team_members = schema::speaker::Entity::find()
                .filter(schema::speaker::Column::TeamId.is_in(team_ids)).all(db).await?;

            all_participants.extend(team_members.into_iter().map(|m| m.uuid));

            all_participants.extend(ballot.speeches.iter().filter_map(|speech| {
                speech.speaker
            }).collect::<Vec<_>>());
            

            all_participants.extend(ballot.president.iter());

            all_participants.extend(ballot.adjudicators.iter());

            for participant_id in all_participants {
                if let Some(sender) = self.participant_broadcast_senders.get(&participant_id) {
                    //We ignore the send error
                    let r = sender.send(event.clone());
                }
            }
        }
        Ok(())
    }

    pub async fn process_entities<C>(&self, _db: &C, entities: &EntityGroup) where C: ConnectionTrait {
        //We only process the round notifications here. All other notifications are handled by
        //the individual server endpoint directly, since the associated values will typically never
        //be updated in a sync from the frontend.
        for round in &entities.tournament_rounds {
            if let Some(sender) = self.tournament_broadcast_senders.get(&round.tournament_id) {
                let prev_state = self.tournament_broadcast_states.get(&round.tournament_id);

                if let Some(prev_state) = prev_state {
                    let mut prev_state = prev_state.lock().await;
                    let states = vec![
                        (ReleaseTime::Draw, round.draw_release_time),
                        (ReleaseTime::MotionForTeams, round.team_motion_release_time),
                        (ReleaseTime::DebateStart, round.debate_start_time),
                        (ReleaseTime::MotionForAll, round.full_motion_release_time),
                        (ReleaseTime::RoundClose, round.round_close_time),
                    ];

                    for (release_time, new_time) in states {
                        let prev_time = prev_state.round_times.get(&round.uuid).unwrap().get(&release_time).unwrap();
                        if prev_time != &new_time {
                            let entry = prev_state.round_times.entry(round.uuid).or_insert_with(|| HashMap::new());
                            entry.insert(release_time.clone(), new_time.clone());
                            
                            //TODO: Temporary fix
                            let _ = sender.send(ParticipantEvent {
                                event: ParticipantEventType::ReleaseTimeUpdated {
                                    round_id: round.uuid,
                                    new_time: new_time.clone(),
                                    time: release_time.clone(),
                                }
                            });
                        }
                    }
                }
                else {
                    eprint!("No state for tournament {}", round.tournament_id);
                }
            }
        }
    }
}

pub async fn get_participant_events(
    State(db): State<DatabaseConnection>,
    State(notifications): State<Arc<RwLock<ParticipantNotificationManager>>>,
    Path(participant_id): Path<Uuid>,
)-> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, APIError> {
    let mut notifications = notifications.write().await;
    let mut result = schema::participant::Entity::find()
        .find_with_related(schema::tournament::Entity)
        .filter(schema::participant::Column::Uuid.eq(participant_id))
    .all(&db).await.map_err(handle_error)?;

    if result.len() == 0 {
        return Err(APIError::from((axum::http::StatusCode::NOT_FOUND, "Submission not found")));
    }

    let (participant, mut tournament) = result.pop().unwrap();
    let tournament = tournament.pop().expect("Guaranteed by db constraints");
    
    let tournament_stream = notifications.subscribe_to_tournament(&db, tournament.uuid).await?;
    let participant_stream = notifications.subscribe_to_participant(participant.uuid).await?;

    let joint_stream = tournament_stream.merge(participant_stream);

    Ok(Sse::new(joint_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    ))
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications/participant/:participant_id", get(get_participant_events))
}