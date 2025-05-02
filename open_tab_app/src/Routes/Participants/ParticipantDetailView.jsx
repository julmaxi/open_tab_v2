import React, { useContext } from "react";
import { useState } from "react";
import { executeAction } from "../../Action";
import { useView } from "../../View";
import { TournamentContext } from "../../TournamentContext";
import Button from "../../UI/Button";
import { SortableTable } from "../../SortableTable";
import ComboBox from "../../UI/ComboBox";
import { useEffect } from "react";
import { useBlocker as useBlocker } from "react-router-dom";
import _, { set } from "lodash";
import TextField from "../../UI/TextField";
import { RowBlockerContext } from "./RowBlocker";

export function ChangeRoleView({ participant, currentType, allTeams }) {
    let teams = Object.values(allTeams);
    let tournamentContext = useContext(TournamentContext);
    return <Section title="Change Role">
        {
            currentType == "Speaker" && <Button
                role="primary"
                className="mt-1 mb-2"
                onClick={() => {
                    let updatedPart = { ...participant };
                    updatedPart.type = "Adjudicator";
                    updatedPart.chair_skill = 50;
                    updatedPart.panel_skill = 50;
                    updatedPart.unavailable_rounds = [];
                    delete updatedPart.team_id;
                    executeAction("UpdateParticipants", {
                        tournament_id: tournamentContext.uuid, updated_participants: [
                            updatedPart
                        ], deleted_participants: []
                    });
                }
            }>
                Make Adjudicator
            </Button>
        }
        <ComboBox placeholder={(currentType == "Adjudicator" ? "Make Speaker and " : "") + "Move to Team…"} items={teams} onSelect={(value, isCreate) => {
            let updatedPart = { ...participant };
            if (participant.type == "Adjudicator") {
                updatedPart.type = "Speaker";
                delete updatedPart.chair_skill;
                delete updatedPart.panel_skill;
                delete updatedPart.unavailable_rounds;
            }

            if (isCreate) {
                delete updatedPart.team_id;
                updatedPart.new_team_name = value;
            }
            else {
                updatedPart.team_id = value.uuid;
            }

            executeAction("UpdateParticipants", {
                tournament_id: tournamentContext.uuid, updated_participants: [
                    updatedPart
                ]
            });
        }} allowCreate={true} />
    </Section>;
}

function Section({ title, children }) {
    return <div className="w-full border-b pb-1">
        {title ? <h2 className="font-bold">{title}</h2> : []}
        {children}
    </div>;
}

function AdjudicatorSkillInput({ value, onChange }) {
    return <input className="border rounded w-12 text-center" type="number" value={value} onChange={(e) => {
        onChange(parseInt(e.target.value));
    }} />
}

export function ParticipantDetailView({ onClose, participant, institutions, break_categories, ...props }) {
    return <InnerParticipantDetailView
        {...props}
        onClose={onClose}
        participant={participant}
        institutions={institutions}
        break_categories={break_categories}
        key={participant.uuid}
    />;
}

function InnerParticipantDetailView({ onClose, participant, institutions, break_categories, ...props }) {
    let [changes, setChanges] = useState({});

    let modifiedParticipant = { ...participant, ...changes };

    let { block } = useContext(RowBlockerContext);
    let lease = useState(null);

    useEffect(() => {
        if (!_.isEmpty(changes)) {
            lease.current = block();
            return () => {
                lease.current.unblock();
            }
        }
        else {
            if (lease.current != null) {
                lease.current.unblock();
            }
            lease.current = null;
        }
    }, [changes]);

    useBlocker(
        !_.isEmpty(changes)
    );

    let tournamentContext = useContext(TournamentContext);
    let roundsOverview = useView({ type: "RoundsOverview", tournament_uuid: tournamentContext.uuid }, { "rounds": [] }).rounds;
    let allInstitutions = useView({ type: "Institutions", tournament_uuid: tournamentContext.uuid }, { "institutions": [] }).institutions;

    let participantView = useView({ type: "ParticipantsList", tournament_uuid: tournamentContext.uuid }, { "teams": {}, "adjudicators": {} });

    let flatParticipantView = Object.values(participantView.teams).flatMap((team) => {
        return Object.values(team.members);
    }).concat(Object.values(participantView.adjudicators));

    if (!modifiedParticipant) {
        return <div className="h-full">
            No participant selected
        </div>;
    }

    let availability = [];
    if (modifiedParticipant.type === "Adjudicator") {
        for (let round of roundsOverview) {
            availability.push({
                round_uuid: round.uuid,
                round_name: round.name,
                available: !modifiedParticipant.unavailable_rounds.includes(round.uuid)
            });
        }
    }

    let flatBreakCategories = Object.values(break_categories);

    return <div className="w-full p-2">
        <button
            onClick={onClose}
            className="absolute top-1 right-1 z-10 bg-transparent text-gray-700 font-semibold hover:text-red-500 text-2xl rounded"
        >
            &times;
        </button>

        <Section>
            <div>
                <label className="font-bold">Name</label>
                    <TextField value={modifiedParticipant.name} onChange={(e) => {
                        setChanges(
                            (changes) => {
                                return { ...changes, name: e.target.value };
                            }
                        )
                    }} />
            </div>

            <div className="flex flex-row items-center">
                <input type="checkbox" checked={modifiedParticipant.is_anonymous} onChange={(e) => {
                    setChanges(
                        (changes) => {
                            return { ...changes, is_anonymous: e.target.checked };
                        }
                    )
                }} />
                <label className="ml-1">Only show initials on tab</label>
            </div>
        </Section>

        <Section title="Break Category">
            <ComboBox
                placeholder={"Select Break Category"}
                items={flatBreakCategories}
                onSelect={(value, isCreate) => {
                    let newBreakCategoryId = null;
                    if (isCreate) {
                        let newUuid = crypto.randomUUID();
                        executeAction("CreateBreakCategory", { tournament_uuid: tournamentContext.uuid, name: value, uuid: newUuid });
                        newBreakCategoryId = newUuid;
                    }
                    else {
                        newBreakCategoryId = value.uuid;
                    }

                    setChanges(
                        (changes) => {
                            return { ...changes, break_category_id: newBreakCategoryId };
                        }
                    )
                }}
                value={modifiedParticipant.break_category_id !== null ? (break_categories[modifiedParticipant.break_category_id]?.name || "<Unknown Category>") : null}
                allowCreate={true}
                
            />
        </Section>

        <Section title="Clashes">
            <div className="h-36">
                <SortableTable
                    rowId={"clash_id"}
                    data={modifiedParticipant.clashes.map(
                        (c) => {
                            return {
                                ...c,
                                clash_id: c.participant_uuid + c.direction
                            };
                        }
                    )}
                    
                    columns={[
                    {
                        "key": "participant_name",
                        "header": "Name",
                        cellFactory: (val, rowIdx, colIdx, row) => {
                            return <td key={colIdx}>
                                {row["direction"] == "Incoming" ? "⬅️" : []} {val}
                            </td>
                        }
                    },
                    {
                        "key": "clash_severity",
                        "header": "Severity",
                    },
                    {
                        "key": "delete",
                        "header": "",
                        cellFactory: (_val, rowIdx, colIdx, row) => {
                            return <td key={colIdx} className="w-4">
                                {row["direction"] !== "Incoming" &&
                                <button
                                    className="bg-transparent w-full text-gray-700 font-semibold hover:text-red-500 rounded"
                                    onClick={() => {
                                        let toDelete = modifiedParticipant.clashes.findIndex(r => r.participant_uuid === row.participant_uuid);

                                        setChanges(
                                            (changes) => {
                                                let newClashes = [...modifiedParticipant.clashes];
                                                newClashes.splice(toDelete, 1);
                                                return { ...changes, clashes: newClashes };
                                            }
                                        );
                                    }}>
                                    &times;
                                </button>
                            }
                            </td>;
                        }
                    }
                ]} />
            </div>
            <ComboBox placeholder={"Add Clash"} items={flatParticipantView} onSelect={(participant) => {
                if (modifiedParticipant.clashes.findIndex(c => c.participant_uuid === participant.uuid) !== -1) {
                    return;
                }
                let clashEntry = {
                    participant_uuid: participant.uuid,
                    participant_name: participant.name,
                    direction: "Outgoing",
                    clash_severity: 100
                };
                let newClashes = [...modifiedParticipant.clashes, clashEntry];

                newClashes.sort(
                    (a, b) => {
                        if (a.participant_name < b.participant_name) {
                            return -1;
                        }
                        if (a.participant_name > b.participant_name) {
                            return 1;
                        }
                        if (a.participant_uuid < b.participant_uuid) {
                            return -1;
                        }
                        if (a.participant_uuid > b.participant_uuid) {
                            return 1;
                        }
                        if (a.direction == "Outgoing" && b.direction == "Incoming") {
                            return -1;
                        }
                        if (a.direction == "Incoming" && b.direction == "Outgoing") {
                            return 1;
                        }
                        return 0;
                    }
                )

                setChanges(
                    (changes) => {
                        return { ...changes, clashes: newClashes };
                    }
                );
                
            }} />
        </Section>

        <Section title={"Institutions"}>
            <div className="h-36">
                <SortableTable rowId={"uuid"} selectedRowId={null} data={
                    modifiedParticipant.institutions.map(
                        (i) => {
                            return {
                                ...i,
                                name: institutions[i.uuid]?.name || "<Unknown Institution>",
                            };
                        }
                    )
                } columns={[
                    {
                        "key": "name",
                        "header": "Name",
                    },
                    {
                        "key": "clash_severity",
                        "header": "Severity",
                    },
                    {
                        "key": "delete",
                        "header": "",
                        cellFactory: (value, rowIdx, colIdx, row) => {
                            return <td key={colIdx} className="w-4"><button
                                className="bg-transparent w-full text-gray-700 font-semibold hover:text-red-500 rounded"
                                onClick={() => {
                                    let toDelete = modifiedParticipant.institutions.findIndex(r => r.uuid === row.uuid);
                                    let newInstitutions = [...modifiedParticipant.institutions];
                                    newInstitutions.splice(toDelete, 1);

                                    setChanges(
                                        (changes) => {
                                            return { ...changes, institutions: newInstitutions };
                                        }
                                    );
                                }}
                            >&times;</button></td>;
                        }
                    }
                ]} />
            </div>
            <ComboBox placeholder={"Add Institution"} items={allInstitutions} ignoredItemNames={modifiedParticipant.institutions.map(i => i.name)} onSelect={(institution, isCreate) => {

                if (isCreate) {
                    let newUuid = crypto.randomUUID();
                    executeAction("CreateInstitution", { tournament_uuid: tournamentContext.uuid, name: institution, uuid: newUuid });
                    let institutionEntry = {
                        uuid: newUuid,
                        clash_severity: 100
                    };
                    
                    setChanges(
                        (changes) => {
                            return { ...changes, institutions: [...modifiedParticipant.institutions, institutionEntry] };
                        }
                    );
                }
                else {
                    if (modifiedParticipant.institutions.findIndex(i => i.uuid === institution.uuid) !== -1) {
                        return;
                    }
                    let institutionEntry = {
                        uuid: institution.uuid,
                        clash_severity: 100
                    };
                    setChanges(
                        (changes) => {
                            return { ...changes, institutions: [...modifiedParticipant.institutions, institutionEntry] };
                        }
                    );
                }
            }} allowCreate={true} />
        </Section>

        {
            modifiedParticipant.type === "Adjudicator" &&
            <Section title={"Adjudicator Settings"}>
                <div className="flex flex-col">
                    <div className="flex w-full">
                        <div className="flex-1 items-end flex justify-end">
                            <label className="pr-2">Chair Skill</label>
                            <AdjudicatorSkillInput value={modifiedParticipant.chair_skill} onChange={(value) => {
                                setChanges((changes) => {
                                    return { ...changes, chair_skill: value };
                                });
                            }} />
                        </div>
                        <div className="flex-1">
                            <AdjudicatorSkillInput value={modifiedParticipant.panel_skill} onChange={(value) => {
                                setChanges((changes) => {
                                    return { ...changes, panel_skill: value };
                                });
                            }} />
                            <label className="pl-2">Panel Skill</label>
                        </div>
                    </div>

                    <SortableTable rowId={"round_uuid"} data={availability} columns={[
                        {
                            "key": "round_name",
                            "header": "Round",
                        },
                        {
                            "key": "available",
                            "header": "Available",
                            cellFactory: (value, rowIdx, colIdx, row) => {
                                return <td key={colIdx} className="w-4"><input type="checkbox" checked={value} onChange={(e) => {
                                    let newAvailability = [...availability];
                                    newAvailability[rowIdx].available = e.target.checked;

                                    setChanges((changes) => {
                                        return { ...changes, unavailable_rounds: newAvailability.filter(r => !r.available).map(r => r.round_uuid) };
                                    });
                                }} /></td>;
                            }
                        }
                    ]} />
                </div>
            </Section>
        }

        {!_.isEmpty(changes) ? <Button className="mt-2 mb-2" onClick={async () => {
            await executeAction("UpdateParticipants", { tournament_id: tournamentContext.uuid, updated_participants: [modifiedParticipant], deleted_participants: [] });
            setChanges({});
        }}>Save Changes</Button> : []}
    </div>;
}
