import { Children, useState, forwardRef, useContext, useEffect } from "react";
import Button from "@/UI/Button";

import { useView } from "@/View";
import { TournamentContext } from "@/TournamentContext";
import { ask } from '@tauri-apps/api/dialog';
import { executeAction } from "../../Action";
import { Popover } from "../../UI/Popover";
import ContentView from "../../ContentView";
import Select from "../../Select";
import ModalOverlay from "../../UI/Modal";
import { SortableTable } from "../../SortableTable";
import { AdjudicatorBreakSelector } from "../../AdjudicatorBreakSelector";
import Stepper from "../../UI/Stepper";
import ManualBreakSelectorDialog from "./ManualBreakSelectorDialog";


import RoundsContainer from "./RoundsContainer";
import BreakContainer from "./BreakContainer";
import NodeDetailsEditor from "./NodeDetailsEditor";


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
    return <div className="flex flex-row justify-center w-full h-full overflow-auto">
        {tournamentTree != null ? <TournamentSubtree tree={tournamentTree.tree} onSelectNode={onSelectNode} selectedNodeId={selectedNodeId} /> : <div>Loading...</div>}
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