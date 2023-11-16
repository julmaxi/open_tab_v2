import { Children, useState, forwardRef, useContext } from "react";
import Button from "./Button";

import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { ask } from '@tauri-apps/api/dialog';


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
                if (!props.rounds.every(r => r.plan_state == "Ghost")) {
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

function NodeContainer(props) {
    return <div className="flex flex-col items-center">
        <NodeContent node={props.node} nodeId={props.nodeId} />
    </div>
}

function TournamentSubtree(props) {
    return <div className="flex flex-col items-center">
        <NodeContainer node={props.tree.content} nodeId={props.tree.uuid} />
        {props.tree.available_actions.length > 0 ? <AddButton actions={props.tree.available_actions} /> : []}
        <div className="flex flex-row p-1">
            {
                (props.tree.children || []).map((child, idx) => <TournamentSubtree tree={child} key={idx} />)
            }
        </div>
    </div>
}

import { executeAction } from "./Action";
import { Popover } from "./Popover";



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
                props.actions.map((action, idx) =>
                    {
                        let {type, ...rest} = action.action;

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

function RoundsOverview(props) {
    let tournamentContext = useContext(TournamentContext);

    let tree = useView({type: "TournamentTree", tournament_uuid: tournamentContext.uuid}, null);
    let tournamentTree = tree;
    console.log(tournamentTree);

    return <div className="flex flex-row justify-center w-full h-full overflow-scroll">
        {tournamentTree != null ? <TournamentSubtree tree={tournamentTree.tree} /> : <div>Loading...</div>}
    </div>
}


export function RoundsEditorRoute(props) {
    return <div className="w-full h-full"><RoundsOverview /></div>
}