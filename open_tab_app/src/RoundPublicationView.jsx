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

function RoundStatusBarButton({name, releaseTime, onSetReleaseTime}) {
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

    let bg_colors = {"release": "bg-blue-500", "scheduled": "bg-blue-200", "unscheduled": "bg-gray-200"};

    let color = isReleased ? bg_colors["release"] : (releaseTime === null ? bg_colors["unscheduled"] : bg_colors["scheduled"]);

    let isScheduledForToday = releaseTime !== null && (releaseTime.getDate() === now.getDate() && releaseTime.getMonth() === now.getMonth() && releaseTime.getFullYear() === now.getFullYear());

    let dateOptions = {
        dateStyle: 'full',
        timeStyle: 'long'
    };
    
    if (isScheduledForToday) {
        dateOptions = {
            timeStyle: 'short'
        };
    }


    let dateFormat = new Intl.DateTimeFormat('en-GB', { ...dateOptions })
    return <div className={`${color} p-2 text-center`}>
        {name}

        <div>
        {isReleased ? <span>Since {dateFormat.format(releaseTime)}</span> : (
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
}



function RoundStatusBar({drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime, onSetReleaseTime}) {
    [drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime] = [drawReleaseTime, teamMotionReleaseTime, debateStartTime, fullMotionReleaseTime, roundCloseTime, feedbackReleaseTime].map((s) => s === null ? s : new Date(s + "+00:00"));
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

    return <div className="flex flex-row w-full">
        {states.map((state, index) => {
            return <RoundStatusBarButton key={state.key} name={state.name} releaseTime={state.releaseTime} onSetReleaseTime={
                (date) => onSetReleaseTime(state.key, date)
            } />
        })}
    </div>
}

function EditMotionPanel({motion, info_slide, onChange}) {
    let [isEditing, setIsEditing] = useState(false);

    return <div>
            <h2>Motion</h2>
            <input type="text" readOnly value={motion || ""} />

            <h2>Info Slide</h2>
            <textarea readOnly value={info_slide || ""} />
            <Button role="secondary" onClick={
                () => {
                    setIsEditing(true);
                }
            }>Edit</Button>
            <ModalOverlay open={isEditing} closeOnOverlayClick={false} onAbort={() => setIsEditing(false)} windowClassName={"w-[80%]"}>
                <EditMotionForm motion={motion} infoSlide={info_slide} onChange={(motion, info_slide) => {
                    onChange(motion, info_slide);
                    setIsEditing(false);
                }} />
            </ModalOverlay> 
    </div>
}

export function EditMotionForm({motion, infoSlide, onChange}) {
    let [newMotion, setNewMotion] = useState();
    let [newInfoSlide, setNewInfoSlide] = useState();

    return <div>
        <h2>Motion</h2>
        <TextField value={newMotion || motion || ""} onChange={(evt) => {
            setNewMotion(evt.target.value);
        }} />

        <h2>Info Slide</h2>
        <TextField value={newInfoSlide || infoSlide || ""} onChange={(evt) => {
            setNewInfoSlide(evt.target.value);
        }} area={true} />

        <Button onClick={
            () => {
                onChange(newMotion, newInfoSlide);
            }
        } role="primary">Save</Button>
    </div>
}

export function RoundPublicationView({roundId}) {
    let currentView = {type: "RoundPublication", round_uuid: roundId};

    let publicationInfo = useView(currentView, null);

    //let [importDialogState, setImportDialogState] = useState(null);

    /*let publicationInfo = {
        "motion": "This house would ban the sale of cigarettes",
        "info_slide": "This is an infoslide",
        "draw_release_time": "2023-06-13T19:02:08.907255568",
        "team_motion_release_time": null,
        "full_motion_release_time": null,
        "round_close_time": "2023-06-13T21:02:08.907255568",
        is_silent: false,
    };*/

    if (publicationInfo === null) {
        return <div>Loading…</div>
    }

    return <div>
        <EditMotionPanel motion={publicationInfo.motion} info_slide={publicationInfo.info_slide} onChange={
            (motion, info_slide) => {
                executeAction("UpdateRound", {update: {motion: motion, info_slide: info_slide}, round_id: roundId});
            }
        } />
        <div>
            <RoundStatusBar
                drawReleaseTime={publicationInfo.draw_release_time}
                teamMotionReleaseTime={publicationInfo.team_motion_release_time}
                debateStartTime={publicationInfo.debate_start_time}
                fullMotionReleaseTime={publicationInfo.full_motion_release_time}
                roundCloseTime={publicationInfo.round_close_time}
                feedbackReleaseTime={publicationInfo.feedback_release_time}
                onSetReleaseTime={(key, date) => {
                    let update = {};
                    key = key.replace(/([A-Z])/g, "_$1").toLowerCase();
                    update[key] = date === null ? null : date.toISOString().slice(0, -1);
                    executeAction("UpdateRound", {update: update, round_id: roundId});
                }}
            />
        </div>

        <div>
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
        </div>

        <div>
            <Button role="primary" onClick={
                () => {
                    open({directory: true}).then((result) => {
                        invoke("save_round_files", {roundId: roundId, dirPath: result}).then((result) => {
                            console.log(result);
                        });
                    });
                }
            }>Export Ballots/Presentation</Button>
        </div>
    </div>
}

export function RoundPublicationRoute(props) {
    let { roundId } = useParams();
    return <RoundPublicationView roundId={roundId} />;
}