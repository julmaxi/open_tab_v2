import React, { useState, useContext } from "react";
import { DropWell, DropList } from "../../UI/DragDrop";
import { AdjudicatorItem } from "./AdjudicatorItem";
import { find_issues_with_target } from "./Clashes";
import { TEAM_DRAW_DISABLED_MESSAGE } from "./Draw";
import { DrawEditorSettingsContext } from "./DrawSettingsEditor";
import { SpeakerItem } from "./SpeakerItem";
import { TeamItem } from "./TeamItem";
import { VenueSelector } from "./VenueSelector";

export function DebateRow(props) {
    let ballot = props.debate.ballot;
    let [localHighlightedIssues, setLocalHighlightedIssues] = useState({
        "government": [],
        "opposition": [],
        "adjudicators": [],
        "non_aligned_speakers": []
    });

    let highlightedIssues = props.dragHighlightedIssues ? props.dragHighlightedIssues : localHighlightedIssues;

    let [shouldExpandLocalIssues, setShouldExpandLocalIssues] = useState(false);

    let settings = useContext(DrawEditorSettingsContext);

    let fixedHeight = props.fixedHeight;

    return <>
        <tr>
            <td colSpan={4}>
                Debate {props.debate.index + 1}: <VenueSelector venue={props.debate.venue} onVenueChange={(venue) => props.onVenueChange(venue)} />
            </td>
        </tr>
        <tr>
            <td className="border">
                <DropWell disabledMessage={TEAM_DRAW_DISABLED_MESSAGE} disabled={!settings.enableAlterTeamDraw} type="team" collection={["debates", props.debate.index, "ballot", "government"]}>
                    {ballot.government !== null ? <TeamItem
                        team={ballot.government}
                        expandIssues={props.expandIssues}
                        onHighlightIssues={(uuid, shouldExpand) => {
                            setLocalHighlightedIssues(find_issues_with_target(ballot, uuid));
                            setShouldExpandLocalIssues(shouldExpand);
                        }}
                        highlightedIssues={highlightedIssues.government} /> : []}
                </DropWell>
                <br />
                <DropWell disabledMessage={TEAM_DRAW_DISABLED_MESSAGE} disabled={!settings.enableAlterTeamDraw} type="team" collection={["debates", props.debate.index, "ballot", "opposition"]}>
                    {ballot.opposition !== null ? <TeamItem
                        team={ballot.opposition}
                        expandIssues={shouldExpandLocalIssues}
                        onHighlightIssues={(uuid, shouldExpand) => {
                            setLocalHighlightedIssues(find_issues_with_target(ballot, uuid));
                            setShouldExpandLocalIssues(shouldExpand);
                        }}
                        highlightedIssues={highlightedIssues.opposition} /> : []}
                </DropWell>
            </td>
            <td className="border">
                <DropList disabledMessage={TEAM_DRAW_DISABLED_MESSAGE} disabled={!settings.enableAlterTeamDraw} type="speaker" collection={["debates", props.debate.index, "ballot", "non_aligned_speakers"]}>
                    {ballot.non_aligned_speakers.map((speaker, idx) => speaker ? <SpeakerItem
                        key={speaker.uuid}
                        speaker={speaker}
                        expandIssues={shouldExpandLocalIssues}
                        onHighlightIssues={(uuid, shouldExpand) => {
                            setLocalHighlightedIssues(find_issues_with_target(ballot, uuid));
                            setShouldExpandLocalIssues(shouldExpand);
                        }}
                        highlightedIssues={highlightedIssues.non_aligned_speakers[idx]} /> : <div key={idx} className="h-8 w-full text-center italic border-2 border-dashed border-gray-500 text-gray-500 rounded">Missing</div>)}
                </DropList>
            </td>
            <td className="border">
                <div className="overflow-scroll" style={fixedHeight !== false ? { height: `${fixedHeight}px` } : {}}>
                    <DropList minWidth={"200px"} type="adjudicator" collection={["debates", props.debate.index, "ballot", "adjudicators"]}>
                        {ballot.adjudicators.map((adjudicator, idx) => <AdjudicatorItem
                            key={adjudicator.uuid}
                            adjudicator={adjudicator}
                            expandIssues={shouldExpandLocalIssues}
                            onHighlightIssues={(uuid, shouldExpand) => {
                                setLocalHighlightedIssues(find_issues_with_target(ballot, uuid));
                                setShouldExpandLocalIssues(shouldExpand);
                            }}
                            highlightedIssues={highlightedIssues.adjudicators[idx]}
                            dragSwapHighlight={props.dragSwapHighlight && props.dragSwapHighlight.adjudicatorId == adjudicator.uuid ? props.dragSwapHighlight : null} />)}
                    </DropList>
                </div>
            </td>
            <td className="border">
                <DropWell
                    minWidth={"200px"}
                    type="adjudicator"
                    collection={["debates", props.debate.index, "ballot", "president"]}
                >
                    {ballot.president ? <AdjudicatorItem adjudicator={ballot.president} onHighlightIssues={() => { }} dragSwapHighlight={props.dragSwapHighlight && props.dragSwapHighlight.adjudicatorId == ballot.president.uuid ? props.dragSwapHighlight : null} /> : []}
                </DropWell>
            </td>
        </tr>
    </>;
}
