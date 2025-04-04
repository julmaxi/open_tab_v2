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

import RoundGroupEditor from "./RoundGroupEditor";
import BreakEditor from "./BreakEditor";

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
        return <RoundGroupEditor nodeId={node.uuid} nodeContent={node.content} />
    }
}

export default NodeDetailsEditor;