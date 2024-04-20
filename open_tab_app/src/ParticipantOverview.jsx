//@ts-check

import React, { useCallback, useContext, useMemo } from "react";
import { useState } from "react";
import { ErrorHandlingContext, executeAction } from "./Action";
import { getPath, useView } from "./View";
import { TournamentContext } from "./TournamentContext";

import ModalOverlay from "./Modal";
import Button from "./Button";
import { SortableTable, EditableCell } from "./SortableTable";
import ComboBox from "./ComboBox";
import { useEffect } from "react";
import { confirm, save } from '@tauri-apps/api/dialog';

import {
    BrowserRouter as Router,
    useBlocker as useBlocker,
  } from "react-router-dom";
import { openImportDialog } from "./openImportDialog";
import { ParticipantImportDialogButton } from "./ParticipantImportDialog";
import ErrorBoundary from "./ErrorBoundary";
import { invoke } from "@tauri-apps/api/tauri";
import _ from "lodash";

function TeamDetailView({team, onChange}) {
    let [name, setName] = useState(team.name);
    let tournamentContext = useContext(TournamentContext);
    let errorContext = useContext(ErrorHandlingContext);

    useEffect(() => {
        setName(team.name);
    }, [team.uuid]);

    let hasChanges = !_.eq(
        team.name,
        name
    );
    
    return <div className="w-full">
        <label>Team Name</label>
        <div>
        <input type="text" value={name} onChange={(e) => {
            setName(e.target.value);
        }} />
        </div>

        {hasChanges && <Button onClick={() => {
            executeAction("UpdateTeams", {tournament_id: tournamentContext.uuid, updates: [
                {
                    uuid: team.uuid,
                    name: name
                }
            ]}, errorContext.handleError)
        }}> Save </Button> }

    </div>
}


function ChangeRoleView({participant, currentType, allTeams}) {
    let teams = Object.values(allTeams);
    let tournamentContext = useContext(TournamentContext);
    console.log(participant);
    return <div className="pt-3">
        <h2 className="font-bold">Change Role</h2>
        {
            currentType == "Speaker" && <Button role="primary" onClick={
                () => {
                    let updatedPart = {...participant};
                    updatedPart.type = "Adjudicator";
                    updatedPart.chair_skill = 50;
                    updatedPart.panel_skill = 50;
                    updatedPart.unavailable_rounds = [];
                    delete updatedPart.team_id;
                    executeAction("UpdateParticipants", {tournament_id: tournamentContext.uuid, updated_participants: [
                        updatedPart
                    ], deleted_participants: []})
                }
            }>
                Make Adjudicator
            </Button>
        }
        <ComboBox placeholder={
            (currentType == "Adjudicator" ? "Make Speaker and " : "") + "Move to Team…"} items={teams} onSelect={
                (value, isCreate) => {
                    console.log(">>", value);
                    let updatedPart = {...participant};
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

                    executeAction("UpdateParticipants", {tournament_id: tournamentContext.uuid, updated_participants: [
                        updatedPart
                    ]});
                }
            } allowCreate={true} />
    </div>
}


function ParticipantDetailView({onClose, participant, ...props}) {
    let [modifiedParticipant, setModifiedParticipant] = useState(participant);
    
    let hasChanges = !_.eq(
        participant,
        modifiedParticipant
    );
    useBlocker(
        hasChanges
    );

    let tournamentContext = useContext(TournamentContext);
    let roundsOverview = useView({type: "RoundsOverview", tournament_uuid: tournamentContext.uuid}, {"rounds": []}).rounds;
    let allInstitutions = useView({type: "Institutions", tournament_uuid: tournamentContext.uuid}, {"institutions": []}).institutions;

    let participantView = useView({type: "ParticipantsList", tournament_uuid: tournamentContext.uuid},  {"teams": {}, "adjudicators": {}});

    let flatParticipantView = Object.values(participantView.teams).flatMap((team) => {
        return Object.values(team.members)
    }).concat(Object.values(participantView.adjudicators));

    useEffect(() => {
        setModifiedParticipant(participant);
    }, [participant]);

    if (!modifiedParticipant) {
        return <div className="h-full">
            No participant selected
        </div>
    }

    let availability = [];
    if (modifiedParticipant.type === "Adjudicator") {
        for (let round of roundsOverview) {
            availability.push({
                round_uuid: round.uuid,
                round_name: round.name,
                available: !modifiedParticipant.unavailable_rounds.includes(round.uuid)
            })
        }
    }

    return <div>
        <button
            onClick={onClose}
            className="absolute top-1 right-1 z-10 bg-transparent text-gray-700 font-semibold hover:text-red-500 text-2xl rounded"
        >
      &times;
    </button>

    <div>
        <label>Name</label>
        <div>
            <input type="text" value={modifiedParticipant.name} onChange={(e) => {
                setModifiedParticipant({...modifiedParticipant, name: e.target.value});
            }} />
        </div>
    </div>

    <div>
        <input type="checkbox" checked={modifiedParticipant.is_anonymous} onChange={(e) => {
            setModifiedParticipant({...modifiedParticipant, is_anonymous: e.target.checked});
        } } />
        <label>Only show initials on tab</label>
    </div>

    <div className="flex flex-wrap">
        <div className="w-full">
            <h2>Clashes</h2>
            <div className="h-36">
                <SortableTable rowId={"participant_uuid"} data={
                    modifiedParticipant.clashes
                } columns={
                    [
                        {
                            "key": "participant_name",
                            "header": "Name",
                        },
                        {
                            "key": "clash_severity",
                            "header": "Severity",
                        },
                        {
                            "key": "delete",
                            "header": "",
                            cellFactory: (_val, rowIdx, colIdx, row) => {
                                return <td key={colIdx} className="w-4"><button
                                    className="bg-transparent w-full text-gray-700 font-semibold hover:text-red-500 rounded"
                                    onClick={() => {
                                        let toDelete = modifiedParticipant.clashes.findIndex(r => r.participant_uuid === row.participant_uuid);
                                        setModifiedParticipant({...modifiedParticipant, clashes: [...modifiedParticipant.clashes.slice(0, toDelete), ...modifiedParticipant.clashes.slice(toDelete + 1)]});
                                    }}
                                >&times;</button></td>
                            }
                        }
                    ]
                } />
            </div>
            <ComboBox placeholder={"Add Clash"} items={
                flatParticipantView
            } onSelect={
                (participant) => {
                    let clashEntry = {
                        participant_uuid: participant.uuid,
                        participant_name: participant.name,
                        direction: "Outgoing",
                        clash_severity: 100
                    };
                    setModifiedParticipant({...modifiedParticipant, clashes: [...modifiedParticipant.clashes, clashEntry]});                        

                }
            }/>
        </div>

        <div className="w-full">
            <h2>Institutions</h2>
            <div className="h-36">
                <SortableTable rowId={"uuid"} selectedRowId={null} data={
                    modifiedParticipant.institutions
                } columns={
                    [
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
                                        setModifiedParticipant({...modifiedParticipant, institutions: [...modifiedParticipant.institutions.slice(0, toDelete), ...modifiedParticipant.institutions.slice(toDelete + 1)]});
                                    }}
                                >&times;</button></td>
                            }
                        }
                    ]
                } />
            </div>
        <ComboBox placeholder={"Add Institution"} items={allInstitutions} ignoredItemNames={modifiedParticipant.institutions.map(i => i.name)} onSelect={
            (institution, isCreate) => {

                if (isCreate) {
                    console.log(institution);
                    let newUuid = crypto.randomUUID();
                    executeAction("CreateInstitution", {tournament_uuid: tournamentContext.uuid, name: institution, uuid: newUuid});
                    let institutionEntry = {
                        name: institution,
                        uuid: newUuid,
                        clash_severity: 100
                    };
                    setModifiedParticipant({...modifiedParticipant, institutions: [...modifiedParticipant.institutions, institutionEntry]});                        
                }
                else {
                    let institutionEntry = {
                        ...institution,
                        clash_severity: 100
                    };
                    setModifiedParticipant({...modifiedParticipant, institutions: [...modifiedParticipant.institutions, institutionEntry]});                        
                }
            }
        } allowCreate={true}/>
        </div>

    </div>

    {
        modifiedParticipant.type === "Adjudicator" &&
            <div className="flex flex-col">
                <div className="flex w-full">
                    <div>
                        <label>Chair Skill</label>
                        <input type="number" value={modifiedParticipant.chair_skill} onChange={(e) => {
                            setModifiedParticipant({...modifiedParticipant, chair_skill: parseInt(e.target.value)});
                        }} />
                    </div>
                    <div>
                        <label>Panel Skill</label>
                        <input type="number" value={modifiedParticipant.panel_skill} onChange={
                            (e) => {
                                setModifiedParticipant({...modifiedParticipant, panel_skill: parseInt(e.target.value)});
                            }
                        } />
                    </div>
                </div>

                <SortableTable rowId={"round_uuid"} data={
                    availability
                } columns = {
                    [
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
                                    setModifiedParticipant({...modifiedParticipant, unavailable_rounds: newAvailability.filter(r => !r.available).map(r => r.round_uuid)});
                                }} /></td>
                            }
                        }
                    ]
                } />
            </div>
     }

    {
        hasChanges ? <Button onClick={() => {
            executeAction("UpdateParticipants", {tournament_id: tournamentContext.uuid, updated_participants: [modifiedParticipant], deleted_participants: []})
        }}>Save Changes</Button> : <Button role="danger" onClick={() => {
            confirm("Are you sure you want to delete this participant? You can not undo this.").then((result) => {
                if (result === true) {
                    executeAction("UpdateParticipants",  {tournament_id: tournamentContext.uuid, updated_participants: [], deleted_participants: [participant.uuid]})
                }
            })
        }}>
            Delete
        </Button>
    }

    </div>
}

function ParticipantTable(props) {
    let [selectedParticipantUuid, setSelectedParticipantUuid] = useState(null);
    let [selectedTeamUuid, setSelectedTeamUuid] = useState(null);

    let tournamentContext = useContext(TournamentContext);

    let statusView = useView({type: "TournamentStatus", tournament_uuid: tournamentContext.uuid}, null);

    let url = statusView ? statusView.remote_url : null;
    //FIXME: This should come from the server somehow.
    //This way, this will not work if the remote server does not
    //call its frontend tabs.servername
    //For now, not worth the additional complexity to fix.
    // Parse the url
    if (url != null) {
        let parsedUrl = new URL(url);
        if (parsedUrl.host == "localhost:3000") {
            parsedUrl.port = "5173";
        }
        else {
            // Replace the first part of the host with tabs
            parsedUrl.host = parsedUrl.host.replace(/^[^\.]+/, "tabs");
        }

        url = parsedUrl.toString().slice(0, -1);
    }

    //url = `tabs.${url.split(".", 1)[1]}`


    let flatTable = Object.entries(props.participants.teams).flatMap(([team_uuid, team]) => {
        return Object.entries(team.members).map(([speaker_uuid, speaker]) => {
            return {
                "uuid": speaker.uuid,
                "role": team.name,
                "name": speaker.name,
                "institutions": speaker.institutions,
                "clashes": speaker.clashes,
                "registration_key": speaker.registration_key,
                "path": ["teams", team_uuid, "members", speaker_uuid],
            }
        })
    });

    flatTable.push(...Object.entries(props.participants.adjudicators).map(
        ([adjudicator_uuid, adjudicator]) => {
            return {
                "uuid": adjudicator.uuid,
                "role": "Adjudicator",
                "name": adjudicator.name,
                "institutions": adjudicator.institutions,
                "clashes": adjudicator.clashes,
                "registration_key": adjudicator.registration_key,
                "path": ["adjudicators", adjudicator_uuid],
            }
        }
    ));

    flatTable = flatTable.map((r) => {
        let row = {...r};
        row.institutions = row.institutions.map((i) => i.name).join(", ");
        row.clashes = row.clashes.map((i) => i.participant_name).join(", ");
        return row;
    });

    flatTable.sort((a, b) => a.name > b.name ? 1 : -1);

    let participantsById = {};

    Object.entries(props.participants.teams).forEach(([team_uuid, team]) => {
        Object.entries(team.members).forEach(([speaker_uuid, speaker]) => {
            participantsById[speaker_uuid] = speaker;
        })
    });

    Object.entries(props.participants.adjudicators).forEach(([adjudicator_uuid, adjudicator]) => {
        participantsById[adjudicator_uuid] = adjudicator;
    });

    let selectedParticipant = useMemo(
        () => {
            return selectedParticipantUuid ? participantsById[selectedParticipantUuid] : null;
        }, [selectedParticipantUuid, participantsById]
    );

    let selectedTeam = useMemo(
        () => {
            return selectedTeamUuid ? props.participants.teams[selectedTeamUuid] : null;
        }, [selectedTeamUuid, props.participants.teams]
    );

    let columns = [
            { "key": "role", "header": "Role", "group": true },
            { "key": "name", "header": "Name",  cellFactory: (value, rowIdx, colIdx, rowValue) => {
                return <EditableCell key={colIdx} value={value} onChange={
                    (newName) => {
                        let newParticipant = {... getPath(props.participants, rowValue.path)};
                        newParticipant.name = newName;
                        executeAction("UpdateParticipants", {updated_participants: [newParticipant], tournament_id: tournamentContext.uuid})
                    }
                } />
            } },
            { "key": "institutions", "header": "Institutions" },
            { "key": "clashes", "header": "Clashes" },
    ];

    if (url != null) {
        columns.push({ "key": "registration_key", "header": "Secret", cellFactory: (value, rowIdx, colIdx, rowValue) => {
            return <td><button className="underline text-blue-500" onClick={(evt) => {
                navigator.clipboard.writeText(`${url}/register/${value}`);
                evt.stopPropagation();
            }}>Copy Reg. URL</button></td>
        }});
    }

    return <div className="h-full flex">
        <SortableTable
            data={flatTable}
            rowId="uuid"
            selectedRowId={selectedParticipantUuid}
            onSelectRow={(uuid) => {
                setSelectedParticipantUuid(uuid)
                let p = participantsById[uuid];
                if (p.team_id) {
                    setSelectedTeamUuid(p.team_id);
                }
                else {
                    setSelectedTeamUuid(null);
                }
            }
            }
            columns={columns}
        />
        <div className="h-full flex flex-col overflow-auto">
        {
            selectedParticipant != null &&
           <>
            <ErrorBoundary>
                    <ParticipantDetailView participant={selectedParticipant} onClose={() => {
                        setSelectedParticipantUuid(null)
                        setSelectedTeamUuid(null)
                    }} />

            <ChangeRoleView participant={selectedParticipant} currentType={selectedParticipant.type} allTeams={props.participants.teams} />
            </ErrorBoundary>
            </>
        }

        { selectedParticipant && selectedTeam && <TeamDetailView team={selectedTeam} /> }
        </div>
    </div>
}


function NumberField(props) {
    return <input
    key={props.key}
    className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
    type="number"
    placeholder={props.def.required ? "Required" : ""}
    value={props.value ?? ""} onChange={(event) => {
        if (event.target.value == "") {
            props.onChange(null);
            return;
        }
        let value = parseInt(event.target.value);
        if (!isNaN(value)) {
            props.onChange(value);
        }
    }} />
}

/**
 * A field that can be either one of a set of options, each of which has a different set of fields.
 * 
 * @param {*} props 
 * @returns 
 */
function EitherOrField(props) {
    let [selectionIndex, setSelectionIndex] = useState(0);

    let selectedOption = props.def.options[selectionIndex];

    return <div key={props.key}>
        <div className="flex">
            {props.def.options.map((option, idx) => {
                return <div key={idx} className="mr-2">
                    <input className="mr-1" type="radio" onChange={
                        () => {
                            setSelectionIndex(idx);
                            props.onChange({});
                        }
                    } checked={selectionIndex == idx} />
                    <Label>{option.displayName}</Label>
                </div>
            })}
        </div>
        {
            <Form fields={selectedOption.fields} values={props.value ?? {}} onValuesChanged={(values) => {
                props.onChange({...values, type: props.def.options[selectionIndex].key});
            }} />
        }
    </div>
}

function getFieldFactoryFromType(type) {
    switch(type) {
        case "number":
            return NumberField
        case "either_or":
            return EitherOrField
        default:
            return () => <div>Unknown field type</div>
    }
}

function Label(props) {
    return  <label className="text-gray-700 text-sm font-bold">
        {props.children}
    </label>
}

function Form(props) {
    return <div>
        {
            props.fields.map((fieldDef, fieldIdx) => {
                let factory = getFieldFactoryFromType(fieldDef.type);
                let field = factory({
                    key: fieldIdx,
                    value: props.values[fieldDef.key],
                    def: fieldDef,
                    onChange: (value) => {
                        props.onValuesChanged({...props.values, [fieldDef.key]: value});
                    }
                });
                return <div key={fieldIdx} className="mb-2">
                    <Label>
                        {fieldDef.displayName}
                    </Label>
                    {field}
              </div>
            })
        }
    </div>
}

function validateForm(values, formDef) {
    let errors = {};
    let hasErrors = false;
    for (let fieldDef of formDef) {
        if (fieldDef.type == "either_or") {
            let validationResult = validateEitherOrField(values[fieldDef.key], fieldDef)
            errors[fieldDef.key] = validationResult.errors;
            hasErrors = hasErrors || validationResult.hasErrors;
        }
        if (fieldDef.required && values[fieldDef.key] == null) {
            errors[fieldDef.key] = "Required";
            hasErrors = true;
        }
    }
    return {errors, hasErrors};
}

function validateEitherOrField(values, fieldDef) {
    if (values == null) {
        return {errors: {"type": "Required"}, hasErrors: true};
    }
    let errors = {};
    let selectedField = fieldDef.options.find(option => option.key == values.type);
    if (selectedField == null) {
        errors["type"] = "Required";
    }
    else {
        let optionErrors = validateForm(values, selectedField.fields)
        for (let key in optionErrors.errors) {
            errors[key] = optionErrors.errors[key];
        }    
    }

    return {errors, hasErrors: Object.keys(errors).length > 0};
}

function CSVImportDialog(props) {
    let csvConfigFields = [
        {
            type: "either_or",
            key: "name_column",
            displayName: "Name",
            required: true,
            options: [
                {
                    "displayName": "Full Name",
                    "key": "full",
                    "fields": [
                        {
                            "key": "column",
                            type: "number",
                            required: true
                        },
                    ]
                },
                {
                    "displayName": "First and Last Name",
                    "key": "first_last",
                    "fields": [
                        {
                            "key": "first",
                            type: "number",
                            "displayName": "First Name",
                            required: true
                        },
                        {
                            "key": "last",
                            type: "number",
                            "displayName": "Last Name",
                            required: true
                        },
                    ]
                },
            ]
        },
        {type: "number", required: true, key: "role_column", displayName: "Role"},
        {type: "number", required: true, key: "institutions_column", displayName: "Institution"},
        {type: "number", required: false, key: "clashes_column", displayName: "Clashes"},
    ];

    let [values, setValues] = useState(props.initialConfig || {});

    let {hasErrors} = validateForm(values, csvConfigFields);

    return <div>
        <h1>Select CSV Fields</h1>
        <Form fields={csvConfigFields} values={values} onValuesChanged={(values) => {
            setValues(values);
        }} />
        <div className="w-full flex justify-right justify-end">
            <Button onClick={props.onAbort}>Abort</Button>
            <Button onClick={() => props.onSubmit(values)} disabled={hasErrors} role="primary">Import</Button>
        </div>
    </div>   
}

export function ParticipantOverview() {
    let tournamentContext = useContext(TournamentContext);
    let currentView = {type: "ParticipantsList", tournament_uuid: tournamentContext.uuid};

    let participants = useView(currentView, {"teams": {}, "adjudicators": {}});

    let [importDialogState, setImportDialogState] = useState(null);

    return <div className="flex flex-col h-full w-full" onKeyDown={(e) => {
        if (e.nativeEvent.key == "Escape") {
            setImportDialogState(null);
        }
    }}>
        <ModalOverlay open={importDialogState !== null}>
        {
            importDialogState !== null ? <CSVImportDialog onAbort={() => setImportDialogState(null)} onSubmit={
                (values) => {
                    executeAction(
                        "UploadParticipantsList", {
                            tournament_id: tournamentContext.uuid,
                            path: importDialogState.file,
                            parser_config: values
                        }
                    );
                    setImportDialogState(null);
                }
            } initialConfig={importDialogState.proposedConfig} /> : []
        }
        </ModalOverlay>
        
        <div className="min-h-0">
            {
                Object.entries(participants.teams).length + Object.entries(participants.adjudicators).length > 0 ?
                <ParticipantTable participants={participants} />
                :
                <div className="flex flex-col items-center justify-center h-full">
                    <div className="text-2xl text-gray-500">No participants</div>
                    <div className="text-gray-500">Click the button below to import a participants from a csv file</div>
                </div>
            }
        </div>
        <div className="flex-none w-full h-12 bg-gray-200">
            <ParticipantImportDialogButton />

            <button onClick={
                () => {
                    save(
                        {
                            defaultPath: "qrcodes.pdf",
                            filters: [
                                {
                                    name: "PDF",
                                    extensions: ["pdf"]
                                }
                            ]
                        }
                    ).then((result) => {
                        if (result !== null) {
                            invoke(
                                "save_participant_qr_codes",
                                {
                                    tournamentId: tournamentContext.uuid,
                                    outPath: result
                                }
                            )
                        }
                    })
                }
            }>
                Export QR Codes…
            </button>
        </div>
    </div>
}