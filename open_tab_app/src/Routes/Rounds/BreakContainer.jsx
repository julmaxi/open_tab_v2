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
import { ErrorHandlingContext } from "@/Action";



function BreakContainer(props) {
    let tournamentContext = useContext(TournamentContext);
    let errorContext = useContext(ErrorHandlingContext);
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
                                    },
                                    errorContext.handleError
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
                        },
                        errorContext.handleError
                    );
                }


            }
        }>
            Generate Break
        </button>
    </div>
}

export default BreakContainer;