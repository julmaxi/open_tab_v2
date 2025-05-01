import { Children, useState, forwardRef, useContext, useEffect } from "react";
import Button from "@/UI/Button";

import { useView } from "@/View";
import { TournamentContext } from "@/TournamentContext";
import { ask } from '@tauri-apps/plugin-dialog';
import { executeAction } from "../../Action";
import { Popover } from "../../UI/Popover";
import ContentView from "../../ContentView";
import Select from "../../Select";
import ModalOverlay from "../../UI/Modal";
import { SortableTable } from "../../SortableTable";
import { AdjudicatorBreakSelector } from "../../AdjudicatorBreakSelector";
import Stepper from "../../UI/Stepper";

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

export default RoundsContainer;