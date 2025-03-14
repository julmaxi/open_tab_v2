import React, { useContext } from "react";
import { DrawEditorSettingsContext } from "./DrawSettingsEditor";
import { ISSUE_COLORS_BG, bucketIssuesBySeverity } from "./Clashes";

export function ClashIndicator(props) {
    let issueBuckets = bucketIssuesBySeverity(props.issues);

    let settings = useContext(DrawEditorSettingsContext);
    let issueCount = props.issues.length;

    if (!settings.showMiscIssues) {
        issueCount -= issueBuckets.misc.length;
        issueBuckets.misc = [];
    }
    if (!settings.showLowIssues) {
        issueCount -= issueBuckets.low.length;
        issueBuckets.low = [];
    }
    if (!settings.showMidIssues) {
        issueCount -= issueBuckets.mid.length;
        issueBuckets.mid = [];
    }
    if (!settings.showHighIssues) {
        issueCount -= issueBuckets.high.length;
        issueBuckets.high = [];
    }

    return <div className="font-mono font-bold flex h-6 rounded-md overflow-hidden w-16 border border-gray-600 text-xs" onMouseEnter={() => props.onHover(true)} onMouseLeave={() => props.onHover(false)}>
        {issueCount == 0 ?
            <div className="h-full flex-1 flex items-center justify-center bg-green-500 text-white pl-1 pr-1 text-lg">{"\u2713"}</div>
            :
            ["misc", "low", "mid", "high"].map(
                (key) => {
                    return issueBuckets[key].length > 0 ?
                        <div key={key} className={`h-full flex flex-1 items-center justify-center text-white pl-1 pr-1 ${ISSUE_COLORS_BG[key]}`}>{issueBuckets[key].length <= 9 ? issueBuckets[key].length : <span className="text-[8px]">&gt;9</span>}</div>
                        :
                        null;
                }
            )}
    </div>;
}


export function IssueList({ highlightedIssues, expandIssues }) {
    return expandIssues ? highlightedIssues.map((i, idx) => <p key={idx}>{i.type}</p>) : <><p>{highlightedIssues[0].type}</p>
        {highlightedIssues.length > 1 ? `+${highlightedIssues.length - 1} more` : []}</>
}
