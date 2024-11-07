//@ts-check

import React, { useCallback, useContext, useEffect } from "react";
import { useState, useMemo } from "react";
import { executeAction } from "./Action";
import { getPath, useView } from "./View";
import { open } from '@tauri-apps/api/dialog';
import { TournamentContext } from "./TournamentContext";

import ModalOverlay from "./UI/Modal";
import { useParams } from "react-router";
import Button from "./UI/Button";

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
    let tournamentId = useContext(TournamentContext).uuid;
    return <div>
        {props.backup_ballots.map((backup_ballot) =>
        <div key={backup_ballot.uuid}>
            <h2>{backup_ballot.name}</h2> 
            <ScoreOverview key={backup_ballot.uuid} ballot={backup_ballot.ballot} />

            <div className="flex flex-col">
            <button onClick={
                () =>
                    executeAction("UpdateScores", {
                        "debate_id": props.debateId,
                        "update": {"SetBallot": backup_ballot.uuid}
                    })
                
            }>
                Make Primary
            </button>

            {
                props.isPendingList ? <button onClick={
                    () =>
                        executeAction("DiscardBallot", {
                            "tournament_id": tournamentId,
                            "backup_ballot_id": backup_ballot.uuid
                        })
                    
                }>
                    Discard
                </button> : []
            }
            </div>
        </div>
    )}
    </div>
}

function checkBallotHasScores(ballot) {
    return ballot.speeches.some(
        (speech) => Object.keys(speech.scores).length > 0
    );
}

function DebateResultCard(props) {
    let [showIgnoredBallots, setShowIgnoredBallots] = useState(false);

    let bgColor = "bg-gray-100";
    console.log(props.debate.ballot);
    if (props.debate.pending_ballots.length > 0) {
        bgColor = "bg-yellow-100";
    }
    else if (checkBallotHasScores(props.debate.ballot)) {
        bgColor = "bg-green-100";
    }

    return <div className={`overflow-hidden sm:rounded-lg border m-2 p-1 ${bgColor}`}>
        <h1 className="text-center">{props.debate.name}</h1>

        {props.debate.ballot ? <ScoreOverview ballot={props.debate.ballot} /> : "Missing Ballot"}

        <div className="text-center text-sm">
            <button onClick={() => props.onStartEditDebateBallot(props.debate.uuid, props.debate.ballot)}>Edit ballot…</button>
        </div>
        {props.debate.pending_ballots.length > 0 ? 
        <div className="text-center text-sm border-t border-b">
            <h3>Pending Ballots</h3>
            <BackupBallotList debateId={props.debate.uuid} backup_ballots={props.debate.pending_ballots} isPendingList={true} />
        </div> : []
        }
        <div className="text-center text-sm">
        {props.debate.ignored_ballots.length > 0 ? <button onClick={() => setShowIgnoredBallots(!showIgnoredBallots)}>
                {props.debate.ignored_ballots.length > 0 ? `${showIgnoredBallots ? "Hide" : "Show"} ${props.debate.ignored_ballots.length} discarded ballot${props.debate.ignored_ballots.length > 1 ? "s" : ""}…` : "" }
            </button> : []}

            {showIgnoredBallots && <BackupBallotList debateId={props.debate.uuid} backup_ballots={props.debate.ignored_ballots} />}
        </div>
   </div>
}

function BallotEditor(props) {
    let [ballot, setBallot] = useState(props.initialBallot);
    let [disableRepetitionConstraints, setDisableRepetitionConstraints] = useState(false);

    useEffect(
        () => {
            setDisableRepetitionConstraints(props.initialBallot.speeches.some(s => s.is_opt_out));
        },
        [props.initialBallot]
    )

    return <div className="">
        <div><BallotEditTable ballot={ballot} onBallotChanged={(ballot) => setBallot(ballot)} disableRepetitionConstraints={disableRepetitionConstraints} />
        </div>
        <div className="flex justify-end">
            <Button role="secondary" onClick={props.onAbort}>Abort</Button>
            <Button role="primary" onClick={() => props.onSave(ballot)}>Save</Button>
        </div>

        <input type="checkbox" checked={disableRepetitionConstraints} onChange={(evt) => setDisableRepetitionConstraints(evt.target.checked)} />
        <label>Disable one speech per speaker constraint (allows opt-out)</label>
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
                {props.disableRepetitionConstraints ? <th>OptOut?</th> : []}
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
                        return <BallotSpeechRow
                            key={idx}
                            speech={speech}
                            adjudicators={props.ballot.adjudicators}
                            speakerChoices={speakerChoices}
                            disableRepetitionConstraints={props.disableRepetitionConstraints}
                            onSpeechChanged={
                            (newSpeech) => {
                                let newBallot = {...props.ballot, speeches: [...props.ballot.speeches]};
                                newBallot.speeches[idx] = newSpeech;

                                if (!props.disableRepetitionConstraints && newSpeech.speaker?.uuid !== props.ballot.speeches[idx].speaker?.uuid && newSpeech.speaker?.uuid !== null) {
                                    let prevIdx = props.ballot.speeches.findIndex(
                                        (s) => s.speaker?.uuid === newSpeech.speaker.uuid
                                    );

                                    if (prevIdx != -1) {
                                        newBallot.speeches[prevIdx].speaker = props.ballot.speeches[idx].speaker;
                                    }
                                }

                                if (speech.role == "government" || speech.role == "opposition") {
                                    let otherSpeeches = newBallot.speeches.map((s, idx) => [idx, s]).filter(
                                        s => s[1].role == speech.role
                                    );
                                    
                                    let speechScores = otherSpeeches.map(
                                        s => s[1].total_score
                                    );

                                    console.log(otherSpeeches);
                                    let missingSpeakerSpeeches = otherSpeeches.filter(s => s[1].speaker == null);
                                    console.log(missingSpeakerSpeeches);
                                    if (missingSpeakerSpeeches.length == 1) {
                                        let allTeamSpeakers = [...new Set(otherSpeeches.map(s => s[1].speaker?.uuid))];
                                        let availableSpeakers = speakerChoices.filter(s => !allTeamSpeakers.includes(s.uuid));
                                        if (availableSpeakers.length == 1) {
                                            newBallot.speeches[missingSpeakerSpeeches[0][0]].speaker = availableSpeakers[0];
                                        }
                                    }

                                    let totalSpeechScore = null;
                                    if (speechScores.length > 0) {
                                        totalSpeechScore = speechScores.filter(s => s != null).reduce(
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
                return <td key={adj.uuid} className="border p-1 text-center">
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
        {
            props.disableRepetitionConstraints && <td><input type="checkbox" tabIndex={-1} checked={props.speech.is_opt_out} onChange={
                (evt) => {
                    let newSpeech = {...props.speech, is_opt_out: evt.target.checked};
                    props.onSpeechChanged(newSpeech);
                }
            } /> </td>
        }
    </tr>
}


function SpeakerSelectBox(props) {
    let choices = props.choices.map(
        (choice) => <option key={choice.uuid} value={choice.uuid}>{choice.name}</option>
    );

    return <select tabIndex={-1} className="appearance-none" value={props.speaker !== null ? props.speaker : ""} onChange={(evt) => {
        let value = evt.target.value;
        let selectedSpeaker = props.choices.find((choice) => choice.uuid === value);
        props.onChange(selectedSpeaker);
    }}>
        <option value={""} disabled={true}>Auswählen…</option>
        {choices}
    </select>
}


function ScoreInputCell(props) {
    return <input className="m-0 w-full text-center" onChange={evt => {
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
    return <div className="w-full h-full overflow-auto justify-center">
        <div className="p-4">
        {
            debates.debates.map((debate) => 
                <DebateResultCard key={debate.uuid} debate={debate} onStartEditDebateBallot={(debateId, initialValues) => setActiveBallot({"debateId": debateId, "initialBallot": initialValues})} />
            )
        }
        </div>

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