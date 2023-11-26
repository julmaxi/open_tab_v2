import { Children, useState, forwardRef, useContext, useEffect } from "react";
import Button from "./Button";

import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { ask } from '@tauri-apps/api/dialog';
import ExponentialStepper from "./ExponentialStepper";

const avgPointFormat = new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function RoundContainer(props) {
    let style = "p-1 text-center w-28";
    if (props.round.plan_state == "Superflous") {
        style += " text-red"
    }
    else if (props.round.plan_state == "Ghost") {
        style += " text-gray-300"
    }

    return <div className={style}>
        {props.round.name}
        {props.round.plan_state == "Ghost" ?
            <p className="text-gray-300 text-xs">Not yet drawn</p>
            :
            []
        }
    </div>
}

function RoundsContainer(props) {
    let tournamentContext = useContext(TournamentContext);
    return <div className="overflow-clip rounded border-2 p-1 ">
        {
            props.rounds.map((round, idx) =>
                <RoundContainer round={round} key={round.uuid ? round.uuid : idx} />
            )
        }
        <button className="text-xs text-center w-full" onClick={
            (e) => {
                e.stopPropagation();
                if (!props.rounds.every(r => r.plan_state == "Ghost")) {
                    ask('Are you sure? This will override the previous draw.', { title: 'Generate Break', type: 'warning' }).then(
                        (yes) => {
                            if (yes) {
                                executeAction(
                                    "ExecutePlanNode",
                                    {
                                        "plan_node": props.nodeId,
                                        "tournament_id": tournamentContext.uuid
                                    }
                                );
                            }
                        }
                    )
                }
                else {
                    executeAction(
                        "ExecutePlanNode",
                        {
                            "plan_node": props.nodeId,
                            "tournament_id": tournamentContext.uuid
                        }
                    );
                }
            }
        }>
            Generate Draw
        </button>
    </div>
}

function BreakContainer(props) {
    let tournamentContext = useContext(TournamentContext);
    return <div className="rounded border-2 p-1 text-center w-28">
        Break

        <div className="text-sm text-center w-full">
            {props.break.break_description}
        </div>
        {props.break.uuid === null ? <p className="text-gray-300 text-xs">No break yet</p> : []}

        <button className="text-xs text-center w-full" onClick={
            (e) => {
                e.stopPropagation();
                if (props.break.uuid !== null) {
                    ask('Are you sure? This will override the previous break', { title: 'Generate Break', type: 'warning' }).then(
                        (yes) => {
                            if (yes) {
                                executeAction(
                                    "ExecutePlanNode",
                                    {
                                        "plan_node": props.nodeId,
                                        "tournament_id": tournamentContext.uuid
                                    }
                                );
                            }
                        }
                    );
                }
                else {
                    executeAction(
                        "ExecutePlanNode",
                        {
                            "plan_node": props.nodeId,
                            "tournament_id": tournamentContext.uuid
                        }
                    );
                }


            }
        }>
            Generate Break
        </button>
    </div>
}

function NodeContent(props) {
    if (props.node.type == "RoundGroup") {
        return <RoundsContainer
            rounds={props.node.rounds}
            nodeId={props.nodeId}
        />
    }
    else if (props.node.type == "Break") {
        return <BreakContainer break={props.node} nodeId={props.nodeId} />
    }

    return props.node.type
}

function NodeContainer({ onSelect, node, nodeId, shouldHighlight = false }) {
    let className = "flex flex-col items-center";
    if (shouldHighlight) {
        className += " shadow shadow-blue-500/50 rounded";
    }
    return <div className={className} onClick={() => {
        onSelect();
    }}>
        <NodeContent node={node} nodeId={nodeId} />
    </div>
}

function TournamentSubtree({ onSelectNode, selectedNodeId, ...props }) {
    return <div className="flex flex-col items-center">
        <NodeContainer shouldHighlight={selectedNodeId !== null && selectedNodeId == props.tree.uuid} node={props.tree.content} nodeId={props.tree.uuid} onSelect={() => {
            onSelectNode(props.tree.uuid);
        }} />
        {props.tree.available_actions.length > 0 ? <AddButton actions={props.tree.available_actions} /> : []}
        <div className="flex flex-row p-1">
            {
                (props.tree.children || []).map((child, idx) => <TournamentSubtree tree={child} key={idx} onSelectNode={onSelectNode} selectedNodeId={selectedNodeId} />)
            }
        </div>
    </div>
}

import { executeAction } from "./Action";
import { Popover } from "./Popover";
import ContentView from "./ContentView";
import Select from "./Select";
import ModalOverlay from "./Modal";
import { SortableTable } from "./SortableTable";
import { AdjudicatorBreakSelector } from "./AdjudicatorBreakSelector";



function AddButton(props) {
    let trigger = <button
        className="bg-transparent p-0.5 rounded-full hover:bg-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-opacity-50 h-5 w-5"
        aria-label="Add"
    >
        <div className="flex items-center justify-center border-2 w-full h-full border-gray-500 rounded-full">
            <svg
                className=" text-gray-500 w-full h-full transition duration-200 ease-in-out"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                xmlns="http://www.w3.org/2000/svg"
            >
                <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                />
            </svg>
        </div>
    </button>;

    let tournamentContext = useContext(TournamentContext);

    let [isOpen, setIsOpen] = useState(false);

    return <Popover trigger={trigger} isOpen={isOpen} onOpen={() => {
        setIsOpen(true);
    }} onClose={() => {
        setIsOpen(false);
    }}>
        <div className="flex flex-col">
            {
                props.actions.map((action, idx) => {
                    let { type, ...rest } = action.action;

                    return <Button role="primary" onClick={
                        () => {
                            setIsOpen(false);
                            executeAction(
                                "EditTournamentTree",
                                {
                                    tournament_id: tournamentContext.uuid,
                                    action: action.action
                                }
                            )
                        }
                    } key={idx}>{action.description}</Button>
                }
                )
            }
        </div>
    </Popover>
}

function RoundsOverview({ tournamentTree, onSelectNode, selectedNodeId }) {
    return <div className="flex flex-row justify-center w-full h-full overflow-scroll">
        {tournamentTree != null ? <TournamentSubtree tree={tournamentTree.tree} onSelectNode={onSelectNode} selectedNodeId={selectedNodeId} /> : <div>Loading...</div>}
    </div>
}


function NodeDetailsEditor({ node }) {
    let tournamentContext = useContext(TournamentContext);

    if (node.content.type == "Break") {
        return <BreakEditor nodeId={node.uuid} nodeContent={node.content} onUpdate={
            (newConfig) => {
                executeAction(
                    "EditTournamentTree",
                    {
                        tournament_id: tournamentContext.uuid,
                        action: {
                            type: "UpdateNode",
                            node: node.uuid,
                            config: {
                                type: "Break",
                                config: newConfig
                            }
                        }
                    }
                )
            }

        } />
    }
    else {
        return <RoundGroupEditor nodeContent={node.content} />
    }
}

function BreakEditor({ nodeId, nodeContent, onUpdate }) {
    let [editedConfig, setEditedConfig] = useState(nodeContent.config);
    let [isEditingManualBreak, setIsEditingManualBreak] = useState(false);
    let [isEditingAdjudicatorBreak, setIsEditingAdjudicatorBreak] = useState(false);

    useEffect(() => {
        setEditedConfig(nodeContent.config);
    }, [nodeContent]);

    let options = [
        { label: "Tab", value: "TabBreak" },
        { label: "Knockout", value: "KnockoutBreak" },
        { label: "Minor Break (Top 2/3)", value: "TwoThirdsBreak" },
        { label: "Reitze Break", value: "TimBreak" }
    ];

    if (nodeContent.config.type == "Manual") {
        options.push({ label: "Manual", value: "Manual", selectable: false });
    }

    return <div className="w-full h-full p-10">
        <Select label="Break Type" options={options} value={editedConfig.type} onChange={(e) => {
            let newConfig = { ...editedConfig, type: e.target.value };
            if (e.target.value == "TabBreak" && isNaN(newConfig.num_debates)) {
                newConfig.num_debates = 1;
            }
            setEditedConfig(newConfig);
        }} />

        {
            <div className="flex flex-row mt-2 border-t">
                {editedConfig.type == "TabBreak" ? <div>
                    <label className="block text-sm font-medium text-gray-700">Break Size</label>

                    <ExponentialStepper value={editedConfig.num_debates * 2} onChange={(value) => {
                        let newConfig = { ...editedConfig };
                        newConfig.num_debates = Math.ceil(value / 2);
                        setEditedConfig(newConfig);
                    }} />
                </div> : []}
            </div>
        }

        {
            editedConfig !== nodeContent.config ? <div className="flex flex-row mt-2 border-t">
                <Button role="primary" onClick={() => {
                    onUpdate(editedConfig);
                }}>Save</Button>
                <Button role="secondary" onClick={() => {
                    setEditedConfig(nodeContent.config);
                }}>Cancel</Button>
            </div> : []
        }
        <div className="pt-2">
            <Button role="primary" onClick={() => {
                setIsEditingManualBreak(true);
            }}>Break manually…</Button>
        </div>

        {nodeContent.uuid ? <div className="pt-2">
            <Button role="primary" onClick={() => {
                setIsEditingAdjudicatorBreak(true);
            }}>Add Adjudictor Break…</Button>
        </div> : []}

        <ModalOverlay open={isEditingManualBreak} windowClassName="flex h-screen">
            {isEditingManualBreak ? <ManualBreakSelectorDialog nodeId={nodeId} onAbort={
                () => {
                    setIsEditingManualBreak(false);
                }
            } onSuccess={
                (breakingTeams, breakingSpeaker) => {
                    executeAction(
                        "SetManualBreak",
                        {
                            node_id: nodeId,
                            breaking_teams: breakingTeams,
                            breaking_speakers: breakingSpeaker
                        }
                    )

                    setIsEditingManualBreak(false);
                }

            } /> : []}
        </ModalOverlay>

        <ModalOverlay open={isEditingAdjudicatorBreak} windowClassName="flex h-screen">
            {isEditingAdjudicatorBreak ? <AdjudicatorBreakSelector nodeId={nodeId} onAbort={
                () => {
                    setIsEditingAdjudicatorBreak(false);
                }
            } onSuccess={
                (breakingAdjudicators) => {
                    executeAction(
                        "SetAdjudicatorBreak",
                        {
                            node_id: nodeId,
                            breaking_adjudicators: breakingAdjudicators,
                        }
                    )

                    setIsEditingAdjudicatorBreak(false);
                }

            } /> : []}
        </ModalOverlay>
    </div>
}

function RoundGroupEditor({ node_content }) {
    return <div className="w-full h-full">

    </div>
}

function ManualBreakSelector({ relevantTab, onDone, ...props }) {
    let [teamTabData, setTeamTabData] = useState([]);
    let [speakerTabData, setSpeakerTabData] = useState([]);

    useEffect(() => {
        let teamTabData = [];
        for (let team of relevantTab.tab.team_tab) {
            teamTabData.push({
                team_name: team.team_name,
                rank: team.rank,
                total_points: team.total_points,
                team_uuid: team.team_uuid,
                break: false,
                has_breaking_speaker: false
            });
        }

        let speakerTabData = [];
        for (let speaker of relevantTab.tab.speaker_tab) {
            speakerTabData.push({
                speaker_name: speaker.speaker_name,
                rank: speaker.rank,
                total_points: speaker.total_points,
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
        console.log(relevantTab.team_members);

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
        console.log(n_breaking_teams);
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
                                relevantTab.tab.team_tab[rowIdx].total_points
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
                                relevantTab.tab.speaker_tab[rowIdx].total_points
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
            <p>Teams: {numMarkedTeams}/{numTargetBreaks} Speaker: {numMarkedSpeaker}/{Math.floor(numTargetBreaks/2*3)} </p>
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

function findNodeInTree(tree, uuid) {
    if (tree.uuid == uuid) {
        return tree;
    }

    for (let child of tree.children) {
        let node = findNodeInTree(child, uuid);
        if (node !== null) {
            return node;
        }
    }

    return null;
}


function RoundsEditor() {
    let tournamentContext = useContext(TournamentContext);

    let tree = useView({ type: "TournamentTree", tournament_uuid: tournamentContext.uuid }, null);
    let tournamentTree = tree;

    let [selectedNodeId, setSelectedNodeUuid] = useState(null);
    let [selectedNode, setSelectedNode] = useState(null);

    useEffect(() => {
        if (selectedNodeId !== null) {
            setSelectedNode(findNodeInTree(tournamentTree.tree, selectedNodeId));
        }
    }, [selectedNodeId, tournamentTree]);

    return <div className="w-full h-screen">
        <ContentView forceOpen={selectedNodeId !== null}>
            <ContentView.Content>
                <RoundsOverview tournamentTree={tournamentTree} onSelectNode={
                    (nodeUuid) => {
                        setSelectedNodeUuid(nodeUuid);
                    }
                } selectedNodeId={selectedNodeId} />
            </ContentView.Content>
            <ContentView.Drawer>
                {
                    selectedNode !== null ? <NodeDetailsEditor node={selectedNode} /> : <div className="w-full h-full p-10">Select a node to edit</div>
                }
            </ContentView.Drawer>
        </ContentView>
    </div>
}

export function RoundsEditorRoute(props) {
    return <RoundsEditor />
}