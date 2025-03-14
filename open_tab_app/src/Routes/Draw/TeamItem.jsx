import React from "react";
import { HorizontalList } from "./HorizontalList";
import { DragBox } from "./DragBox";

export function TeamItem(props) {
    let all_participant_institutions = props.team.members.map((m) => m.institutions).flat().sort((a, b) => a.name.localeCompare(b.name));
    let unique_participant_institutions = [...new Set(all_participant_institutions.map((i) => i.uuid))].map((uuid) => all_participant_institutions.find((i) => i.uuid === uuid));

    return <DragBox
        issues={props.team.issues}
        expandIssues={props.expandIssues}
        onHighlightIssues={(shouldHighlight, shouldExpand) => {
            if (shouldHighlight) {
                props.onHighlightIssues(props.team.uuid, shouldExpand);
            }
            else {
                props.onHighlightIssues(null, false);
            }
        }}
        highlightedIssues={props.highlightedIssues}
    >
        <div>{props.team.name}</div>
        <HorizontalList>
            {props.team.members.map((member) => <div key={member.uuid} className="text-xs">{member.name}</div>)}
        </HorizontalList>
        <HorizontalList>
            {unique_participant_institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
        </HorizontalList>
    </DragBox>;
}
