//@ts-check

import React, { useCallback, useContext } from "react";
import { useState, useMemo } from "react";
import { executeAction } from "./Action";
import { getPath, useView } from "./View";
import { open } from '@tauri-apps/api/dialog';
import { TournamentContext } from "./TournamentContext";

import ModalOverlay from "./Modal";
import { useParams } from "react-router";
import Button from "./Button";

const ROLE_COLORS = {
    "government": "bg-green-200",
    "opposition": "bg-orange-200",
    "non_aligned": "bg-purple-200",
  }

function formatScore(score) {
    let style = new Intl.NumberFormat('de-DE', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    return score !== null ? style.format(score) : "-";
}


function ScoreOverview(props) {
    return <div>
        <div className="flex">
            <div className="flex-1 text-center">
                <h2>{props.ballot.government.name || "-"}</h2>
                {formatScore(props.ballot.government.total_score)}
            </div>
            <div className="flex-1 text-center">
                <h2>{props.ballot.opposition.name || "-"}</h2>
                {formatScore(props.ballot.opposition.total_score)}
            </div>
        </div>

        <div className="flex text-center text-sm flex-wrap items-end">
            {props.ballot.speeches.map((speech, idx) => <div key={idx} className="flex-1">
                <h3>{speech.speaker ? speech.speaker.name : "-"}</h3>
                {formatScore(speech.total_score)}
            </div>)}
        </div>
    </div>
}

function BackupBallotList(props) {
    return <div className="border-t">
        {props.backup_ballots.map((backup_ballot) =>
        <div>
            <h2>{backup_ballot.name}</h2> 
            <ScoreOverview key={backup_ballot.uuid} ballot={backup_ballot.ballot} />
        </div>
    )}
    </div>
}

function DebateResultCard(props) {
    let [showBackupBallots, setShowBackupBallots] = useState(false);

    let backupBallots = props.debate.backup_ballots.filter(ballot => ballot.ballot.uuid != props.debate.ballot.uuid);
    let numBackupBallots = backupBallots.length;

    return <div className="overflow-hidden sm:rounded-lg border m-2 p-1">
        <h1 className="text-center">{props.debate.name}</h1>

        {props.debate.ballot ? <ScoreOverview ballot={props.debate.ballot} /> : "Missing Ballot"}

        <div className="text-center text-sm">
            <button onClick={() => props.onStartEditDebateBallot(props.debate.uuid, props.debate.ballot)}>Edit ballot…</button>
        </div>
        <div className="text-center text-sm">
            <button disabled={numBackupBallots == 0} onClick={() => setShowBackupBallots(!showBackupBallots)}>
                {numBackupBallots > 0 ? `${showBackupBallots ? "Hide" : "Show"} ${numBackupBallots} other ballot${numBackupBallots > 1 ? "s" : ""}…` : "No Backup Ballots" }
            </button>

            {showBackupBallots && <BackupBallotList backup_ballots={backupBallots} />}
        </div>
   </div>
}

function BallotEditor(props) {
    let [ballot, setBallot] = useState(props.initialBallot);
    return <div className="">
        <div><BallotEditTable ballot={ballot} onBallotChanged={(ballot) => setBallot(ballot)} />
        </div>
        <div className="flex justify-end">
            <Button role="secondary" onClick={props.onAbort}>Abort</Button>
            <Button role="primary" onClick={() => props.onSave(ballot)}>Save</Button>
        </div>
    </div>
}

function BallotEditTable(props) {
     return <table className="w-full">
        <thead>
            <tr>
                <th>Speaker</th>
                {props.ballot.adjudicators.map(
                    (adj) => <th key={adj.uuid}>{adj.name}</th>
                )}
                <th>⌀</th>
                <th>Total</th>
            </tr>
        </thead>
        <tbody>
            {
                props.ballot.speeches.map(
                    (speech, idx) => {
                        let speakerChoices = null;

                        if (speech.role == "government") {    
                            speakerChoices = props.ballot.government.members;
                        }
                        else if (speech.role == "opposition") {
                            speakerChoices = props.ballot.opposition.members;
                        }
                        return <BallotSpeechRow key={idx} speech={speech} adjudicators={props.ballot.adjudicators} speakerChoices={speakerChoices} onSpeechChanged={
                            (newSpeech) => {
                                let newBallot = {...props.ballot, speeches: [...props.ballot.speeches]};
                                newBallot.speeches[idx] = newSpeech;

                                if (newSpeech.speaker?.uuid !== props.ballot.speeches[idx].speaker?.uuid && newSpeech.speaker?.uuid !== null) {
                                    let prevIdx = props.ballot.speeches.findIndex(
                                        (s) => s.speaker?.uuid === newSpeech.speaker.uuid
                                    );

                                    if (prevIdx != -1) {
                                        newBallot.speeches[prevIdx].speaker = props.ballot.speeches[idx].speaker;
                                    }
                                }

                                if (speech.role == "government" || speech.role == "opposition") {
                                    let speechScores = newBallot.speeches.filter(
                                        (s) => s.role == speech.role && s.total_score !== null
                                    ).map(
                                        (s) => s.total_score
                                    );
                                    let totalSpeechScore = null;
                                    if (speechScores.length > 0) {
                                        totalSpeechScore = speechScores.reduce(
                                            (a, b) => a + b, 0
                                        );
                                    }

                                    newBallot[speech.role].total_speech_score = totalSpeechScore;
                                    let totalTeamScore = newBallot[speech.role].total_team_score;
                                    if (totalTeamScore !== null || totalSpeechScore !== null) {
                                        newBallot[speech.role].total_score = (totalSpeechScore !== null ? totalSpeechScore : 0) + (totalTeamScore !== null ? totalTeamScore : 0);
                                    }
                                    else {
                                        newBallot[speech.role].total_score = null;
                                    }
                                }

                                props.onBallotChanged(newBallot);
                            }
                        } />
                    }
                )
            }
            <BallotTeamRow team={props.ballot.government} adjudicators={props.ballot.adjudicators} onTeamChanged={
                (newTeam) => {
                    let newBallot = {...props.ballot, government: newTeam};
                    props.onBallotChanged(newBallot);
                }
            } />
            <BallotTeamRow team={props.ballot.opposition} adjudicators={props.ballot.adjudicators} onTeamChanged={
                (newTeam) => {
                    let newBallot = {...props.ballot, opposition: newTeam};
                    props.onBallotChanged(newBallot);
                }
            } />
        </tbody>
    </table>
}

function BallotTeamRow(props) {
    return <tr>
        <td className="border p-1">
            {props.team.name}
        </td>
        {
            props.adjudicators.map((adj) => {
                let score = props.team.scores[adj.uuid];
                return <td key={adj.uuid} className="border p-1">
                    <ScoreInputCell score={score === undefined ? null : score} maxScore={200} onChange={
                        (newScore) => {
                            let newTeam = {...props.team, scores: {...props.team.scores}};
                            if (newScore === null) {
                                delete newTeam.scores[adj.uuid];
                            }
                            else {
                                newTeam.scores[adj.uuid] = newScore;
                            }

                            let scoreValues = Object.values(newTeam.scores);
                            let totalTeamScore = null;
                            if (scoreValues.length > 0) {
                                totalTeamScore = scoreValues.reduce(
                                    (a, b) => a + b, 0
                                ) / scoreValues.length;
                            }
                            newTeam.total_team_score = totalTeamScore;
                            let totalScore = newTeam.total_speech_score;

                            if (totalScore !== null || totalTeamScore !== null) {
                                newTeam.total_score = (totalScore !== null ? totalScore : 0) + (totalTeamScore !== null ? totalTeamScore : 0);
                            }

                            props.onTeamChanged(newTeam);
                        }
                    } />
                </td>;
            }
        )}
        <td className="border p-1 text-center min-w-[3em]">{formatScore(props.team.total_team_score)}</td>
        <td className="border p-1 text-center min-w-[3em]">{formatScore(props.team.total_score)}</td>
    </tr>;
}

function BallotSpeechRow(props) {
    return <tr>
        <td className="border p-1">
            {
                props.speakerChoices ? <SpeakerSelectBox speaker={props.speech.speaker?.uuid || null} choices={props.speakerChoices} onChange={
                    (newSpeaker) => {
                        let newSpeech = {...props.speech, speaker: newSpeaker};
                        props.onSpeechChanged(newSpeech);
                    }
                } /> : <span>{props.speech.speaker.name}</span>
            }
        </td>
        {
            props.adjudicators.map((adj) => {
                let score = props.speech.scores[adj.uuid];
                return <td key={adj.uuid} className="border p-1">
                    <ScoreInputCell score={score === undefined ? null : score} maxScore={100} onChange={(score) => {
                        let newSpeech = {...props.speech, scores: {...props.speech.scores}};
                        if (score === null) {
                            delete newSpeech.scores[adj.uuid];
                        }
                        else {
                            newSpeech.scores[adj.uuid] = score;
                        }
                    let newScores = Object.values(newSpeech.scores).filter((x) => x !== null);
                        let newTotal = newScores.length > 0 ? newScores.reduce((a, b) => a + b, 0) / newScores.length : null;

                        newSpeech.total_score = newTotal;

                        props.onSpeechChanged(
                            newSpeech
                        )
                    }} />
                </td>;
            })
        }
        <td className="border p-1 text-center min-w-[3em]">{formatScore(props.speech.total_score)}</td>
        <td className="border p-1 text-center min-w-[3em]"></td>
    </tr>
}


function SpeakerSelectBox(props) {
    let choices = props.choices.map(
        (choice) => <option key={choice.uuid} value={choice.uuid}>{choice.name}</option>
    );

    return <select className="appearance-none" value={props.speaker !== null ? props.speaker : ""} onChange={(evt) => {
        let value = evt.target.value;
        let selectedSpeaker = props.choices.find((choice) => choice.uuid === value);
        props.onChange(selectedSpeaker);
    }}>
        <option value={""} disabled={true}>Auswählen…</option>
        {choices}
    </select>
}


function ScoreInputCell(props) {
    return <input className="m-0" onChange={evt => {
        var value = parseInt(evt.target.value);
        if (evt.target.value === "" || isNaN(value)) {
            value = null
        }

        if (value > props.maxScore) {
            return
        }
        props.onChange(value);
    }} value={props.score !== null ? props.score : ""} />
}

export function RoundResultList(props) {
    let debates = useView({type: "RoundResults", round_uuid: props.roundId}, {"debates": []});
    let [activeBallot, setActiveBallot] = useState(null);
    console.log(activeBallot);
    return <div>
        <div className="p-4">
        {
            debates.debates.map((debate) => 
                <DebateResultCard key={debate.uuid} debate={debate} onStartEditDebateBallot={(debateId, initialValues) => setActiveBallot({"debateId": debateId, "initialBallot": initialValues})} />
            )
        }
        </div>

        <div className="w-[80%]"></div>

        <ModalOverlay open={activeBallot !== null} closeOnOverlayClick={false} onAbort={() => setActiveBallot(null)} windowClassName={"w-[80%]"}>
            {
                activeBallot !== null ? <BallotEditor
                    initialBallot={activeBallot.initialBallot}
                    onAbort={() => setActiveBallot(null)}
                    onSave={(ballot) => {
                        executeAction("UpdateScores", {
                            "debate_id": activeBallot.debateId,
                            "update": {"NewBallot": ballot}
                        });
                        setActiveBallot(null);
                    }}    
                /> : []
            }
        </ModalOverlay> 
    </div>
}

export function RoundResultRoute(props) {
    let { roundId } = useParams();
    return <RoundResultList roundId={roundId} />;
}