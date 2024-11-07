import React, {useContext, useEffect, useState} from "react";
import {useParams} from "react-router-dom";

import {TournamentContext} from "./TournamentContext";


import {useView} from "./View";
import { executeAction } from "./Action";
import ModalOverlay from "./UI/Modal";
import Button from "./UI/Button";
import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { DateTimeSelectorButton } from "./UI/DateTimeSelectorButton";
import TextField from "./UI/TextField";

function RoundStatusBarButton({name, releaseTime, onSetReleaseTime, position, numElements}) {
    let [refeshVal, setRefresh] = useState(0);
    let now = new Date();
    let isReleased = releaseTime !== null && now > releaseTime;

    useEffect(() => {
        if (!isReleased && releaseTime !== null) {
            let timeout = setTimeout(() => {
                setRefresh(refeshVal + 1)
            }, (now - releaseTime) + 1.0);
            return () => {
                clearTimeout(timeout);
            }
        }
    }, [isReleased, releaseTime, refeshVal]);

    //let bg_colors = {"release": "bg-blue-500", "scheduled": "bg-blue-200", "unscheduled": "bg-gray-200"};
    let bg_colors = {
        "release": "rgb(59, 130, 246)",
        "scheduled": "rgb(191, 219, 254)",
        "unscheduled": "rgb(229, 231, 235)"
    }

    let color = isReleased ? bg_colors["release"] : (releaseTime === null ? bg_colors["unscheduled"] : bg_colors["scheduled"]);

    let isScheduledForToday = releaseTime !== null && (releaseTime.getDate() === now.getDate() && releaseTime.getMonth() === now.getMonth() && releaseTime.getFullYear() === now.getFullYear());
    let isLast = position === numElements - 1;
    let isFirst = position === 0;

    let dateOptions = {
        dateStyle: 'short',
        timeStyle: 'short'
    };
    
    if (isScheduledForToday) {
        dateOptions = {
            timeStyle: 'short'
        };
    }
    let dateFormat = new Intl.DateTimeFormat('en-GB', { ...dateOptions })

    return <div
        className={`p-2 text-center min-w-48 relative`}
        style={
            {
                //clipPath,
                marginTop: !isFirst ? "-10px" : "",
                zIndex: numElements - position,
                color: isReleased ? "white" : "black"
            }
        }
    >
        <svg className="absolute top-0 left-0" width="100%" height="100%" viewBox="0 0 100 100" preserveAspectRatio="none" xmlns="http://www.w3.org/2000/svg" style={{filter: !isLast ? "drop-shadow( 0px 0px 2px rgba(0, 0, 0, .7))" : "", zIndex: -1}}>
            {!isLast ? <path d="M 0 0 H 100 V 90 L 50 100 L 0 90 V 0" stroke="transparent" fill={color}/> : <rect x="0" y="0" width="100" height="100" fill={color} />}
        </svg>

        <div>
            {name}
            
            <div>
                {isReleased ? <span>Released</span> : (
                    releaseTime === null ?
                        <span>Not Scheduled</span> : <span>Scheduled at {dateFormat.format(releaseTime)}</span>
                )}
            </div>

            {
                releaseTime === null ? <DateTimeSelectorButton label={"Release…"} onSetDate={(date) => {
                    onSetReleaseTime(date);
                }} />
                :
                <button onClick={() => onSetReleaseTime(null)}>
                    Undo Release
                </button>
            }
        </div>
    </div>
}



function RoundStatusBar({drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime, silentRoundResultsReleaseTime, isSilent, onSetReleaseTime}) {
    [drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime, silentRoundResultsReleaseTime] = [drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime, silentRoundResultsReleaseTime].map((s) => s === null ? s : new Date(s + "+00:00"));
    let states = [
        {
            "key": "drawReleaseTime",
            "name": "Draw Release",
            "releaseTime": drawReleaseTime,
        },
        {
            "key": "teamMotionReleaseTime",
            "name": "Motion Release to Teams",
            "releaseTime": teamMotionReleaseTime,
        },
        {
            "key": "debateStartTime",
            "name": "Debate Start",
            "releaseTime": debateStartTime,
        },
        {
            "key": "fullMotionReleaseTime",
            "name": "Motion Release to All",
            "releaseTime": fullMotionReleaseTime,
        },
        {
            "key": "roundCloseTime",
            "name": "Round Close",
            "releaseTime": roundCloseTime,
        },
        {
            "key": "feedbackReleaseTime",
            "name": "Feedback Release",
            "releaseTime": feedbackReleaseTime,
        }
    ];

    if (isSilent) {
        states.push({
            "key": "silentRoundResultsReleaseTime",
            "name": "Results Release",
            "releaseTime": silentRoundResultsReleaseTime,
        });
    }

    return <div className="flex flex-col w-full overflow-hidden rounded">
        {states.map((state, index) => {
            return <RoundStatusBarButton key={state.key} name={state.name} releaseTime={state.releaseTime} onSetReleaseTime={
                (date) => onSetReleaseTime(state.key, date)
            } position={index} numElements={states.length} />
        })}
    </div>
}

function EditMotionPanel({motion, info_slide, onChange}) {
    let [isEditing, setIsEditing] = useState(false);

    return <div className="mb-2 mt-2">
            <h2 className="font-bold">Motion</h2>
            { motion === null ? <p className="italic text-red-500">This round has no motion yet</p> : <p>{motion}</p> }
            {
                info_slide === null ?  [] :
                <>
                    <h2 className="font-bold">Info Slide</h2>
                    <p>{info_slide}</p>
                </>
            }

            <Button role="secondary" onClick={
                () => {
                    setIsEditing(true);
                }
            }>Edit Motion…</Button>
            <ModalOverlay open={isEditing} closeOnOverlayClick={false} onAbort={() => setIsEditing(false)} windowClassName={"w-[80%]"}>
                <EditMotionForm motion={motion} infoSlide={info_slide} onChange={(motion, info_slide) => {
                    onChange(motion, info_slide);
                    setIsEditing(false);
                }} />
            </ModalOverlay> 
    </div>
}

export function EditMotionForm({motion, infoSlide, onChange}) {
    let [newMotion, setNewMotion] = useState(null);
    let [newInfoSlide, setNewInfoSlide] = useState(null);

    return <div>
        <h2>Motion</h2>
        <TextField value={newMotion !== null ? newMotion : (motion || "")} onChange={(evt) => {
            setNewMotion(evt.target.value);
        }} />

        <h2>Info Slide</h2>
        <TextField value={newInfoSlide !== null ? newInfoSlide : (infoSlide || "")} onChange={(evt) => {
            setNewInfoSlide(evt.target.value);
        }} area={true} />

        <Button onClick={
            () => {
                onChange(newMotion !== null ? newMotion : (motion || ""), newInfoSlide !== null ? newInfoSlide : (infoSlide || ""));
            }
        } role="primary">Save</Button>
    </div>
}

export function RoundPublicationView({roundId}) {
    let currentView = {type: "RoundPublication", round_uuid: roundId};

    let publicationInfo = useView(currentView, null);

    if (publicationInfo === null) {
        return <div>Loading…</div>
    }

    return <div className="ml-auto mr-auto overflow-scroll max-w-xl">
        <h1 className="text-2xl font-bold">Publication Settings for Round {publicationInfo.index + 1}</h1>
        <EditMotionPanel motion={publicationInfo.motion} info_slide={publicationInfo.info_slide} onChange={
            (motion, info_slide) => {
                executeAction("UpdateRound", {update: {motion: motion, info_slide: info_slide}, round_id: roundId});
            }
        } />

        <div className="mb-2 mt-2">
            <input
                type="checkbox"
                className="w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                checked={publicationInfo.is_silent}
                onChange={(event) => {
                    let val = event.target.checked;
                    executeAction("UpdateRound", {update: {is_silent: val}, round_id: roundId});
                }}
            />
            <label className="ml-2 text-sm font-medium text-gray-900">Silent Round</label>
            <p><em className="text-gray-700 text-sm">{publicationInfo.is_silent ? "Results of this round will not appear in the tab until after the scheduled result release time. Teams will not be asked for feedback in this round." : "Results of this round will be visible in the tab after the round has been closed. Teams will be asked to give adjudicator feedback."}</em></p>
        </div>

        <div className="mb-2 mt-2">
            <Button role="secondary" onClick={
                () => {
                    open({directory: true}).then((result) => {
                        invoke("save_round_files", {roundId: roundId, dirPath: result}).then((result) => {
                            console.log(result);
                        });
                    });
                }
            }>Export Ballots/Presentation…</Button>
        </div>

        <div>
            <RoundStatusBar
                drawReleaseTime={publicationInfo.draw_release_time}
                teamMotionReleaseTime={publicationInfo.team_motion_release_time}
                debateStartTime={publicationInfo.debate_start_time}
                fullMotionReleaseTime={publicationInfo.full_motion_release_time}
                roundCloseTime={publicationInfo.round_close_time}
                feedbackReleaseTime={publicationInfo.feedback_release_time}
                silentRoundResultsReleaseTime={publicationInfo.silent_round_results_release_time}
                isSilent={publicationInfo.is_silent}
                onSetReleaseTime={(key, date) => {
                    let update = {};
                    key = key.replace(/([A-Z])/g, "_$1").toLowerCase();
                    update[key] = date === null ? null : date.toISOString().slice(0, -1);
                    executeAction("UpdateRound", {update: update, round_id: roundId});
                }}
            />
        </div>
    </div>
}

export function RoundPublicationRoute(props) {
    let { roundId } = useParams();
    return <RoundPublicationView roundId={roundId} />;
}