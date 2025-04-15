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
import ManualBreakSelectorDialog from "../ManualBreakSelectorDialog";

/**
 * @typedef TournamentEligibleBreakCategory
 * @type {{
 *   category_id: string,
 *   config: {
 *     team_eligibility_mode?: string,
 *     non_aligned_eligibility_mode?: string,
 *     adjudicator_eligibility_mode?: string
 *   }
 * }}
 */

function updatedEligibilityVecFromMap(
    eligibleCategoriesMap
) {
    let out = [];
    for (let [key, val] of eligibleCategoriesMap.entries()) {
        out.push({
            category_id: key,
            config: {
                team_eligibility_mode: val.config.team_eligibility_mode,
                non_aligned_eligibility_mode: val.config.non_aligned_eligibility_mode,
                adjudicator_eligibility_mode: val.config.adjudicator_eligibility_mode
            }
        });
    }

    return out;
}

/**
 * 
 * @param {{ eligibleCategories: TournamentEligibleBreakCategory[], onChange: (categories: string[]) => void }} props
 * @returns 
 */
function BreakEligibilityEditor({ eligibleCategories, onChange }) {
    let tournamentContext = useContext(TournamentContext);

    /** 
     * @type {{
     *   categories: Array<{
     *     name: string,
     *     uuid: string,
     *     isSet: boolean
     *   }>
     * }}
     */
    let view = useView(
        { type: "BreakCategories", tournament_uuid: tournamentContext.uuid },
        { categories: [] }
    );

    /**
     * @type {Map<string, TournamentEligibleBreakCategory>}
     */
    let eligibleCategoriesMap = new Map();
    for (let category of eligibleCategories) {
        eligibleCategoriesMap.set(category.category_id, category);
    }

    function updatedEligibility(uuid, key, value) {
        let newEligibilityMap = new Map(eligibleCategoriesMap);
        let cat =  newEligibilityMap.get(
            uuid
        );
        if (!cat) {
            cat = {
                category_id: uuid,
                config: {
                    team_eligibility_mode: "DoNotRestrict",
                    non_aligned_eligibility_mode: "DoNotRestrict",
                    adjudicator_eligibility_mode: "DoNotRestrict"    
                }
            }
            newEligibilityMap.set(
                uuid,
                cat
            )
        }
        cat.config[key] = value;
        onChange(updatedEligibilityVecFromMap(
            newEligibilityMap
        ));
    }

    let { categories } = view;

    let categoryInfo = categories.map(
        (category) => {
            let val = eligibleCategoriesMap.get(category.uuid);
            if (val) {
                return {
                    name: category.name,
                    uuid: category.uuid,
                    team_eligibility_mode: val.config.team_eligibility_mode,
                    non_aligned_eligibility_mode: val.config.non_aligned_eligibility_mode,
                    adjudicator_eligibility_mode: val.config.adjudicator_eligibility_mode
                }
            }
            else {
                return {
                    name: category.name,
                    uuid: category.uuid,
                    team_eligibility_mode: "DoNotRestrict",
                    non_aligned_eligibility_mode: "DoNotRestrict",
                    adjudicator_eligibility_mode: "DoNotRestrict"
                }
            }
        }
    );

    return <div>
        <SortableTable
            data={categoryInfo}
            rowId={"uuid"}
            columns={
                [
                    { key: "name", header: "Category"},
                    {
                        key: "team_eligibility_mode",
                        header: "Teams",
                        cellFactory: (val, rowIdx, colIdx, rowValue) => {
                            let configOptions = [
                                { label: "Req. Any", value: "AnyEligible" },
                                { label: "Req. Maj.", value: "MajorityEligible" },
                                { label: "Req. All", value: "AllEligible" },
                                { label: "No Restriction", value: "DoNotRestrict"}
                            ];
        
                            return <td key="team_eligibility_mode"><Select
                                options={configOptions}
                                value={val}
                                onChange={(e) => {
                                    updatedEligibility(
                                        rowValue.uuid,
                                        "team_eligibility_mode",
                                        e.target.value
                                    )
                                }} /></td>
                        }
                    },
                    {
                        key: "non_aligned_eligibility_mode",
                        header: "Non-Aligned",
                        cellFactory: (val, rowIdx, colIdx, rowValue) => {
                            let configOptions = [
                                { label: "Req. Elig.", value: "AllEligible" },
                                { label: "Req. Team", value: "AllInEligibleTeams" },
                                { label: "Req. Elig. + Team", value: "AllInEligibleTeams" },
                                { label: "No Restriction", value: "DoNotRestrict"}
                            ];
        
                            return <td key="non_aligned_eligibility_mode"><Select
                                options={configOptions}
                                value={val}
                                onChange={(e) => {
                                    updatedEligibility(
                                        rowValue.uuid,
                                        "non_aligned_eligibility_mode",
                                        e.target.value
                                    )
                                }} /></td>
                        }
                    },
                    {
                        key: "adjudicator_eligibility_mode",
                        header: "Adjudicators",
                        cellFactory: (val, rowIdx, colIdx, rowValue) => {
                            let configOptions = [
                                { label: "Req.", value: "AllEligible" },
                                { label: "No Restriction", value: "DoNotRestrict"}
                            ];
        
                            return <td key="adjudicator_eligibility_mode"><Select
                                options={configOptions}
                                value={val}
                                onChange={(e) => {
                                    updatedEligibility(
                                        rowValue.uuid,
                                        "adjudicator_eligibility_mode",
                                        e.target.value
                                    )
                                }} /></td>
                        }
                    }

                ]
            }
        />
    </div>
}

function BreakEditorInner({ nodeId, nodeContent }) {
    let [actualEditedContent, setEditedContent] = useState({});

    let tournamentContext = useContext(TournamentContext);

    let [isEditingManualBreak, setIsEditingManualBreak] = useState(false);
    let [isEditingAdjudicatorBreak, setIsEditingAdjudicatorBreak] = useState(false);

    let editedContent = {..._.cloneDeep(nodeContent), ...actualEditedContent}

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
        <Select label="Break Type" options={options} value={editedContent.config.type} onChange={(e) => {
            let newConfig = { ...editedContent.config, type: e.target.value };
            if (e.target.value == "TabBreak" && isNaN(editedContent.config.num_teams)) {
                newConfig.num_teams = 2;
            }
            if (e.target.value == "TabBreak" && isNaN(editedContent.config.num_non_aligned)) {
                newConfig.num_non_aligned = 3;
            }
            let newContent = {...editedContent, config: newConfig}
            setEditedContent(newContent);
        }} />

        <label className="block text-sm font-medium text-gray-700">Award Title</label>
        <input value={
            editedContent.suggested_award_title
        } onChange={
            (e) => {
                let val = e.target.value.trim();
                if (val.length == 0) {
                    val = null;
                }
                let newContent = {...editedContent, suggested_award_title: e.target.value}
                setEditedContent(newContent);
            }
        } className="w-full mt-2 border p-1" placeholder="Award Title" />

        <label className="block text-sm font-medium text-gray-700">Award Series Key</label>

        <input value={
            editedContent.suggested_award_series_key
        } onChange={
            (e) => {
                let val = e.target.value.trim();
                if (val.length == 0) {
                    val = null;
                }
                let newContent = {...editedContent, suggested_award_series_key: e.target.value}
                setEditedContent(newContent);
            }
        } className="w-full mt-2 border p-1" placeholder="Award Series Key" />

        {
            <div className="flex flex-row mt-2 border-t">
                {editedContent.config.type == "TabBreak" ? <div>
                    <label className="block text-sm font-medium text-gray-700">Breaking Teams</label>

                    <Stepper value={editedContent.config.num_teams} onChange={(value) => {
                        let newConfig = { ...editedContent.config, num_teams: value };
                        newConfig.num_teams = value;
                        let newContent = {...editedContent, config: newConfig}
                        setEditedContent(newContent);
                    }} />

                    <label className="block text-sm font-medium text-gray-700">Breaking Speakers</label>

                    <Stepper value={editedContent.config.num_non_aligned} onChange={(value) => {
                        let newConfig = { ...editedContent.config, num_non_aligned: value };
                        newConfig.num_non_aligned = value;
                        let newContent = {...editedContent, config: newConfig}
                        setEditedContent(newContent);
                    }} />
                </div> : []}
            </div>
        }

        {
            !_.isEqual(editedContent, nodeContent) ? <div className="flex flex-row mt-2 border-t">
                <Button role="primary" onClick={() => {
                    let edited = {...editedContent};+
                    delete edited.break_id
                    delete edited.type
                
                    executeAction(
                        "EditTournamentTree",
                        {
                            tournament_id: tournamentContext.uuid,
                            action: {
                                type: "UpdateNode",
                                node: nodeId,
                                ...edited
                            }
                        }
                    ).then(
                        () => {
                            setEditedContent({})
                        }
                    )

                }}>Save</Button>
                <Button role="secondary" onClick={() => {
                    setEditedContent({});
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

        <div className="pt-2">
            <BreakEligibilityEditor
                eligibleCategories={editedContent.eligible_categories}
                onChange={
                    (eligibleCategories) => {
                        setEditedContent({
                            ...editedContent,
                            ...{
                                eligible_categories: eligibleCategories
                            }
                        })
                    }
                }
            />
        </div>

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

function BreakEditor({ nodeId, ...props }) {
    return <BreakEditorInner key={nodeId} nodeId={nodeId} {...props} />
}

export default BreakEditor;