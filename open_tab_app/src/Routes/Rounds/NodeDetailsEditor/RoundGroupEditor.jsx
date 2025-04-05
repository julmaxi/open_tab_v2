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
import _ from "lodash";

function RoundGroupEditorInner({ nodeId, nodeContent }) {
    let [actualEditedConfig, setEditedConfig] = useState({});
    let tournamentContext = useContext(TournamentContext);

    let editedConfig = {..._.cloneDeep(nodeContent.config), ...actualEditedConfig}

    return <div className="w-full h-full">
        {
            nodeContent.config.type == "Preliminaries" ? (
                <PreliminariesConfigurator
                    config={editedConfig}
                    onChangeConfig={(c) => setEditedConfig(c)}
                />
            ) : (
                <FoldConfigurator
                    config={editedConfig}
                    onChangeConfig={(c) => setEditedConfig(c)}
                />
            )
        }

        {
            !_.isEqual(editedConfig, nodeContent.config) ? <div className="flex flex-row mt-2 border-t">
                <Button role="primary" onClick={() => {
                    executeAction(
                        "EditTournamentTree",
                        {
                            tournament_id: tournamentContext.uuid,
                            action: {
                                type: "UpdateNode",
                                node: nodeId,
                                config: editedConfig
                            }
                        }
                    ).then(
                        setEditedConfig({})
                    )
                }}>Save</Button>
                <Button role="secondary" onClick={() => {
                    setEditedConfig(nodeContent.config);
                }}>Cancel</Button>
            </div> : []
        }
    </div>
}

function PreliminariesConfigurator({ config, onChangeConfig }) {
    return <div>
        <label className="block text-sm font-medium text-gray-700">Number of Rounds</label>
        <input type="number" step={3} min={3} className="text-black text-center w-12 ml-2 mr-2" value={config.num_roundtrips * 3} onChange={
            (e) => {
                onChangeConfig({
                    ...config,
                    num_roundtrips: Math.floor(e.target.value / 3)
                });
            }
        } />
    </div>
}

function TeamFoldMethodSelector({ method, onChange }) {
    let options = [
        {
            value: "PowerPaired",
            label: "Power-Pairing"
        },
        {
            value: "InversePowerPaired",
            label: "Inverse Power-Pairing"
        },
        {
            value: "BalancedPowerPaired",
            label: "Balanced Power-Pairing"
        },
        {
            value: "Random",
            label: "Random"
        },
        {
            value: "HalfRandom",
            label: "Random within tab half"
        },
    ]
    return <Select label="Team Fold Method" options={options} value={method} onChange={(e) => {
        onChange(e.target.value);
    }} />
}

function TeamAssignmentRuleSelector({ method, onChange }) {
    let options = [
        {
            value: "Random",
            label: "Random"
        },
        {
            value: "InvertPrevious",
            label: "Try to avoid prev. position"
        },
     ]
    return <Select label="Team Fold Method" options={options} value={method} onChange={(e) => {
        onChange(e.target.value);
    }} />
}

function NonAlignedFoldMethodSelector({ method, onChange }) {
    let options = [
        {
            value: "TabOrder",
            label: "By Tab Order"
        },
        {
            value: "Random",
            label: "Random"
        },
    ]
    return <Select label="Non-Aligned Fold Method" options={options} value={method} onChange={(e) => {
        onChange(e.target.value);
    }} />
}

function FoldConfigurator({ config, onChangeConfig }) {
    return <div>
        {
            config.round_configs.map(
                (roundConfig, idx) => {
                    return <div key={idx}>
                        <h1>{numToOrdinal(idx + 1)} Round</h1>
                        <TeamFoldMethodSelector method={roundConfig.team_fold_method} onChange={(newValue) => {
                            let newRounds = [...config.round_configs];
                            newRounds[idx].team_fold_method = newValue;
                            onChangeConfig(
                                {
                                    ...config,
                                    round_configs: newRounds
                                }
                            )
                        }} />
                        <NonAlignedFoldMethodSelector method={roundConfig.non_aligned_fold_method} onChange={(newValue) => {
                            let newRounds = [...config.round_configs];
                            newRounds[idx].non_aligned_fold_method = newValue;
                            onChangeConfig(
                                {
                                    ...config,
                                    round_configs: newRounds
                                }
                            )
                        }} />
                        <TeamAssignmentRuleSelector method={roundConfig.team_assignment_rule} onChange={(newValue) => {
                            let newRounds = [...config.round_configs];
                            newRounds[idx].team_assignment_rule = newValue;
                            onChangeConfig(
                                {
                                    ...config,
                                    round_configs: newRounds
                                }
                            )
                        }} />
                    </div>
                }
            )
        }
    </div>
}

function numToOrdinal(n) {
    let s = ["th", "st", "nd", "rd"];
    let v = n % 100;
    return n + (s[(v - 20) % 10] || s[v] || s[0]);
}

function RoundGroupEditor({ nodeId, nodeContent }) {
    return <RoundGroupEditorInner key={nodeId} nodeId={nodeId} nodeContent={nodeContent} />
}

export default RoundGroupEditor;