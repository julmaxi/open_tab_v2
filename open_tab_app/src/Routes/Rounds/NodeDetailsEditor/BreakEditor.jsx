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
        { label: "Reitze Break", value: "TimBreak" },
        { label: "Break Winning Teams only (no speakers)", value: "TeamOnlyKnockoutBreak" },
        { label: "Break only the best speaker", value: "BestSpeakerOnlyBreak"}
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
                    <label className="block text-sm font-medium text-gray-700">Breaking Teams</label>

                    <Stepper value={editedConfig.num_teams} onChange={(value) => {
                        let newConfig = { ...editedConfig };
                        newConfig.num_teams = value;
                        setEditedConfig(newConfig);
                    }} />

                    <label className="block text-sm font-medium text-gray-700">Breaking Speakers</label>

                    <Stepper value={editedConfig.num_non_aligned} onChange={(value) => {
                        let newConfig = { ...editedConfig };
                        newConfig.num_teams = value;
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

export default BreakEditor;