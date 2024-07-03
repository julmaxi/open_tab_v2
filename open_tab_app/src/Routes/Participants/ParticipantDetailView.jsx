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
import _ from "lodash";
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

export function ParticipantDetailView({ onClose, participant, ...props }) {
    let [modifiedParticipant, setModifiedParticipant] = useState(participant);

    useEffect(() => {
        if (modifiedParticipant?.uuid != participant?.uuid) {
            setModifiedParticipant(participant);
        }
    }, [participant, modifiedParticipant.uuid]);

    let [hasChanges, setHasChanges] = useState(false);
    let { block } = useContext(RowBlockerContext);

    useEffect(() => {
        let newHasChanges = !_.isEqual(
            participant,
            modifiedParticipant
        );
        setHasChanges(newHasChanges);

        if (newHasChanges) {
            let lease = block();
            return () => {
                lease.unblock();
            }
        }
    }, [participant, modifiedParticipant]);

    useBlocker(
        hasChanges
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
                        setModifiedParticipant({ ...modifiedParticipant, name: e.target.value });
                    }} />
            </div>

            <div className="flex flex-row items-center">
                <input type="checkbox" checked={modifiedParticipant.is_anonymous} onChange={(e) => {
                    setModifiedParticipant({ ...modifiedParticipant, is_anonymous: e.target.checked });
                }} />
                <label className="ml-1">Only show initials on tab</label>
            </div>
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
                                        setModifiedParticipant({ ...modifiedParticipant, clashes: [...modifiedParticipant.clashes.slice(0, toDelete), ...modifiedParticipant.clashes.slice(toDelete + 1)] });
                                }}>
                                    &times;
                                </button>}</td>;
                        }
                    }
                ]} />
            </div>
            <ComboBox placeholder={"Add Clash"} items={flatParticipantView} onSelect={(participant) => {
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

                setModifiedParticipant(
                    { ...modifiedParticipant, clashes: newClashes });

            }} />
        </Section>

        <Section title={"Institutions"}>
            <div className="h-36">
                <SortableTable rowId={"uuid"} selectedRowId={null} data={modifiedParticipant.institutions} columns={[
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
                                    setModifiedParticipant({ ...modifiedParticipant, institutions: [...modifiedParticipant.institutions.slice(0, toDelete), ...modifiedParticipant.institutions.slice(toDelete + 1)] });
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
                        name: institution,
                        uuid: newUuid,
                        clash_severity: 100
                    };
                    setModifiedParticipant({ ...modifiedParticipant, institutions: [...modifiedParticipant.institutions, institutionEntry] });
                }
                else {
                    let institutionEntry = {
                        ...institution,
                        clash_severity: 100
                    };
                    setModifiedParticipant({ ...modifiedParticipant, institutions: [...modifiedParticipant.institutions, institutionEntry] });
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
                                setModifiedParticipant({ ...modifiedParticipant, chair_skill: value });
                            }} />
                        </div>
                        <div className="flex-1">
                            <AdjudicatorSkillInput value={modifiedParticipant.panel_skill} onChange={(value) => {
                                setModifiedParticipant({ ...modifiedParticipant, panel_skill: value });
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
                                    setModifiedParticipant({ ...modifiedParticipant, unavailable_rounds: newAvailability.filter(r => !r.available).map(r => r.round_uuid) });
                                }} /></td>;
                            }
                        }
                    ]} />
                </div>
            </Section>
        }

        {hasChanges ? <Button className="mt-2 mb-2" onClick={() => {
            executeAction("UpdateParticipants", { tournament_id: tournamentContext.uuid, updated_participants: [modifiedParticipant], deleted_participants: [] });
        }}>Save Changes</Button> : []}
    </div>;
}
