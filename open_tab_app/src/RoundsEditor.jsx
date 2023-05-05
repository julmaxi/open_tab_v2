import { Children, useState, useRef, forwardRef, cloneElement, useContext } from "react";
import Button from "./Button";

import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";

function RoundCard(props) {
    return <div className="bg-gray-200 rounded-lg p-2 m-2">
        {props.round.name}
    </div>
}

function RoundGroup(props) {
    return <div className="flex flex-col items-center">
        {
            props.rounds.map((round) => 
                <RoundCard round={round} key={round.uuid} />
            )
        }
    </div>
}

function RoundBreakHierarchy(props) {
    return <div className="flex-col w-full h-full border-t">
        <RoundGroup rounds={props.rounds} />
        <div className="flex flex-row justify-center">
            <Button role="primary">Add Break…</Button>
        </div>
        <div className="flex flex-row">
            {
                props.breaks.map((roundBreak) => 
                    <div className="flex flex-1">
                        <RoundBreakHierarchy rounds={roundBreak.rounds} key={roundBreak.uuid} breaks={roundBreak.breaks || []} />
                    </div>
                )
            }
        </div>
    </div>
}

function RoundContainer(props) {
    return <div className="bg-gray-300 p-1 text-center w-24">
        {props.round.name}
    </div>
}

function RoundsContainer(props) {
    return <div className="overflow-clip rounded">
        {
            props.rounds.map((round) => 
                <RoundContainer round={round} key={round.uuid} />
            )
        }
    </div>
}

function BreakContainer(props) {
    return <div className="rounded border-2 p-1 text-center w-24">
        Break
    </div>
}

function NodeContent(props) {    
    if (props.node.type == "RoundGroup") {
        return <RoundsContainer
            rounds={props.node.rounds}
        />
    }
    else if (props.node.type == "Break") {
        return <BreakContainer break={props.node} />
    }
    else if (props.node.type == "Round") {
        //return <RoundContainer round={props.node} />
        return <RoundsContainer
            rounds={[props.node]}
        />
    }

    return props.node.type
}

function NodeContainer(props) {
    return <div className="flex flex-col items-center">
        <NodeContent node={props.node} />
    </div>
}

function TournamentSubtree(props) {
    return <div className="flex flex-col items-center">
        <NodeContainer node={props.tree.content} />
        {props.tree.available_actions.length > 0 ? <AddButton actions={props.tree.available_actions} /> : []}
        <div className="flex flex-row p-1">
            {
                (props.tree.children || []).map((child, idx) => <TournamentSubtree tree={child} key={idx} />)
            }
        </div>
    </div>
}

import {
    useFloating,
    autoUpdate,
    offset,
    flip,
    shift,
    arrow,
    useDismiss,
    useRole,
    useClick,
    useInteractions,
    FloatingFocusManager,
    useId,
    FloatingArrow
    
  } from '@floating-ui/react';
import { executeAction } from "./Action";



function Popover(props) {
    const arrowRef = useRef(null);

    const { refs, floatingStyles, context } = useFloating({
      open: props.isOpen,
      onOpenChange: (open) => {
        if (open) {
            props.onOpen();
        }
        else {
            props.onClose();
        }
      },
      placement: "right",
      middleware: [
        offset(10),
        flip({ fallbackAxisSideDirection: "end" }),
        shift(),
        arrow({
        element: arrowRef,
        }),    
      ],
      whileElementsMounted: autoUpdate
    });
  
    const click = useClick(context);
    const dismiss = useDismiss(context);
    const role = useRole(context);
  
    const { getReferenceProps, getFloatingProps } = useInteractions([
      click,
      dismiss,
      role
    ]);
  
    const headingId = useId();
  
    return (
      <>
        {
            cloneElement(
                props.trigger,
                getReferenceProps({
                  ref: refs.setReference,
                  ...props,
                  ...props.trigger.props,
                  "data-state": context.open ? "open" : "closed"
            }))
        }

        {props.isOpen && (
          <FloatingFocusManager context={context} modal={false}>
            <div
              className="bg-white border rounded p-2"
              ref={refs.setFloating}
              style={floatingStyles}
              aria-labelledby={headingId}
              {...getFloatingProps()}
            >
                <FloatingArrow ref={arrowRef} context={context} />
                {props.children}
            </div>
          </FloatingFocusManager>
        )}
      </>
    );
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



    return tournamentTree != null ? <TournamentSubtree tree={tournamentTree.tree} /> : <div>Loading...</div>;

    /*return <div>
        <RoundBreakHierarchy rounds={roundsHierarchy.rounds} breaks={roundsHierarchy.breaks}/>
    </div>*/
}


export function RoundsEditorRoute(props) {
    return <div><RoundsOverview /></div>
}