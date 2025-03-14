import React from "react";
import { HorizontalList } from "./HorizontalList";
import { DragBox } from "./DragBox";

export function SpeakerItem(props) {
    return <DragBox
        issues={props.speaker.issues}
        expandIssues={props.expandIssues}
        onHighlightIssues={(shouldHighlight, shouldExpand) => {
            if (shouldHighlight) {
                props.onHighlightIssues(props.speaker.uuid, shouldExpand);
            }
            else {
                props.onHighlightIssues(null, false);
            }
        }} highlightedIssues={props.highlightedIssues}>
        <div>{props.speaker.name}</div>
        <div className="text-xs">{props.speaker.team_name}</div>
        <HorizontalList>
            {props.speaker.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
        </HorizontalList>
    </DragBox>;
}
