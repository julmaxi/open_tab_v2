//@ts-check
import React, { useState, useCallback, useContext, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "../../App.css";

import { DndContext, closestCenter, DragOverlay, TraversalOrder, AutoScrollActivator, MeasuringStrategy, useDndContext } from '@dnd-kit/core';

import { makeDragHandler } from '../../UI/DragDrop.jsx';

import { useView, updatePath, getPath, clone } from '../../View.js';
import { ErrorHandlingContext, executeAction } from "../../Action.js";
import { TournamentContext } from "../../TournamentContext.js";

import { getMaxSeverityFromEvaluationResult, ISSUE_COLORS_BG, severityToBucket } from "./Clashes.jsx";
import { DebateRow } from "./DebateRow.jsx";
import { DrawToolTray } from "./DrawToolTray.jsx";
import { DrawEditorSettingsContext } from "./DrawSettingsEditor";
import _ from 'lodash';


export const TRAY_DRAG_PATH = "__tray__";
export const TEAM_DRAW_DISABLED_MESSAGE = "You need to enable team assignment in the settings to change the team and speaker draw.";

function simulateDragOutcome(draw, from, to, isSwap) {
    from = clone(from);
    to = clone(to);
    if (_.isEqual(from.collection, TRAY_DRAG_PATH)) {
        if (_.isEqual(to.collection, TRAY_DRAG_PATH)) {
            return {}
        }

        let val = draw.adjudicator_index.find(
            (adjudicator) => adjudicator.adjudicator.uuid == from.index
        );

        if (val === undefined) {
            console.warn(`Could not find ${from.index}`);
            console.info(
                draw.adjudicator_index
            );
            return {};
        }

        if (val.position.type === "NotSet") {
            let to_collection = clone(getPath(draw, to.collection));
            let to_debate = clone(draw.debates[to.collection[1]]);

            if (to.index !== undefined) {
                if (isSwap) {
                    to_collection[to.index] = val.adjudicator;
                }
                else {
                    to_collection.splice(to.index, 0, val.adjudicator);
                }
            }
            else {
                to_collection = val.adjudicator;
            }

            to_debate = updatePath(to_debate, to.collection.slice(2), to_collection);

            return { [to.collection[1]]: to_debate };
        }
        else {
            if (val.position.position.type === "Panel") {
                from.collection = ["debates", val.position.debate_index, "ballot", "adjudicators"];
                from.index = val.position.position.position;
            }
            else if (val.position.position.type === "President") {
                from.collection = ["debates", val.position.debate_index, "ballot", "president"];
                from.index = undefined;
            }
        }
    }

    var from_debate = clone(draw.debates[from.collection[1]]);
    var from_collection = clone(getPath(draw, from.collection));

    var to_debate;
    if (from.collection[1] == to.collection[1]) {
        to_debate = from_debate;
    }
    else {
        to_debate = clone(draw.debates[to.collection[1]]);
    }

    if (_.isEqual(to.collection, TRAY_DRAG_PATH)) {
        if (from.index !== undefined) {
            from_collection.splice(from.index, 1);
        }
        else {
            from_collection = null;
        }
        from_debate = updatePath(from_debate, from.collection.slice(2), from_collection);
        return { [from.collection[1]]: from_debate };
    }
    else {
        var to_collection;
        if (_.isEqual(from.collection, to.collection)) {
            to_collection = from_collection
        }
        else {
            to_collection = clone(getPath(draw, to.collection));
        }
        to.collection = to.collection.slice();

        if (to.index !== undefined && from.index !== undefined) {
            if (isSwap) {
                let tmp = from_collection[from.index];
                from_collection[from.index] = to_collection[to.index];
                to_collection[to.index] = tmp;
            }
            else {
                if (from.index < to.index) {
                    let tmp = from_collection[from.index];
                    to_collection.splice(to.index, 0, tmp);
                    from_collection.splice(from.index, 1);
                }
                else {
                    let tmp = from_collection[from.index];
                    from_collection.splice(from.index, 1);
                    to_collection.splice(to.index, 0, tmp);
                }
            }
        } else if (to.index !== undefined) {
            let from_val = from_collection;
            let to_val = to_collection[to.index];
            from_collection = isSwap ? to_val : null;
            to_collection.splice(to.index, isSwap ? 1 : 0, from_val);
        } else if (from.index !== undefined) {
            let from_val = from_collection[from.index];
            let to_val = to_collection;
            if (isSwap && to_val !== null) {
                from_collection.splice(from.index, 1, to_val);
            }
            else {
                from_collection.splice(from.index, 1);
            }
            to_collection = from_val;
        } else {
            let tmp = from_collection;
            from_collection = to_collection;
            to_collection = tmp;
        }
    }

    from_debate = updatePath(from_debate, from.collection.slice(2), from_collection);

    if (from.collection[1] == to.collection[1]) {
        from_debate = updatePath(from_debate, to.collection.slice(2), to_collection)
        return { [from.collection[1]]: from_debate };
    }
    else {
        to_debate = updatePath(to_debate, to.collection.slice(2), to_collection);
        return { [from.collection[1]]: from_debate, [to.collection[1]]: to_debate };
    }
}

function getDragInfoFromDragInfo(drag_info, draw) {
    if (drag_info.collection === TRAY_DRAG_PATH) {
        if (drag_info.type == "adjudicator") {
            return draw.adjudicator_index.find(
                (adj) => adj.adjudicator.uuid == drag_info.index
            ).adjudicator;
        }
        else if (drag_info.type == "team") {
            return draw.team_index.find(
                (team) => team.team.uuid == drag_info.index
            ).team;
        }
        else if (drag_info.type == "speaker") {
            for (let team of draw.team_index) {
                let member = team.members.find(
                    (member) => member.uuid == drag_info.index
                );
                if (member !== undefined) {
                    return member;
                }
            }
        }
    }
    else {
        let collection = getPath(draw, drag_info.collection);
        if (drag_info.index !== undefined) {
            return collection[drag_info.index];
        }
        else {
            return collection;
        }
    }
}

function DragItemPreview({ item, highlight, ...props }) {
    let issueColor = highlight ? ISSUE_COLORS_BG[highlight] : "bg-gray-100";

    return <div className={`${issueColor} min-w-[14rem] p-1 rounded`}>
        {item.name}
    </div>
}

function Loading() {
    return (
        <div className="flex items-center justify-center h-full">
            <div className="loader">Loading...</div>
        </div>
    );
}

function DrawTable({roundId, debates, dragHighlightedIssues, dragSwapHighlight}) {
    let errorContext = useContext(ErrorHandlingContext);
    let tournament = useContext(TournamentContext);

    let onVenueChange = useCallback((venue, debate) => {
        executeAction("UpdateDraw", { tournament_id: tournament.uuid, updated_debates: [{ ...debate, venue: venue }] }, errorContext.handleError);
    }, [tournament.uuid]);
    return <div className="flex-1 h-full min-w-0" style={
        {
            scrollbarWidth: "none"
        }
    }>
        <div className="w-full h-full flex flex-col">
            <div className="overflow-y-scroll flex-1 w-full scroll-smooth">
                <table className="w-full h-full">
                
                <tbody>
                {debates.map((debate, debateIdx) => {
                    return <DebateRow
                        key={debate.uuid}
                        debate={debate}
                        dragHighlightedIssues={dragHighlightedIssues ? dragHighlightedIssues[debateIdx] : null}
                        dragSwapHighlight={dragSwapHighlight.debateIdx == debateIdx ? dragSwapHighlight : null}
                        onVenueChange={onVenueChange} />;
                    })
                }
                </tbody>
                </table>
            </div>
        </div>
    </div>;
}

function DrawEditor(props) {
    let errorContext = useContext(ErrorHandlingContext);
    let tournament = useContext(TournamentContext);

    function onDragEnd(from, to, isSwap) {
        setDragHighlightedIssues(null);
        setDraggedItemHighlight(null);
        setDraggedItem(null);
        setDragSwapHighlight({
            severityBucket: null,
            debateIdx: null,
            adjudicatorId: null
        });        
        let changedDebates = simulateDragOutcome(draw, from, to, isSwap);

        executeAction("UpdateDraw", {
            tournament_id: tournament.uuid,
            updated_ballots: Object.keys(changedDebates).map(key => changedDebates[key].ballot)
        }, errorContext.handleError);
    }

    function onDragOverFunc(from, to, isSwap) {
        if (dragHighlightedIssues === null) {
            return;
        }
        if (to.collection === TRAY_DRAG_PATH) {
            setDraggedItemHighlight("neutral");
            return;
        }

        if (to.collection !== TRAY_DRAG_PATH) {
            let draggedAdjudicatorId = null;

            if (from.collection == TRAY_DRAG_PATH) {
                draggedAdjudicatorId = from.index;
            }
            else {
                if (from.index !== undefined) {
                    draggedAdjudicatorId = getPath(draw, from.collection)[from.index].uuid;
                }
                else {
                    draggedAdjudicatorId = getPath(draw, from.collection).uuid;
                }
            }

            let outcome = simulateDragOutcome(draw, from, to, isSwap);
            let dragTargetRoomId = to.collection[to.collection.length - 3];
            let targetRoom = outcome[dragTargetRoomId].ballot;

            invoke("evaluate_ballots", { tournamentId: tournament.uuid, roundId: roundId, ballots: [targetRoom], targetUuid: draggedAdjudicatorId }).then(
                (issues) => {
                    let maxSeverity = getMaxSeverityFromEvaluationResult(issues[0]);
                    let severityBucket = maxSeverity == 0 ? "none" : severityToBucket(maxSeverity);
                    setDraggedItemHighlight(severityBucket);
                }
            );

            if (isSwap) {
                let swapAdjudicatorId = null;
                if (to.index !== undefined) {
                    swapAdjudicatorId = getPath(draw, to.collection)[to.index].uuid;
                }
                else {
                    let collectionValue = getPath(draw, to.collection);

                    if (collectionValue) {
                        swapAdjudicatorId = collectionValue.uuid;
                    }
                }
                if (swapAdjudicatorId !== null && from.collection !== TRAY_DRAG_PATH) {
                    let dragSourceRoomId = from.collection[from.collection.length - 3];
                    let sourceRoom = outcome[dragSourceRoomId].ballot;
                    invoke("evaluate_ballots", { tournamentId: tournament.uuid, roundId: roundId, ballots: [sourceRoom], targetUuid: swapAdjudicatorId }).then(
                        (issues) => {
                            let maxSeverity = getMaxSeverityFromEvaluationResult(issues[0]);
                            let severityBucket = maxSeverity == 0 ? "none" : severityToBucket(maxSeverity);
                            if (draggedItem !== null) {
                                setDragSwapHighlight({
                                    severityBucket: severityBucket,
                                    debateIdx: dragTargetRoomId,
                                    adjudicatorId: swapAdjudicatorId
                                });
                            }
                        }
                    );
                }
            }
            else {
                setDragSwapHighlight({
                    severityBucket: null,
                    debateIdx: null,
                    adjudicatorId: null
                });
            }
        }
    }
    const onDragOver = makeDragHandler(onDragOverFunc);

    let currentView = { type: "Draw", uuid: props.round_uuid };
    let draw = useView(currentView, { "debates": [], "adjudicator_index": [],  });
    let debates = draw.debates;

    let roundId = props.round_uuid;

    let [dragHighlightedIssues, setDragHighlightedIssues] = useState(null);
    let [dragSwapHighlight, setDragSwapHighlight] = useState({
        severityBucket: null,
        debateIdx: null,
        adjudicatorId: null
    });
    let [draggedItem, setDraggedItem] = useState(null);
    let [draggedItemHighlight, setDraggedItemHighlight] = useState(null);


    let dragEnd = useCallback(makeDragHandler(onDragEnd), [draw]);
    let dragOver = useCallback(onDragOver, [draw, dragHighlightedIssues]);
    let dragStart = useCallback((x) => {
        setDraggedItem(x.active.data.current);
        if (x.active.data.current.type != "adjudicator") {
            return;
        }

        let simulatedBallots = [];

        for (let i = 0; i < debates.length; i++) {
            let outcome = simulateDragOutcome(
                draw,
                x.active.data.current,
                {
                    index: 0,
                    collection: ["debates", i, "ballot", "adjudicators"]
                },
                false
            );
            simulatedBallots.push(outcome[i].ballot);
        }
        invoke("evaluate_ballots", { tournamentId: tournament.uuid, roundId: roundId, ballots: simulatedBallots, targetUuid: simulatedBallots[0].adjudicators[0].uuid }).then(
            (issues) => {
                setDragHighlightedIssues(issues);
            }
        );
    }, [draw, roundId]);

    let dragItemInfo = null;
    if (draggedItem) {
        dragItemInfo = getDragInfoFromDragInfo(draggedItem, draw);
    }

    let [settings, setSettings] = useState({
        showMiscIssues: true,
        showLowIssues: true,
        showMidIssues: true,
        showHighIssues: true,
        updateSettings: (newSettings) => {
            setSettings(newSettings);
        }
    });

    const dragCancel = useCallback(() => {
            setDragHighlightedIssues(null);
            setDraggedItemHighlight(null);
            setDraggedItem(null);
            setDragSwapHighlight({
                severityBucket: null,
                debateIdx: null,
                adjudicatorId: null
            });
        },
        []
    );

    return <div className="flex flex-row w-full h-full">
        {draw.isLoading ? <Loading /> : 
        <DrawEditorSettingsContext.Provider value={settings}>
            <DndContext
                collisionDetection={closestCenter}
                onDragEnd={dragEnd}
                onDragOver={dragOver}
                onDragStart={dragStart}
                onDragCancel={dragCancel}
                autoScroll={true}
            >

                <DrawTable roundId={roundId} debates={debates} dragHighlightedIssues={dragHighlightedIssues} dragSwapHighlight={dragSwapHighlight} />

                <DrawToolTray round_id={draw.round_uuid} adjudicator_index={draw.adjudicator_index} team_index={draw.team_index} isDragging={dragItemInfo !== null} />

                <DragOverlay dropAnimation={null}>
                    {dragItemInfo ? <DragItemPreview item={dragItemInfo} highlight={draggedItemHighlight} /> : []}
                </DragOverlay>
            </DndContext>
        </DrawEditorSettingsContext.Provider>}
    </div>
}


export default DrawEditor;
