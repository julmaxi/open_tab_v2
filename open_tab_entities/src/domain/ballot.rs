use std::{collections::{HashMap, HashSet}, iter::{zip, self}, cmp::Ordering, error::Error, fmt::Display, str::FromStr};

use async_trait::async_trait;
use sea_orm::JoinType;
use sea_orm::{prelude::*, ActiveValue, Condition, QuerySelect};
use serde::{Serialize, Deserialize};

use crate::schema::{self};

use itertools::{izip, Itertools};

use super::TournamentEntity;
use crate::utilities::{BatchLoad, BatchLoadError};

#[derive(Debug, PartialEq, Eq)]
pub enum BallotParseError {
    UnknownTeamRole,
    UnknownSpeechRole,
    UnknownJudgeRole,
    TooManyPresidents,
    BallotDoesNotExist(String),
    DbErr(DbErr)
}

impl Display for BallotParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))?;
        Ok(())
    }
}

impl Error for BallotParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BallotParseError::DbErr(e) => Some(e),
            _ => None
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<DbErr> for BallotParseError {
    fn from(value: DbErr) -> Self {
        BallotParseError::DbErr(value)
    }
}


#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize, Clone)]
pub struct Ballot {
    pub uuid: Uuid,
    pub speeches: Vec<Speech>,
    pub government: BallotTeam,
    pub opposition: BallotTeam,

    pub adjudicators: Vec<Uuid>,
    pub president: Option<Uuid>
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all="snake_case")]
pub enum SpeechRole {
    Government,
    Opposition,
    NonAligned
}

impl FromStr for SpeechRole {
    type Err = BallotParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "g" => Ok(SpeechRole::Government),
            "o" => Ok(SpeechRole::Opposition),
            "n" => Ok(SpeechRole::NonAligned),
            _ => Err(BallotParseError::UnknownSpeechRole)
        }
    }
}

impl SpeechRole {
    fn to_str(&self) -> String {
        match self {
            SpeechRole::Government => "g".into(),
            SpeechRole::Opposition => "o".into(),
            SpeechRole::NonAligned => "n".into()
        }
    }
}


#[derive(Debug, PartialEq, Eq)]
pub enum JudgeRole {
    Normal,
    President,
}

impl FromStr for JudgeRole {
    type Err = BallotParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "n" => Ok(JudgeRole::Normal),
            "p" => Ok(JudgeRole::President),
            _ => Err(BallotParseError::UnknownJudgeRole)
        }
    }
}

impl JudgeRole {
    /*fn from_str(s: &str) -> Result<JudgeRole, BallotParseError> {
        match s {
            "n" => Ok(JudgeRole::Normal),
            "p" => Ok(JudgeRole::President),
            _ => Err(BallotParseError::UnknownSpeechRole)
        }
    }*/

    fn to_str(&self) -> String {
        match self {
            JudgeRole::President => "p".into(),
            JudgeRole::Normal => "n".into(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Speech {
    pub speaker: Option<Uuid>,
    pub role: SpeechRole,
    pub position: u8,
    pub scores: HashMap<Uuid, SpeakerScore>,
}

impl Speech {
    pub fn speaker_score(&self) -> Option<f64> {
        if self.scores.len() > 0 {
            Some(
                self.scores.values().map(|s| s.total() as f64).sum::<f64>() / self.scores.len() as f64
            )
        }
        else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize, Clone)]
pub struct BallotTeam {
    pub team: Option<Uuid>,
    pub scores: HashMap<Uuid, TeamScore>
}

impl BallotTeam {
    pub fn team_score(&self) -> Option<f64> {
        if self.scores.len() > 0 {
            Some(
                self.scores.values().map(|s| s.total() as f64).sum::<f64>() / self.scores.len() as f64
            )
        }
        else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum SpeakerScore {
    Aggregate(i16)
}

impl SpeakerScore {
    pub fn total(&self) -> i16{
        match self {
            SpeakerScore::Aggregate(s) => *s,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub enum TeamScore {
    Aggregate(i16)
}

impl TeamScore {
    pub fn total(&self) -> i16 {
        match self {
            TeamScore::Aggregate(s) => *s,
        }
    }
}

impl Ballot {
    pub async fn get_one(db: &impl ConnectionTrait, uuid: Uuid) -> Result<Ballot, BallotParseError> {
        Self::get_many(db, vec![uuid]).await.map(|r| r.into_iter().next().unwrap())
    }

    pub async fn try_get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Option<Ballot>>, BallotParseError> where C: ConnectionTrait {
        let ballots = schema::ballot::Entity::batch_load(db, uuids.clone()).await?;
        let has_value = ballots.iter().map(|b| b.is_some()).collect_vec();
        let mut retrieved_ballots_iter = Self::get_from_ballots(db, ballots.into_iter().filter(|b| b.is_some()).map(|b| b.unwrap()).collect()).await?.into_iter();

        Ok(has_value.into_iter().map(|has_value| {
            if has_value {
                retrieved_ballots_iter.next()
            }
            else {
                None
            }
        }).collect())
    }

    pub async fn get_many<C>(db: &C, uuids: Vec<Uuid>) -> Result<Vec<Ballot>, BallotParseError> where C: ConnectionTrait {
        let ballots = schema::ballot::Entity::batch_load_all(db, uuids.clone()).await.map_err(|e| match e {
            BatchLoadError::DbErr(e) => BallotParseError::DbErr(e),
            BatchLoadError::RowNotFound { id } => BallotParseError::BallotDoesNotExist(id)
        })?;

        Self::get_from_ballots(db, ballots).await
    }

    pub async fn get_all_in_rounds<C>(db: &C, round_uuids: Vec<Uuid>) -> Result<Vec<(Uuid, Ballot)>, BallotParseError> where C: ConnectionTrait {
        //TODO: With a little work, could do this in one query for the rounds.
        //Custom return values are a bit annoying though, so we leave this for later.

        let mut rounds = vec![];
        let mut ballots = vec![];
        for round_id in round_uuids {
            let round_ballots = schema::ballot::Entity::find().inner_join(
                schema::tournament_debate::Entity
            ).filter(
                schema::tournament_debate::Column::RoundId.eq(round_id)
            ).all(db).await.map_err(|e| BallotParseError::DbErr(e))?;

            rounds.extend(itertools::repeat_n(round_id, round_ballots.len()));
            ballots.extend(round_ballots);
        }
        Ok(zip(rounds, Self::get_from_ballots(db, ballots).await?).collect_vec())
    }

    async fn get_from_ballots<C>(db: &C, ballots: Vec<schema::ballot::Model>) -> Result<Vec<Ballot>, BallotParseError> where C: ConnectionTrait {
        let teams = ballots.load_many(schema::ballot_team::Entity, db).await?;
        let adjudicators = ballots.load_many(schema::ballot_adjudicator::Entity, db).await?;

        let team_scores = ballots.load_many(schema::adjudicator_team_score::Entity, db).await?;

        let speeches = ballots.load_many(schema::ballot_speech::Entity, db).await?;
        let speech_scores = ballots.load_many(schema::adjudicator_speech_score::Entity, db).await?;

        let ballots : Result<Vec<_>, _> = izip!(ballots.into_iter(), teams.into_iter(), adjudicators.into_iter(), team_scores.into_iter(), speeches.into_iter(), speech_scores.into_iter()).map(|
            (b, t, a, s, sp, sps)
        | Ballot::from_rows(b, t, a, s, sp, sps)).collect();
        
        ballots
    }

    /// Converts a set of query results into a Ballot.
    /// This function assumes that the following basic integrity checks have been verfied before
    /// For the default schema, this is ensured by the database constraints
    /// 1. For each ballot, there can only be one team per role string
    /// 2. All teams, speeches and scores belong to the ballot
    /// 3. A speech that does not exists can have no scores
    /// 4. A team that does not exists can have no scores
    /// 5. An adjudicator not in adjudicators can not give scores
    fn from_rows(
        ballot: schema::ballot::Model,
        teams: Vec<schema::ballot_team::Model>,
        mut adjudicators: Vec<schema::ballot_adjudicator::Model>,
        team_scores: Vec<schema::adjudicator_team_score::Model>,
        speeches: Vec<schema::ballot_speech::Model>,
        speech_scores: Vec<schema::adjudicator_speech_score::Model>,
    ) -> Result<Self, BallotParseError> {
        adjudicators.sort_by(|a, b| i32::cmp(&a.position,&b.position));
        let (chair, adjudicators) : (Vec<_>, Vec<_>) = adjudicators.into_iter().partition(|a| a.role == JudgeRole::President.to_str());
        let adjudicators = adjudicators.into_iter().map(|a| a.adjudicator_id).collect();
        
        let chair = match chair.len() {
            0 => Ok(None),
            1 => Ok(chair.into_iter().map(|a| a.adjudicator_id).next()),
            _ => Err(BallotParseError::TooManyPresidents)
        }?;

        let gov_team_id = teams.iter().find(|t| t.role == "g").map(|t| t.team_id);
        let opp_team_id = teams.iter().find(|t| t.role == "o").map(|t| t.team_id);

        if gov_team_id.map_or(0, |_| 1) + opp_team_id.map_or(0, |_| 1) != teams.len() {
            return Err(BallotParseError::UnknownTeamRole)
        }

        let gov_team_id = gov_team_id.flatten();
        let opp_team_id = opp_team_id.flatten();

        let gov_scores : HashMap<Uuid, TeamScore> = team_scores.iter().filter(|s| s.role_id == "g").map(
            |score| (score.adjudicator_id, TeamScore::Aggregate(score.manual_total_score.unwrap() as i16))
        ).collect();
        let opp_scores : HashMap<Uuid, TeamScore> = team_scores.iter().filter(|s| s.role_id == "o").map(
            |score| (score.adjudicator_id, TeamScore::Aggregate(score.manual_total_score.unwrap() as i16))
        ).collect();

        if gov_scores.len() + opp_scores.len() != team_scores.len() {
            return Err(BallotParseError::UnknownTeamRole)
        }

        let government = BallotTeam {
            team: gov_team_id,
            scores: gov_scores
        };

        let opposition = BallotTeam {
            team: opp_team_id,
            scores: opp_scores
        };

        let mut speech_score_map = HashMap::new();
        for score in  speech_scores.into_iter() {
            speech_score_map.entry((score.speech_role, score.speech_position)).or_insert(HashMap::new()).insert(
                score.adjudicator_id,
                SpeakerScore::Aggregate(score.manual_total_score.unwrap_or(0) as i16)
            );
        };

        let speeches : Result<Vec<Speech>, BallotParseError> = speeches.into_iter().map(
            |s| -> Result<Speech, BallotParseError> {
                let speaker = s.speaker_id;
                let role = SpeechRole::from_str(&s.role)?;

                let scores = speech_score_map.remove(&(s.role, s.position)).unwrap_or_else(HashMap::new);
                Ok(Speech {
                    speaker,
                    role,
                    position: s.position as u8,
                    scores
                })
            }
        ).collect();
        let mut speeches = speeches?;

        speeches.sort_by(|s1, s2| {
            match (&s1.role, &s2.role) {
                (SpeechRole::NonAligned, _) => {
                    if s2.position <= 1 {
                        Ordering::Greater
                    }
                    else {
                        Ordering::Less
                    }
                },
                (_, SpeechRole::NonAligned) => {
                    if s1.position <= 1 {
                        Ordering::Less
                    }
                    else {
                        Ordering::Greater
                    }
                },
                (role_1, role_2) => {
                    if s1.position != s2.position {
                        u8::cmp(&s1.position, &s2.position)
                    }
                    else {
                        let c = SpeechRole::cmp(&role_1, &role_2);

                        if s1.position == 2 {
                            c.reverse()
                        }
                        else {
                            c
                        }
                    }
                }
            }
        });

        Ok(
            Ballot { uuid: ballot.uuid, speeches, government, opposition, adjudicators, president: chair }
        )

    }

    pub async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), DbErr> where C: ConnectionTrait {
        let mut ballot_model = schema::ballot::ActiveModel {
            uuid: ActiveValue::Set(self.uuid)
        };

        let is_insert = if guarantee_insert {
            ballot_model.insert(db).await?;
            true
        }
        else {
            let r = schema::ballot::Entity::find_by_id(self.uuid).one(db).await?;
            if let Some(_) = r {
                ballot_model.uuid = ActiveValue::Unchanged(self.uuid);
                ballot_model.update(db).await?;
                false
            }
            else {
                ballot_model.insert(db).await?;
                true
            }
        };

        self.save_adjudicators(db, is_insert).await?;
        self.save_teams(db, is_insert).await?;
        self.save_speeches(db, is_insert).await?;
        Ok(())
    }

    async fn save_adjudicators<C>(&self, db: &C, is_insert: bool) -> Result<(), DbErr> where C: ConnectionTrait {
        let current_adjudicators : HashMap<Uuid, (i32, bool)> = if !is_insert {
             schema::ballot_adjudicator::Entity::find().filter(schema::ballot_adjudicator::Column::BallotId.eq(self.uuid)).all(db).await?.into_iter().map(|a| (a.adjudicator_id, (a.position, a.role == JudgeRole::President.to_str()))).collect()
        }
        else {
            HashMap::new()
        };

        let current_adjudicator_uuids = current_adjudicators.keys().map(|x| *x);

        let to_delete = current_adjudicator_uuids.filter(|a| !self.adjudicators.contains(a) && self.president != Some(*a)).collect_vec();

        schema::ballot_adjudicator::Entity::delete_many().filter(
            Condition::all()
            .add(schema::ballot_adjudicator::Column::AdjudicatorId.is_in(
                to_delete
            ))
            .add(
                schema::ballot_adjudicator::Column::BallotId.eq(self.uuid)
            )
        ).exec(db).await?;

        for (idx, adj) in self.adjudicators.iter().enumerate() {
            let idx = idx as i32;
            if let Some((prev_pos, was_chair)) = current_adjudicators.get(adj) {
                if *prev_pos != idx || *was_chair {
                    schema::ballot_adjudicator::ActiveModel {
                        ballot_id: ActiveValue::Unchanged(self.uuid),
                        adjudicator_id: ActiveValue::Unchanged(*adj),
                        position: ActiveValue::Set(idx),
                        role: ActiveValue::Set(JudgeRole::Normal.to_str()),
                    }.update(db).await?;
                }
            }
            else {
                schema::ballot_adjudicator::ActiveModel {
                    ballot_id: ActiveValue::Set(self.uuid),
                    adjudicator_id: ActiveValue::Set(*adj),
                    position: ActiveValue::Set(idx),
                    role: ActiveValue::Set(JudgeRole::Normal.to_str()),
                }.insert(db).await?;
            }
        }

        if is_insert {
            if let Some(president) = self.president {
                schema::ballot_adjudicator::ActiveModel {
                    ballot_id: ActiveValue::Unchanged(self.uuid),
                    adjudicator_id: ActiveValue::Unchanged(president),
                    position: ActiveValue::Set(0),
                    role: ActiveValue::Set(JudgeRole::President.to_str()),
                }.insert(db).await?;
            }
        }
        else {
            if let Some(new_president) = self.president {
                if current_adjudicators.contains_key(&new_president) {
                    schema::ballot_adjudicator::ActiveModel {
                        ballot_id: ActiveValue::Unchanged(self.uuid),
                        adjudicator_id: ActiveValue::Unchanged(new_president),
                        position: ActiveValue::Set(0),
                        role: ActiveValue::Set(JudgeRole::President.to_str()),
                    }.update(db).await?;
                }
                else {
                    schema::ballot_adjudicator::ActiveModel {
                        ballot_id: ActiveValue::Set(self.uuid),
                        adjudicator_id: ActiveValue::Set(new_president),
                        position: ActiveValue::Set(0),
                        role: ActiveValue::Set(JudgeRole::President.to_str()),
                    }.insert(db).await?;
                }
            }
            // If there is no president and there was a previous
            // it has been deleted already
        }

        Ok(())
    }

    async fn save_teams<C>(&self, db: &C, is_insert: bool) -> Result<(), DbErr> where C: ConnectionTrait {
        let (current_teams, current_scores) = if !is_insert {
            let current_teams = schema::ballot_team::Entity::find().filter(schema::ballot_team::Column::BallotId.eq(self.uuid)).all(db).await?;
            let current_scores : Vec<HashMap<_, _>> = current_teams.load_many(schema::adjudicator_team_score::Entity, db).await?.into_iter().map(
                |scores| {
                    scores.into_iter().map(|score| (score.adjudicator_id, score)).collect()
                }
            ).collect();

            (current_teams, current_scores)
        }
        else {
            (vec![], vec![])
        };


        // FIXME: We might want to guard against too many teams here

        let (gov, opp) : (Vec<_>, Vec<_>) = zip(current_teams, current_scores).into_iter().partition(|(t, _s)| t.role == SpeechRole::Government.to_str());

        for (team_model, new_team_entry, role) in izip!(vec![gov, opp].into_iter(), vec![&self.government, &self.opposition].into_iter(), vec![SpeechRole::Government, SpeechRole::Opposition].into_iter()) {
            if team_model.len() == 0 {
                schema::ballot_team::ActiveModel {
                    ballot_id: ActiveValue::Set(self.uuid),
                    team_id: ActiveValue::Set(new_team_entry.team),
                    role: ActiveValue::Set(role.to_str())
                }.insert(db).await?;

                for (adj, score) in new_team_entry.scores.iter() {
                    schema::adjudicator_team_score::ActiveModel {
                        adjudicator_id: ActiveValue::Set(*adj),
                        ballot_id: ActiveValue::Set(self.uuid),
                        role_id: ActiveValue::Set(role.to_str()),
                        manual_total_score: ActiveValue::Set(Some(score.total() as i32)),
                    }.insert(db).await?;
                }
            }
            else {
                let (team, scores) = team_model.into_iter().next().unwrap();
                if team.team_id != new_team_entry.team {
                    schema::ballot_team::ActiveModel {
                        team_id: ActiveValue::Set(new_team_entry.team),
                        ..team.into()
                    }.update(db).await?;
                }

                let scores_to_delete = scores.keys().filter(|adj| !new_team_entry.scores.contains_key(*adj)).map(|x| *x).collect_vec();

                let mut filter_condition = Condition::any();
                for adj in scores_to_delete.into_iter() {
                    filter_condition = filter_condition.add(
                        Condition::all()
                        .add(schema::adjudicator_team_score::Column::BallotId.eq(self.uuid))
                        .add(schema::adjudicator_team_score::Column::AdjudicatorId.eq(adj))
                        .add(schema::adjudicator_team_score::Column::RoleId.eq(role.to_str()))
                    );
                }
                schema::adjudicator_team_score::Entity::delete_many().filter(
                    filter_condition
                ).exec(db).await?;

                for (adj, score) in new_team_entry.scores.iter() {
                    if let Some(old_score) = scores.get(adj) {
                        if score.total() as i32 != old_score.manual_total_score.unwrap() {
                            schema::adjudicator_team_score::ActiveModel {
                                adjudicator_id: ActiveValue::Unchanged(*adj),
                                ballot_id: ActiveValue::Unchanged(self.uuid),
                                role_id: ActiveValue::Unchanged(role.to_str()),
                                manual_total_score: ActiveValue::Set(Some(score.total() as i32)),
                            }.update(db).await?;
                        }
                    }
                    else {
                        schema::adjudicator_team_score::ActiveModel {
                            adjudicator_id: ActiveValue::Set(*adj),
                            ballot_id: ActiveValue::Set(self.uuid),
                            role_id: ActiveValue::Set(role.to_str()),
                            manual_total_score: ActiveValue::Set(Some(score.total() as i32)),
                        }.insert(db).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn save_speeches<C>(&self, db: &C, is_insert: bool) -> Result<(), DbErr> where C: ConnectionTrait {
        let (current_speeches, current_scores) = if !is_insert {
            let current_speeches = schema::ballot_speech::Entity::find().filter(schema::ballot_speech::Column::BallotId.eq(self.uuid)).all(db).await?;
            let current_scores = current_speeches.load_many(schema::adjudicator_speech_score::Entity, db).await?;

            (
                current_speeches, 
                current_scores
            )
        }
        else {
            (vec![], vec![])
        };

        let current_speeches : HashMap<(String, i32), (_, HashMap<_, _>)> = zip(current_speeches.into_iter(), current_scores.into_iter()).map(|(speech, scores)| {
            ((speech.role.clone(), speech.position.clone()), (speech, scores.into_iter().map(
                |score| (score.adjudicator_id, score)
            ).collect()))
        }).collect();

        let to_delete : HashSet<_> = current_speeches.keys().map(|k| k.clone()).collect();
        let existing_keys : HashSet<_> = self.speeches.iter().map(|speech| (speech.role.to_str(), speech.position as i32)).collect();
        let to_delete = to_delete.difference(&existing_keys);
        // The number of speeches per ballot is low, so this should be fine.

        let mut filter_condition = Condition::any();

        for (role, position) in to_delete.into_iter() {
            filter_condition = filter_condition.add(
                Condition::all()
                .add(schema::ballot_speech::Column::BallotId.eq(self.uuid))
                .add(schema::ballot_speech::Column::Role.eq(role))
                .add(schema::ballot_speech::Column::Position.eq(*position))
            );
        }

        schema::ballot_speech::Entity::delete_many().filter(
            filter_condition
        ).exec(db).await?;

        for speech in self.speeches.iter() {
            let prev_speech = current_speeches.get(&(speech.role.to_str(), speech.position as i32));

            if let Some((prev_speech, prev_scores)) = prev_speech {
                if prev_speech.speaker_id != speech.speaker {
                    schema::ballot_speech::ActiveModel {
                        ballot_id: ActiveValue::Unchanged(self.uuid),
                        position: ActiveValue::Unchanged(prev_speech.position),
                        role: ActiveValue::Unchanged(prev_speech.role.clone()),
                        speaker_id: ActiveValue::Set(speech.speaker)
                    }.update(db).await?;
                }

                for (adj, score) in speech.scores.iter() {
                    if let Some(prev_score) = prev_scores.get(adj) {
                        if prev_score.manual_total_score != Some(score.total() as i32) {
                            schema::adjudicator_speech_score::ActiveModel {
                                adjudicator_id: ActiveValue::Unchanged(*adj),
                                ballot_id: ActiveValue::Unchanged(self.uuid),
                                speech_role: ActiveValue::Unchanged(prev_speech.role.clone()),
                                speech_position: ActiveValue::Unchanged(prev_speech.position ),
                                manual_total_score: ActiveValue::Set(Some(score.total() as i32))
                            }.update(db).await?;
                        }
                    }
                    else {
                        schema::adjudicator_speech_score::ActiveModel {
                            adjudicator_id: ActiveValue::Set(*adj),
                            ballot_id: ActiveValue::Set(self.uuid),
                            speech_role: ActiveValue::Set(speech.role.to_str()),
                            speech_position: ActiveValue::Set(speech.position as i32),
                            manual_total_score: ActiveValue::Set(Some(score.total() as i32)),
                        }.insert(db).await?;    
                    }
                }
            }
            else {
                schema::ballot_speech::ActiveModel {
                    ballot_id: ActiveValue::Set(self.uuid),
                    position: ActiveValue::Set(speech.position as i32),
                    role: ActiveValue::Set(speech.role.to_str()),
                    speaker_id: ActiveValue::Set(speech.speaker)
                }.insert(db).await?;

                for (adj, score) in speech.scores.iter() {
                    schema::adjudicator_speech_score::ActiveModel {
                        adjudicator_id: ActiveValue::Set(*adj),
                        ballot_id: ActiveValue::Set(self.uuid),
                        speech_role: ActiveValue::Set(speech.role.to_str()),
                        speech_position: ActiveValue::Set(speech.position as i32),
                        manual_total_score: ActiveValue::Set(Some(score.total() as i32)),
                    }.insert(db).await?;
                }
            }

        }

        Ok(())
    }

    pub fn government_total(&self) -> Option<f64> {
        self.team_total(SpeechRole::Government)
    }

    pub fn government_speech_total(&self) -> Option<f64> {
        let scores = self.team_speech_scores(SpeechRole::Government);
        
        if scores.is_empty() {
            None
        }
        else {
            Some(scores.into_iter().sum::<f64>())
        }
    }

    pub fn opposition_speech_total(&self) -> Option<f64> {
        let scores = self.team_speech_scores(SpeechRole::Opposition);
        
        if scores.is_empty() {
            None
        }
        else {
            Some(scores.into_iter().sum::<f64>())
        }
    }

    pub fn opposition_total(&self) -> Option<f64> {
        self.team_total(SpeechRole::Opposition)
    }

    pub fn government_speech_scores(&self) -> Vec<f64> {
        self.team_speech_scores(SpeechRole::Government)
    }

    pub fn opposition_speech_scores(&self) -> Vec<f64> {
        self.team_speech_scores(SpeechRole::Opposition)
    }

    fn team_speech_scores(&self, role: SpeechRole) -> Vec<f64> {
        self.speeches.iter().filter(|speech| speech.role == role).filter_map(|speech| speech.speaker_score()).collect()
    }


    fn team_total(&self, role: SpeechRole) -> Option<f64> {
        let scores = self.team_speech_scores(role);
        let team_score = match role {
            SpeechRole::Government => self.government.team_score(),
            SpeechRole::Opposition => self.opposition.team_score(),
            SpeechRole::NonAligned => None
        };

        if scores.len() > 0 || team_score.is_some() {
            let total = scores.into_iter().sum::<f64>();
            Some(total + team_score.unwrap_or(0.0))
        }
        else {
            None
        }
    }

    pub fn is_scored(&self) -> bool {
        return self.government_total().is_some() || self.opposition_total().is_some();
    }
}


#[async_trait]
impl TournamentEntity for Ballot {
    async fn save<C>(&self, db: &C, guarantee_insert: bool) -> Result<(), Box<dyn Error>> where C: ConnectionTrait {
        self.save(db, guarantee_insert).await?;
        Ok(())
    }

    async fn get_tournament<C>(&self, db: &C) -> Result<Option<Uuid>, Box<dyn Error>> where C: ConnectionTrait {
        let id = schema::tournament_round::Entity::find().join(JoinType::InnerJoin, schema::tournament_round::Relation::TournamentDebate.def()).filter(
            schema::tournament_debate::Column::BallotId.eq(self.uuid)
        ).one(db).await.map(|round| round.map(|round| round.tournament_id))?;
        Ok(id)
    }
}


#[test]
fn test_get_empty_ballot() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        }
    , vec![], vec![], vec![], vec![], vec![])?;

    assert_eq!(ballot.uuid, Uuid::from_u128(100));
    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators.len(), 0);
    assert!(ballot.president.is_none());

    Ok(())
}


#[test]
fn test_get_ballot_with_gov_only() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        }
    , vec![
        schema::ballot_team::Model {
            ballot_id: Uuid::from_u128(100),
            team_id: Some(Uuid::from_u128(200)),
            role: "g".into()
        }
    ], vec![], vec![], vec![], vec![])?;

    assert!(ballot.government.team == Some(Uuid::from_u128(200)));
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators.len(), 0);
    assert!(ballot.president.is_none());

    Ok(())
}


#[test]
fn test_get_ballot_with_opp_only() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        }
    , vec![
        schema::ballot_team::Model {
            ballot_id: Uuid::from_u128(100),
            team_id: Some(Uuid::from_u128(201)),
            role: "o".into()
        }
    ], vec![], vec![], vec![], vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team == Some(Uuid::from_u128(201)));
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators.len(), 0);
    assert!(ballot.president.is_none());

    Ok(())
}

#[test]
fn test_get_ballot_with_randomly_ordered_adjudicators() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(310),
                position: 1, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(323),
                position: 2, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(342),
                position: 0, role: "n".into()
            }
        ],
        vec![],
        vec![],
        vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators, vec![
        Uuid::from_u128(342),
        Uuid::from_u128(310),
        Uuid::from_u128(323)
    ]);
    assert!(ballot.president.is_none());

    Ok(())
}

#[test]
fn test_get_ballot_with_position_gaps() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(310),
                position: 100, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(323),
                position: 201, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(342),
                position: 3, role: "n".into()
            }
        ],
        vec![],
        vec![],
        vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators, vec![
        Uuid::from_u128(342),
        Uuid::from_u128(310),
        Uuid::from_u128(323)
    ]);
    assert!(ballot.president.is_none());

    Ok(())
}

#[test]
fn test_get_ballot_with_president() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(301),
                position: 0, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(302),
                position: 1, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(303),
                position: 2, role: "n".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(304),
                position: 0, role: "p".into()
            }
        ],
        vec![],
        vec![],
        vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.len(), 0);
    assert_eq!(ballot.adjudicators, vec![
        Uuid::from_u128(301),
        Uuid::from_u128(302),
        Uuid::from_u128(303)
    ]);
    assert!(ballot.president == Some(Uuid::from_u128(304)));

    Ok(())
}

#[test]
fn test_get_ballot_with_two_presidents() -> Result<(), BallotParseError> {
    let result = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(304),
                position: 0, role: "p".into()
            },
            schema::ballot_adjudicator::Model {
                ballot_id: Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(304),
                position: 1, role: "p".into()
            }
        ],
        vec![],
        vec![],
        vec![]);

    assert_eq!(result, Err(BallotParseError::TooManyPresidents));
    Ok(())
}

#[test]
fn test_get_ballot_speech_order() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![],
        vec![],
        vec![
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 0,
                role: "g".into(),
                speaker_id: Some(Uuid::from_u128(410))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 2,
                role: "o".into(),
                speaker_id: Some(Uuid::from_u128(422))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 1,
                role: "g".into(),
                speaker_id: Some(Uuid::from_u128(411))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 1,
                role: "o".into(),
                speaker_id: Some(Uuid::from_u128(421))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 0,
                role: "n".into(),
                speaker_id: Some(Uuid::from_u128(430))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 0,
                role: "o".into(),
                speaker_id: Some(Uuid::from_u128(420))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 1,
                role: "n".into(),
                speaker_id: Some(Uuid::from_u128(431))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 2,
                role: "g".into(),
                speaker_id: Some(Uuid::from_u128(412))
            },
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 2,
                role: "n".into(),
                speaker_id: Some(Uuid::from_u128(432))
            },
        ],
        vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.iter().map(|s| s.speaker).collect_vec(), vec![
        Some(Uuid::from_u128(410)),
        Some(Uuid::from_u128(420)),
        Some(Uuid::from_u128(411)),
        Some(Uuid::from_u128(421)),
        Some(Uuid::from_u128(430)),
        Some(Uuid::from_u128(431)),
        Some(Uuid::from_u128(432)),
        Some(Uuid::from_u128(422)),
        Some(Uuid::from_u128(412)),
    ]);
    assert_eq!(ballot.speeches.iter().map(|s| s.position).collect_vec(), vec![0, 0, 1, 1, 0, 1, 2, 2, 2]);
    assert_eq!(ballot.adjudicators, vec![]);
    assert!(ballot.president.is_none());

    Ok(())
}


#[test]
fn test_get_ballot_missing_speeches() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![],
        vec![],
        vec![
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 2,
                role: "g".into(),
                speaker_id: Some(Uuid::from_u128(410))
            },
        ],
        vec![])?;

    assert!(ballot.government.team.is_none());
    assert!(ballot.opposition.team.is_none());
    assert_eq!(ballot.speeches.iter().map(|s| s.speaker).collect_vec(), vec![
        Some(Uuid::from_u128(410)),
    ]);
    assert_eq!(ballot.speeches.iter().map(|s| s.position).collect_vec(), vec![2]);
    assert_eq!(ballot.adjudicators, vec![]);
    assert!(ballot.president.is_none());

    Ok(())
}

#[test]
fn test_get_ballot_speech_scores() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![],
        vec![],
        vec![],
        vec![
            schema::ballot_speech::Model {
                ballot_id: Uuid::from_u128(100),
                position: 0,
                role: "g".into(),
                speaker_id: Some(Uuid::from_u128(410))
            },
        ],
        vec![
            schema::adjudicator_speech_score::Model {
                ballot_id:Uuid::from_u128(100),
                adjudicator_id: Uuid::from_u128(301),
                speech_role: "g".into(), speech_position: 0, manual_total_score: Some(72) }
        ])?;

    assert_eq!(ballot.speeches[0].scores, HashMap::from_iter(vec![(Uuid::from_u128(301), SpeakerScore::Aggregate(72))].into_iter()));

    Ok(())
}

#[test]
fn test_get_ballot_team_scores() -> Result<(), BallotParseError> {
    let ballot = Ballot::from_rows(
        schema::ballot::Model {
            uuid: Uuid::from_u128(100),
        },
        vec![
            schema::ballot_team::Model { ballot_id: Uuid::from_u128(100),role:"g".into(), team_id: None },
        ],
        vec![],
        vec![
            schema::adjudicator_team_score::Model { adjudicator_id: Uuid::from_u128(301), ballot_id: Uuid::from_u128(100), role_id: "g".into(), manual_total_score: Some(32) }
        ],
        vec![],
        vec![])?;

    assert_eq!(ballot.government.scores, HashMap::from_iter(vec![(Uuid::from_u128(301), TeamScore::Aggregate(32))].into_iter()));

    Ok(())
}
