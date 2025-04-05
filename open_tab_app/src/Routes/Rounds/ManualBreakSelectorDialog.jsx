import { Children, useState, forwardRef, useContext, useEffect } from "react";
import Button from "@/UI/Button";

import { useView } from "@/View";
import { TournamentContext } from "@/TournamentContext";
import { ask } from '@tauri-apps/api/dialog';
import { executeAction } from "@/Action";
import { Popover } from "@/UI/Popover";
import ContentView from "@/ContentView";
import Select from "@/Select";
import ModalOverlay from "@/UI/Modal";
import { SortableTable } from "@/SortableTable";
import { AdjudicatorBreakSelector } from "@/AdjudicatorBreakSelector";
import Stepper from "@/UI/Stepper";
import { values } from "lodash";
import { event } from "@tauri-apps/api";
import _ from "lodash";

const avgPointFormat = new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function ManualBreakSelector({ relevantTab, onDone, ...props }) {
    let [teamTabData, setTeamTabData] = useState([]);
    let [speakerTabData, setSpeakerTabData] = useState([]);

    useEffect(() => {
        let teamTabData = [];
        for (let team of relevantTab.tab.team_tab) {
            teamTabData.push({
                team_name: team.team_name,
                rank: team.rank + 1,
                total_points: team.total_score,
                team_uuid: team.team_uuid,
                break: false,
                has_breaking_speaker: false
            });
        }

        let speakerTabData = [];
        for (let speaker of relevantTab.tab.speaker_tab) {
            speakerTabData.push({
                speaker_name: speaker.speaker_name,
                rank: speaker.rank + 1,
                total_points: speaker.total_score,
                speaker_uuid: speaker.speaker_uuid,
                break: false,
                is_in_breaking_team: false
            });
        }

        setTeamTabData(teamTabData);
        setSpeakerTabData(speakerTabData);
    }, [relevantTab]);

    function setBreakForTeam(teamIdx, value) {
        let newTeamTabData = [...teamTabData];
        newTeamTabData[teamIdx] = {
            ...newTeamTabData[teamIdx],
            break: value
        };

        let teamMemberIds = relevantTab.team_members[newTeamTabData[teamIdx].team_uuid];
        let newSpeakerTabData = speakerTabData.map(
            s => {
                if (teamMemberIds.includes(s.speaker_uuid)) {
                    return {
                        ...s,
                        is_in_breaking_team: value
                    }
                }
                return s
            }
        )
        setSpeakerTabData(newSpeakerTabData);

        setTeamTabData(newTeamTabData);
    }

    function setBreakForSpeaker(speakerIdx, value) {
        let newSpeakerTabData = [...speakerTabData];
        newSpeakerTabData[speakerIdx] = {
            ...newSpeakerTabData[speakerIdx],
            break: value
        };

        let newTeamTabData = [...teamTabData];

        let team = teamTabData.find(
            t => {
                return t.team_uuid == relevantTab.speaker_teams[newSpeakerTabData[speakerIdx].speaker_uuid];
            }
        );

        let teamMemberIds = relevantTab.team_members[team.team_uuid];
        let teamHasBreakingSpeaker = newSpeakerTabData.filter(
            (s, idx) => teamMemberIds.includes(s.speaker_uuid)
        ).some(
            (s, idx) => {
                return s.break
            }
        );

        team.has_breaking_speaker = teamHasBreakingSpeaker;

        setTeamTabData(newTeamTabData);
        setSpeakerTabData(newSpeakerTabData);
    }

    function setTopNBreak(n_breaking_teams) {
        let newTeamTabData = structuredClone(teamTabData);
        let newSpeakerTabData = structuredClone(speakerTabData);

        let breakingTeams = [];

        for (let idx = 0; idx < newTeamTabData.length; idx++) {
            if (idx < n_breaking_teams) {
                newTeamTabData[idx].break = true;
            }
            else {
                newTeamTabData[idx].break = false;
            }

            for (let memberId of relevantTab.team_members[newTeamTabData[idx].team_uuid]) {
                let speaker = newSpeakerTabData.find(
                    s => s.speaker_uuid == memberId
                );
                speaker.is_in_breaking_team = newTeamTabData[idx].break;
            }
        }

        let numBreakingSpeakers = 0;

        for (let idx = 0; idx < newSpeakerTabData.length; idx++) {
            let speaker = newSpeakerTabData[idx];
            if (numBreakingSpeakers < (n_breaking_teams / 2 * 3)) {
                if (!speaker.is_in_breaking_team) {
                    numBreakingSpeakers++;
                    speaker.break = true;

                    let team = newTeamTabData.find(
                        t => t.team_uuid == relevantTab.speaker_teams[speaker.speaker_uuid]
                    );
                    team.has_breaking_speaker = true;
                }
                else {
                    speaker.break = false;
                }
            }
            else {
                speaker.break = false;
            }
        }

        for (let team of breakingTeams) {
            let teamMemberIds = relevantTab.team_members[team.team_uuid];
            for (let speaker of newSpeakerTabData) {
                if (teamMemberIds.includes(speaker.speaker_uuid)) {
                    speaker.is_in_breaking_team = true;
                }
            }
        }

        setTeamTabData(newTeamTabData);
        setSpeakerTabData(newSpeakerTabData);
    }

    //let teamTabData = relevantTab.tab.team_tab;
    //let speakerTabData = relevantTab.tab.speaker_tab;

    let [numTargetBreaks, setNumTargetBreaks] = useState(2);

    let numMarkedTeams = teamTabData.filter(
        t => t.break
    ).length;

    let numMarkedSpeaker = speakerTabData.filter(
        s => s.break
    ).length;

    return <div className="flex-1 w-full h-full flex flex-col min-h-0">
        <div className="flex-1 w-full h-full flex flex-row min-h-0">
            <div className="flex-1 flex flex-col min-h-0">
                <SortableTable columns={[
                    { key: "rank", header: "#" },
                    { key: "team_name", header: "Name" },
                    {
                        key: "total_points", header: "Points", cellFactory: (val, rowIdx, idx, row) => {
                            return <td>{avgPointFormat.format(
                                relevantTab.tab.team_tab[rowIdx].total_score
                            )}</td>
                        }
                    },
                    {
                        key: "break", header: "Break?", cellFactory: (val, rowIdx, idx, row) => {
                            return <td><input type="checkbox" disabled={row.has_breaking_speaker} checked={val} onChange={(e) => {
                                setBreakForTeam(rowIdx, e.target.checked);
                            }} /></td>
                        }
                    },
                ]} data={teamTabData} rowId={"team_uuid"} rowStyler={
                    (rowIdx, row) => {
                        if (row.has_breaking_speaker) {
                            return "text-gray-600 line-through";
                        }
                        return "";
                    }
                } />
            </div>
            <div className="flex-1 flex flex-col">
                <SortableTable columns={[
                    { key: "rank", header: "#" },
                    { key: "speaker_name", header: "Name" },
                    {
                        key: "total_points", header: "Points", cellFactory: (val, rowIdx, idx, row) => {
                            return <td>{avgPointFormat.format(
                                relevantTab.tab.speaker_tab[rowIdx].total_score
                            )}</td>
                        }
                    },
                    {
                        key: "break", header: "Break?", cellFactory: (val, rowIdx, idx, row) => {
                            return <td><input type="checkbox" disabled={row.is_in_breaking_team} checked={val} onChange={(e) => {
                                setBreakForSpeaker(rowIdx, e.target.checked);
                            }} /></td>
                        }
                    },
                ]} data={speakerTabData} rowId={"speaker_uuid"} rowStyler={
                    (rowIdx, row) => {
                        if (row.is_in_breaking_team) {
                            return "text-gray-600 line-through";
                        }
                        return "";
                    }
                } />
            </div>
        </div>
        <div>
            <p>Teams: {numMarkedTeams}/{numTargetBreaks} Speaker: {numMarkedSpeaker}/{Math.floor(numTargetBreaks / 2 * 3)} </p>
            <label>Target: </label>
            <input type="number" className="text-black text-center w-12 ml-2 mr-2" value={numTargetBreaks} onChange={
                (e) => {
                    setNumTargetBreaks(e.target.value);
                }
            } />

            <Button onClick={
                (e) => {
                    setTopNBreak(numTargetBreaks);
                }
            }>
                Automatically mark break
            </Button>
        </div>


        <Button role="primary" onClick={
            () => {
                let breakingTeams = teamTabData.filter(
                    t => t.break
                ).map(
                    t => t.team_uuid
                );
                let breakingSpeakers = speakerTabData.filter(
                    s => s.break
                ).map(
                    s => s.speaker_uuid
                );
                onDone(
                    breakingTeams,
                    breakingSpeakers
                )
            }
        }>Set Break</Button>
    </div>
}

function ManualBreakSelectorDialog({ nodeId, onSuccess, onAbort, ...props }) {
    let relevantTab = useView(
        { type: "BreakRelevantTab", node_uuid: nodeId },
        null
    );

    return <div className="w-full h-full flex flex-col">
        <div className="w-full h-full flex-1">
            {relevantTab !== null ? <ManualBreakSelector relevantTab={relevantTab} onDone={
                onSuccess
            } /> : "Loading..."}
        </div>
        <div>
            <Button role="secondary" onClick={onAbort}>Abort</Button>
        </div>
    </div>
}

export default ManualBreakSelectorDialog;