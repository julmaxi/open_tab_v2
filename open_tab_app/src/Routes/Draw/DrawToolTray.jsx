import React, { useState, memo } from "react";
import { TabGroup, Tab } from "../../TabGroup";
import { DragItem, DropSlot } from "../../UI/DragDrop";
import { TRAY_DRAG_PATH } from "./Draw";
import { DrawSettingsEditor } from "./DrawSettingsEditor";

function adjPositionToStr(position) {
    if (position.type == "NotSet") {
        return "-";
    }
    else {
        let chairStr = "";

        if (position.position.type == "President") {
            chairStr = "Pres.";
        }
        else {
            let isChair = position.position.position == 0;
            chairStr = isChair ? "Chair" : "Panel";
        }

        return `${chairStr} ${position.debate_index + 1}`;
    }
}

function teamPositionToStr(position) {
    if (position.type == "NotSet") {
        return "-";
    }
    else if (position.type == "NonAligned") {
        let positions = Object.entries(position.member_positions).map(
            ([_, p]) => p.debate_index + 1
        );

        return `Non. ${positions.join(", ")}`;
    }
    else {
        const abbreviations = {
            "Government": "Gov.",
            "Opposition": "Opp.",
        };
        return `${abbreviations[position.role] || "<Unknown>"} ${position.debate_index + 1}`;

    }
}
const AdjudicatorTable = memo(function ({ adjudicator_index, ...props }) {
    return <div className="h-full overflow-auto">
        <table className="w-full text-sm">
            <thead className="sticky top-0 bg-white">
                <tr>
                    <th>Name</th>
                    <th>Position</th>
                </tr>
            </thead>
            <tbody className="w-full">
                {adjudicator_index.map(
                    (adj, idx) => {
                        return <DragItem content_tag="tr" key={idx} collection={TRAY_DRAG_PATH} index={adj.adjudicator.uuid} type={"adjudicator"}>
                            <td className={adj.is_available ? "" : "line-through"}>{adj.adjudicator.name}</td>
                            <td>{adjPositionToStr(adj.position)}</td>
                        </DragItem>;
                    }
                )}
            </tbody>
        </table>
    </div>;
});

const TeamTable = memo(function ({ team_index, ...props }) {
    return <div className="h-full overflow-auto">
        <table className="w-full text-sm">
            <thead className="sticky top-0 bg-white">
                <tr>
                    <th>Name</th>
                    <th>Position</th>
                </tr>
            </thead>
            <tbody className="w-full">
                {team_index.map(
                    (entry, idx) => <TeamIndexEntry key={entry.team.uuid} entry={entry} />
                )}
            </tbody>
        </table>
    </div>;
});

function SpeakerIndexEntries({ team, positions, ...props }) {
    let rows = team.members.map(
        (member) => {
            let position = positions[member.uuid];
            return <tr>
                <td className="pl-4">{member.name}</td>
                <td>{position.debate_index + 1}{position.position !== undefined ? ` (${position.position + 1})` : []}</td>
            </tr>;
        }
    );
    return <>
        {rows}
    </>;
}
function TeamIndexEntry({ entry, ...props }) {
    let [isExpanded, setIsExpanded] = useState(false);

    return <>
        <tr onClick={() => {
            setIsExpanded(!isExpanded);
        }}>
            <td>{entry.team.name}</td>
            <td>{teamPositionToStr(entry.position)}</td>
        </tr>
        {isExpanded ? <SpeakerIndexEntries team={entry.team} positions={entry.position.member_positions || Object.fromEntries(
            entry.team.members.map(
                (member) => [member.uuid, entry.position]
            )
        )} /> : []}

    </>;
}
const DropIndicator = ({ visible }) => {
    return (
        <div
            className={`absolute w-full h-full bg-black bg-opacity-50 z-10 flex items-center justify-center 
      ${visible ? '' : 'hidden'}`}>
            <div className="text-center py-3 px-6 rounded-lg">
                <svg className="w-12 h-12 mx-auto mb-2 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12"></path>
                </svg>
                <p className="text-white">Remove</p>
            </div>
        </div>
    );
};
export function DrawToolTray({ round_id, adjudicator_index, team_index, isDragging, ...props }) {
    return <div className="w-72 border-l h-full relative">
        <DropSlot collection={TRAY_DRAG_PATH} type={"adjudicator"} className={"h-full"}>
            <DropIndicator visible={isDragging} />
            <TabGroup>
                <Tab name="Adjudicators" autoScroll={false}>
                    <AdjudicatorTable adjudicator_index={adjudicator_index} />
                </Tab>
                <Tab name="Teams">
                    <TeamTable team_index={team_index} />
                </Tab>
                <Tab name="Settings" autoScroll={false}>
                    <DrawSettingsEditor round_id={round_id} />
                </Tab>
            </TabGroup>
        </DropSlot>
    </div>;
}
