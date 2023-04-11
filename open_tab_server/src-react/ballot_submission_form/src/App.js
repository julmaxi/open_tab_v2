import logo from './logo.svg';
import './App.css';
import React, { useEffect, useState } from 'react';
import { Routes, Route, useParams } from 'react-router-dom';


const ROLE_COLORS = {
  "government": "bg-green-200",
  "opposition": "bg-orange-200",
  "non_aligned": "bg-purple-200",
}


function ScoreInput(props) {
  let format = new Intl.NumberFormat('en-IN', { maximumFractionDigits: 2, minimumFractionDigits: 2 });
  let className = "w-full h-full text-center min-w-[44px] min-h-[44px] m-0 border-none appearance-none";

  if (props.readOnly) {
    className += " " + ROLE_COLORS[props.role];
  }

  return <input
    type={props.readOnly ? "text" : "number"}
    className={className}
    value={props.value === null ? "" : (props.readOnly ? format.format(props.value) : props.value)}
    readOnly={props.readOnly}
    placeholder={props?.adjudicator?.name ? getInitials(props.adjudicator.name) : "-"}
    onChange={(e) => {
      if (e.target.value === "") {
        props.onChange(null);
      }
      else {
        let val = parseInt(e.target.value);
        if (val !== undefined && !isNaN(val) && val >= 0 && val <= props.maxVal) {
          props.onChange(val);
        }
      }
    }}>
  </input>
}


function SpeakerSelector(props) {
  return (
    <select className='appearance-none' value={props.selected?.uuid || ""} onChange={(event) => {
      props.onChange(event.target.value);
    }}>
      <option disabled={true} value="">Auswählen…</option>
      {Object.values(props.options).sort((a, b) => a.name.localeCompare(b.name)).map(
        (speaker, speaker_idx) => <option key={speaker_idx} value={speaker.uuid}>{speaker.name}</option>
      )}
    </select>
  );
}

function getInitials(name) {
  let parts = name.split(" ");
  let first_part = parts[0];
  if (parts.length > 1) {
    let last_part = parts[parts.length - 1];
    return first_part[0] + "." + last_part[0] + ".";
  }
  return first_part[0] + ".";
}

function ScoreRow(props) {
  return (
    <tr>
      {
        props.scores.map(
          (score, idx) => <td className='border' key={idx}>
            <ScoreInput
              value={score}
              adjudicator={props.adjudicators[idx]}
              onChange={
                (new_score) => {
                  props.onChange(idx, new_score);
                }
              }
              maxVal={props.maxVal}
            />
          </td>
        )
      }
      <td><ScoreInput readOnly={true} value={props.total} role={props.role} /></td>
    </tr>
  );
}

function BallotForm(props) {
  let speaker_lookup = {};

  for (let team of [props.ballot.government, props.ballot.opposition]) {
    for (let speaker of Object.values(team.members)) {
      speaker_lookup[speaker.uuid] = speaker;
    }
  }

  return (
    <div>
      <form>
        <table className='sm:min-w-full text-lg'>
          <tbody>
            {props.ballot.speeches.map(
              (speech, speech_idx) => {
                return [
                  <tr
                    key={`speech_${speech_idx}_header`}
                    className={`${ROLE_COLORS[speech.role]}`}
                  >
                    <td colSpan={props.ballot.adjudicators.length + 1} className='p-2'>
                      {
                        speech.role !== "non_aligned" ?
                          <SpeakerSelector options={
                            speech.role === "government" ? props.ballot.government.members : props.ballot.opposition.members
                          } onChange={
                            (new_speaker_uuid) => {
                              let prev_speaker_speech_idx = props.ballot.speeches.findIndex(
                                (s) => s.speaker?.uuid === new_speaker_uuid
                              );

                              let new_speeches = [...props.ballot.speeches];
                              if (prev_speaker_speech_idx !== -1) {
                                let prev_speaker_speech = props.ballot.speeches[prev_speaker_speech_idx];
                                let new_prev_speaker_speech = {...prev_speaker_speech, speaker: null};
                                new_speeches[prev_speaker_speech_idx] = new_prev_speaker_speech;
                              }

                              let new_speech = {...speech, speaker: speaker_lookup[new_speaker_uuid]};
                              new_speeches[speech_idx] = new_speech;
                              let new_ballot = {...props.ballot, speeches: new_speeches};
                              console.log(new_ballot);
                              props.onBallotChanged(new_ballot)
                            }
                          } selected= {
                            speech.speaker
                          }/>
                          :
                          <span>{speech?.speaker?.name || "Fehlt"}</span>
                      }
                    </td>
                  </tr>,
                  <ScoreRow
                    key={`speech_${speech_idx}_scores`}
                    scores={props.ballot.adjudicators.map((adj) => speech.scores[adj.uuid] || null)}
                    total={speech.total || null}
                    adjudicators={props.ballot.adjudicators}
                    onChange={
                      (adj_idx, new_score) => {
                        let speech = props.ballot.speeches[speech_idx];
                        let new_speech = {...speech, scores: {...speech.scores, [props.ballot.adjudicators[adj_idx].uuid]: new_score}};
                        let new_speeches = [...props.ballot.speeches];
                        new_speeches[speech_idx] = new_speech;
                        let new_ballot = {...props.ballot, speeches: new_speeches};
                        props.onBallotChanged(new_ballot)
                      }
                    }
                    role={speech.role}
                    maxVal={100}
                  />
                ]
              }
            )}
            {
              ["government", "opposition"].map(
                (role) => {
                  return [<tr key={`${role}_header`}>
                    <td colSpan={props.ballot.adjudicators.length + 1}>
                      {`${role === "government" ? "Reg:" : "Opp:"} ${props.ballot[role].name}`}
                      </td>
                    </tr>,

                    <ScoreRow
                      key={`${role}_scores`}
                      scores={props.ballot.adjudicators.map((adj) => props.ballot[role].scores[adj.uuid] || null)}
                      total={props.ballot[role].total || null}
                      adjudicators={props.ballot.adjudicators}
                      onChange={
                        (adj_idx, new_score) => {
                          let new_role = {...props.ballot[role], scores: {...props.ballot[role].scores, [props.ballot.adjudicators[adj_idx].uuid]: new_score}};
                          let new_ballot = {...props.ballot, [role]: new_role};
                          props.onBallotChanged(new_ballot);
                        }
                      }
                      maxVal={200}
                    />
                  ]
                }
              )
            }
          </tbody>
        </table>
      </form>
    </div>
  );
}

function computeScoreAverage(scores) {
  let num_scores = Object.values(scores).filter((s) => s !== null).length;
  return num_scores > 0 ? Object.values(scores).reduce((a, b) => a + b, 0) / num_scores : null;
}

function updateBallotTotalScores(ballot) {
  for (let speech of ballot.speeches) {
    speech.total = computeScoreAverage(speech.scores);
  }

  ballot.government.total = computeScoreAverage(ballot.government.scores);
  ballot.opposition.total = computeScoreAverage(ballot.opposition.scores);

  let gov_speech_scores = ballot.speeches.filter((s) => s.role === "government").reduce((a, b) => a + (isFinite(b.total) ? b.total : 0), 0);
  let opp_speech_scores = ballot.speeches.filter((s) => s.role === "opposition").reduce((a, b) => a + (isFinite(b.total) ? b.total : 0), 0);

  ballot.government.final_score = ballot.government.total + gov_speech_scores;
  ballot.opposition.final_score = ballot.opposition.total + opp_speech_scores;
}

function BallotEditor(props) {
  /*let ballotTemplate = {
    uuid: "1234",
    adjudicators: [
      {uuid: "a1", name: "Tim Tom"},
      {uuid: "a2", name: "Art Sigil"},
      {uuid: "a3", name: "Biggus Dickus"},
      {uuid: "a4", name: "Incontinentia"},
      {uuid: "a5", name: "Person with a very long Name"},
    ],

    speeches: [
      {uuid: "s1", speaker: null, role: "government", position: 0, scores: {}},
      {uuid: "s2", speaker: null, role: "opposition", position: 0, scores: {}},
      {uuid: "s3", speaker: null, role: "government", position: 1, scores: {}},
      {uuid: "s4", speaker: null, role: "opposition", position: 1, scores: {}},
      {uuid: "s5", speaker: {uuid: "sp7", name: "Non aligned 1"}, role: "non_aligned", position: 0, scores: {}},
      {uuid: "s6", speaker: {uuid: "sp8", name: "Non aligned 2"}, role: "non_aligned", position: 1, scores: {}},
      {uuid: "s7", speaker: {uuid: "sp9", name: "Non aligned 3"}, role: "non_aligned", position: 2, scores: {}},
      {uuid: "s8", speaker: null, role: "opposition", position: 2, scores: {}},
      {uuid: "s9", speaker: null, role: "government", position: 2, scores: {}},
    ],

    government: {
      uuid: "t1",
      name: "Team 1",
      members: {
        "sp1": {uuid: "sp1", name: "Speaker 1"},
        "sp2": {uuid: "sp2", name: "Speaker 2"},
        "sp3": {uuid: "sp3", name: "Speaker 3"},
      },
      scores: {},
    },
    opposition: {
      uuid: "t2",
      name: "Team 2",
      members: {
        "sp4": {uuid: "sp4", name: "Speaker 4"},
        "sp5": {uuid: "sp5", name: "Speaker 5"},
        "sp6": {uuid: "sp6", name: "Speaker 6"},
      },
      scores: {}
    },
  }*/

  let [ballot, setBallot] = React.useState(() => {
    let ballot = structuredClone(props.initialBallot);
    updateBallotTotalScores(ballot);
    return ballot;
  });

  let number_format = new Intl.NumberFormat('en-IN', { maximumFractionDigits: 2, minimumFractionDigits: 2 });

  return (
    <div>
      <BallotForm ballot={ballot} onBallotChanged={
        (new_ballot) => {
          updateBallotTotalScores(new_ballot);
          setBallot(new_ballot);
        }
      } />
      <div className='flex'>
        {
          ["government", "opposition"].map(
            (role) => {
              let className = "flex-1 text-center text-lg font-bold p-2";

              className += " " + ROLE_COLORS[role];
              console.log(ballot)
              return <div key={`${role}_total`} className={className}>
                {ballot[role].final_score !== null ? number_format.format(ballot[role].final_score) : "-"}
              </div>
            }
          )
        }
      </div>
      <div className='flex'>
        <button className='flex-1 bg-green-500 text-white font-bold text-lg p-2' onClick={() => props.onSubmit(ballot)}>Submit</button>
      </div>
    </div>
  );
}

function Spinner(props) {
  return <div role="status">
      <svg aria-hidden="true" className="w-8 h-8 mr-2 text-gray-200 animate-spin dark:text-gray-600 fill-blue-600" viewBox="0 0 100 101" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z" fill="currentColor"/>
          <path d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z" fill="currentFill"/>
      </svg>
      <span className="sr-only">Loading...</span>
  </div>
  
}

const HOST = "http://localhost:8000/"

export function BallotRoute(props) {
  let { debateId } = useParams();

  let [ballot, setBallot] = useState(null);
  
  useEffect(() => {
    async function fetchData() {
      let response = await fetch(`${HOST}/api/v1/debate/${debateId}/ballot`);
      let ballot = await response.json();

      setBallot(ballot);
    }

    fetchData();
  }, [debateId])

  return ballot === null ? <Spinner /> : <BallotEditor onSubmit={ballot => {
    async function submitBallot() {
      let response = await fetch(`${HOST}/api/v1/debate/${debateId}/ballots`, {
        method: "POST",
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(ballot),
      });
      let values = await response.json();
      console.log(values);
    }
    
    submitBallot();
  }} initialBallot={ballot} />;
}

export default BallotEditor;
