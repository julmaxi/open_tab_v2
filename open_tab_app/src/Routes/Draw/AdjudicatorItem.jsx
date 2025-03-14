import React from "react";
import { HorizontalList } from "./HorizontalList";
import { DragBox } from "./DragBox";


export function SkillDisplay({ label, skill, ...props }) {
    let hue = skill / 100 * 120;

    return <div className="flex-1 inline-block text-center" style={{ backgroundColor: `hsl(${hue}, 60%, 45%)` }}>
        {label}: <span className="font-bold">{skill}</span>
    </div>
}

export function AdjudicatorItem(props) {
    let highlightedIssues = props.highlightedIssues;
    let swapIssueSeverity = null;

    if (props.dragSwapHighlight !== null) {
        highlightedIssues = [];

        swapIssueSeverity = props.dragSwapHighlight.severityBucket;
    }
    return <DragBox
        issues={props.adjudicator.issues}
        swapHighlightSeverity={swapIssueSeverity}
        expandIssues={props.expandIssues}
        onHighlightIssues={(shouldHighlight, shouldExpand) => {
            if (shouldHighlight) {
                props.onHighlightIssues(props.adjudicator.uuid, shouldExpand);
            }
            else {
                props.onHighlightIssues(null, shouldExpand);
            }
        }} highlightedIssues={highlightedIssues}>
        <div className={props.adjudicator.is_available ? "" : "line-through"}>{props.adjudicator.name}</div>
        <div className="text-xs rounded w-20 flex flex-row overflow-hidden">
            <SkillDisplay label="C" skill={props.adjudicator.chair_skill} />
            <SkillDisplay label="P" skill={props.adjudicator.panel_skill} />
        </div>
        <HorizontalList>
            {props.adjudicator.institutions.map((i) => <div key={i.uuid} className="text-xs">{i.name}</div>)}
        </HorizontalList>
    </DragBox>;
}
