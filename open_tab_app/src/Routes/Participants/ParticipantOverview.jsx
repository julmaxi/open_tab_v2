//@ts-check

import React, { useContext, useEffect, useMemo, useState } from "react";
import { ErrorHandlingContext, executeAction } from "../../Action";
import { TournamentContext } from "../../TournamentContext";
import { getPath, useView } from "../../View";

import { save } from '@tauri-apps/plugin-dialog';
import { EditableCell, SortableTable } from "../../SortableTable";
import ModalOverlay from "../../UI/Modal";

import { invoke } from "@tauri-apps/api/core";
import ErrorBoundary from "../../ErrorBoundary";
import { ParticipantImportDialogButton } from "./ParticipantImportDialog";
import { SplitDetailView } from "../../UI/SplitDetailView";
import { CSVImportDialog } from "./CSVImportDialog";
import { ChangeRoleView, ParticipantDetailView } from "./ParticipantDetailView";
import { TeamDetailView } from "./TeamDetailView";
import Button from "../../UI/Button";
import { RowBlockerContext, BlockLease, RowBlockManager } from "./RowBlocker";
import AddParticipantDialog from "./AddParticipantDialog";
import { Toolbar, ToolbarButton } from "../../UI/Toolbar";
import { appDataDir } from "@tauri-apps/api/path";

function ParticipantTable({ participants }) {
    let [selectedParticipantUuid, setSelectedParticipantUuid] = useState(null);
    let [selectedTeamUuid, setSelectedTeamUuid] = useState(null);

    let tournamentContext = useContext(TournamentContext);

    let statusView = useView({ type: "TournamentStatus", tournament_uuid: tournamentContext.uuid }, null);

    let url = statusView ? statusView.remote_url : null;
    //FIXME: This should come from the server somehow.
    //This way, this will not work if the remote server does not
    //call its frontend tabs.servername
    //For now, not worth the additional complexity to fix.
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

    let flatTable = Object.entries(participants.teams).flatMap(([team_uuid, team]) => {
        return Object.entries(team.members).map(([speaker_uuid, speaker]) => {
            return {
                "uuid": speaker.uuid,
                "role": team.name,
                "name": speaker.name,
                "institutions": speaker.institutions,
                "clashes": speaker.clashes,
                "registration_key": speaker.registration_key,
                "path": ["teams", team_uuid, "members", speaker_uuid],
                "break_category": speaker.break_category_id !== null ? (participants.break_categories[speaker.break_category_id]?.name || "<Unknown Break Category>") : "-",
            }
        })
    });

    flatTable.push(...Object.entries(participants.adjudicators).map(
        ([adjudicator_uuid, adjudicator]) => {
            return {
                "uuid": adjudicator.uuid,
                "role": "Adjudicator",
                "name": adjudicator.name,
                "institutions": adjudicator.institutions,
                "clashes": adjudicator.clashes,
                "registration_key": adjudicator.registration_key,
                "path": ["adjudicators", adjudicator_uuid],
                "break_category": adjudicator.break_category_id !== null ? (participants.break_categories[adjudicator.break_category_id]?.name || "<Unknown Break Category>") : "-",
            }
        }
    ));

    flatTable = flatTable.map((r) => {
        let row = { ...r };
        row.institutions = row.institutions.map((i) => participants.institutions[i.uuid]?.name || "<Unknown Institution>").join(", ");
        let clashes = [];
        // Assumes clashes are sorted by id, with outgoing clashes first
        for (let idx = 0; idx < r.clashes.length; idx++) {
            let clash = r.clashes[idx];
            if (idx > 0) {
                if (r.clashes[idx - 1].participant_uuid == clash.participant_uuid) {
                    clashes[clashes.length - 1] += "⇔"
                    continue;
                }
            }
            if (clash.direction == "Incoming") {
                clashes.push(clash.participant_name + "⇐");
            }
            else {
                clashes.push(clash.participant_name);
            }
        }
        row.clashes = clashes.join(", ")
        return row;
    });

    flatTable.sort((a, b) => a.name > b.name ? 1 : -1);

    let participantsById = {};

    Object.entries(participants.teams).forEach(([, team]) => {
        Object.entries(team.members).forEach(([speaker_uuid, speaker]) => {
            participantsById[speaker_uuid] = speaker;
        })
    });

    Object.entries(participants.adjudicators).forEach(([adjudicator_uuid, adjudicator]) => {
        participantsById[adjudicator_uuid] = adjudicator;
    });

    let selectedParticipant = useMemo(
        () => {
            return selectedParticipantUuid ? participantsById[selectedParticipantUuid] : null;
        }, [selectedParticipantUuid, participantsById]
    );

    let selectedTeam = useMemo(
        () => {
            return selectedTeamUuid ? participants.teams[selectedTeamUuid] : null;
        }, [selectedTeamUuid, participants.teams]
    );

    let columns = [
        { "key": "role", "header": "Role", "group": true },
        {
            "key": "name", "header": "Name", cellFactory: (value, rowIdx, colIdx, rowValue) => {
                return <EditableCell key={colIdx} value={value} onChange={
                    (newName) => {
                        let newParticipant = { ...getPath(participants, rowValue.path) };
                        newParticipant.name = newName;
                        executeAction("UpdateParticipants", { updated_participants: [newParticipant], tournament_id: tournamentContext.uuid })
                    }
                } />
            }
        },
        { "key": "institutions", "header": "Institutions" },
        { "key": "clashes", "header": "Clashes" },
        { "key": "break_category", "header": "Break Cat." },
    ];

    if (url != null) {
        columns.push({
            "key": "registration_key", "header": "Secret", cellFactory: (value, rowIdx, colIdx) => {
                return <td key={colIdx}><button className="underline text-blue-500" onClick={(evt) => {
                    navigator.clipboard.writeText(`${url}/register/${value}`);
                    evt.stopPropagation();
                }}>Copy Reg. URL</button></td>
            }
        });
    }

    let rowBlockManager = useMemo(
        () => {
            return new RowBlockManager()
        },
        [flatTable]
    );

    useEffect(() => {
        // If the selected participant changes from a team to an adjudicator (or changes team),
        // the selected team should be cleared.
        // It is repeated below, to prevent flickering in all other cases.
        let p = participantsById[selectedParticipantUuid];
        if (p && p.team_id) {
            setSelectedTeamUuid(p.team_id);
        }
        else {
            setSelectedTeamUuid(null);
        }
    }, [selectedParticipantUuid, participantsById]);

    return <RowBlockerContext.Provider value={rowBlockManager}>
    <div className="h-full flex">
        <SplitDetailView>
            <SortableTable
                data={flatTable}
                rowId="uuid"
                selectedRowId={selectedParticipantUuid}
                onSelectRow={async (uuid) => {
                    if (
                        !rowBlockManager.isBlocked() ||
                        await confirm("You have unsaved changes. Are you sure you want to switch participants? Changes will be lost.")
                    ) {
                        setSelectedParticipantUuid(uuid)
                        let p = participantsById[uuid];
                        if (p.team_id) {
                            setSelectedTeamUuid(p.team_id);
                        }
                        else {
                            setSelectedTeamUuid(null);
                        }                            
                    }
                }}
                columns={columns}
            />
            {
                selectedParticipant != null &&
                <ErrorBoundary>
                    <div className="p-1">
                        <ParticipantDetailView
                            participant={selectedParticipant}
                            institutions={participants.institutions}
                            break_categories={participants.break_categories}
                            onClose={() => {
                                setSelectedParticipantUuid(null)
                                setSelectedTeamUuid(null)
                            }}
                        />

                        <ChangeRoleView participant={selectedParticipant} currentType={selectedParticipant.type} allTeams={participants.teams} />
                        {selectedParticipant && selectedTeam && <TeamDetailView team={selectedTeam} />}

                        <div className="">
                            <h2 className="font-bold text-red-500">Danger Zone</h2>
                            <Button role="danger" onClick={() => {
                                confirm("Are you sure you want to delete this participant? You can not undo this.").then((result) => {
                                    if (result === true) {
                                        executeAction("UpdateParticipants", { tournament_id: tournamentContext.uuid, updated_participants: [], deleted_participants: [selectedParticipant.uuid] });
                                    }
                                });
                            }}>
                                Delete
                            </Button>
                        </div>

                    </div>
                </ErrorBoundary>
            }
        </SplitDetailView>
    </div>
    </RowBlockerContext.Provider>
}

export function ParticipantOverview() {
    let tournamentContext = useContext(TournamentContext);

    let errorContext = useContext(ErrorHandlingContext);

    let currentView = { type: "ParticipantsList", tournament_uuid: tournamentContext.uuid };

    let participants = useView(currentView, { "teams": {}, "adjudicators": {}, "institutions": {}, "break_categories": [] });

    let [addParticipantDialogOpen, setAddParticipantDialogOpen] = useState(false);

    return <div className="flex flex-col h-full w-full">
        <ModalOverlay open={addParticipantDialogOpen}>
            <AddParticipantDialog onAbort={() => setAddParticipantDialogOpen(false)} onSubmit={(values) => {
                let role = null;
                console.log(values);

                if (values.role.type == "adjudicator") {
                    role = {
                        chair_skill: values.role.chair_skill,
                        panel_skill: values.role.panel_skill,
                        unavailable_rounds: [],
                        type: "Adjudicator"
                    }
                }
                else {
                    role = {
                        team_id: values.role.team,
                        type: "Speaker"
                    }
                }
                executeAction(
                    "UpdateParticipants",
                    {
                        tournament_id: tournamentContext.uuid,
                        added_participants: [{
                            name: values.name,
                            tournament_id: tournamentContext.uuid,
                            institutions: [],
                            clashes: [],
                            registration_key: null,
                            is_anonymous: false,
                            ...role,
                        }],
                        deleted_participants: [],
                    }
                );
                setAddParticipantDialogOpen(false);
            }} />
        </ModalOverlay>

        <div className="min-h-0 flex-1">
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
        <Toolbar>
            <ParticipantImportDialogButton buttonFactory={({children, onClick}) => <ToolbarButton icon={"upload"} onClick={onClick}>{children}</ToolbarButton>} />

            <ToolbarButton icon="add" onClick={() => setAddParticipantDialogOpen(true)}>Add Participant…</ToolbarButton>

            <ToolbarButton icon="qr" onClick={
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
            </ToolbarButton>
        </Toolbar>
    </div>
}