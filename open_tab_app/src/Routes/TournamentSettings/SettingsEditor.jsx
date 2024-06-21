import React, { useEffect, useState, useContext } from 'react';
import { Form, validateForm } from "../../UI/Form";
import ModalOverlay from "../../UI/Modal";
import Button from '../../UI/Button';
import { invoke } from '@tauri-apps/api/tauri';
import { TournamentContext } from '../../TournamentContext';
import { useView } from '../../View';
import { set } from 'lodash';

export default function SettingsEditorButton() {
    const [isOpen, setIsOpen] = useState(false);

    let tournamentContext = useContext(TournamentContext);
    let tournamentId = tournamentContext.uuid;

    return <div>
        <Button onClick={() => setIsOpen(true)}>Edit Publication Settings</Button>
        <ModalOverlay open={isOpen} onAbort={() => {
            setIsOpen(false);
        }} closeOnOverlayClick={true}>
            {
                isOpen ? <SettingsEditor tournamentId={tournamentId} onClose={() => setIsOpen(false)} /> : []
            }
        </ModalOverlay>
    </div>
}

function SettingsEditor({tournamentId, onClose}) {
    const [values, setValues] = useState({});
    const [formIsValid, setFormIsValid] = useState(false);

    const [loadState, setLoadState] = useState("loading");

    let { name } = useView({type: "TournamentStatus", tournament_uuid: tournamentId}, { "name": "" });

    useEffect(() => {
        async function fetchSettings() {
            setLoadState("loading");

            try {
                let response = await invoke("send_tournament_api_request", {tournamentId: tournamentId, request: {
                    method: "GET",
                    url: `/api/tournament/${tournamentId}/settings`
                }
                });
                if (response.status === 200) {
                    let body = JSON.parse(response.body);
                    let values = body;
    
                    setValues(values);
                    setLoadState("loaded");
                }
                else {
                    console.error(response);
                    setLoadState("error");
                }    
            }
            catch (e) {
                console.error(e);
                setLoadState("error");
            }
        }

        fetchSettings();        
    }, [tournamentId]);

    return loadState === "loaded" || loadState == "saving"
                ?
                <>
                    <SettingsEditorForm defaultTournamentName={name} values={values} onChangeIsValid={
                        (isValid) => {
                            setFormIsValid(isValid);
                        }
                    } onValuesChanged={(v, isValid) => {
                        setValues(v)
                        setFormIsValid(isValid);
                    }} />
        
                    <Button onClick={() => onClose()}>Close</Button>
                    <Button role="primary" onClick={async () => {
                        let requestBody = {
                            list_publicly: values.list_publicly,
                            public_name: values.public_name,
                            show_participants: values.show_participants,
                            show_motions: values.show_motions,
                            show_draws: values.show_draws,
                            show_tab: values.show_tab,
                            start_date: values.start_date && new Date(Date.parse(values.start_date)).toISOString().slice(0, -1),
                            end_date: values.end_date && new Date(Date.parse(values.end_date)).toISOString().slice(0, -1),
                            image: values.image
                        };
                        console.log(requestBody);

                        let response = await invoke("send_tournament_api_request", {tournamentId: tournamentId, request: {
                                body: JSON.stringify(requestBody),
                                method: "PATCH",
                                url: `/api/tournament/${tournamentId}/settings`
                            }
                        });
        
                        if (response.status === 200) {
                            onClose();
                        }
                        else {
                            console.error(response);
                        }
                    }} disabled={!formIsValid}>Save</Button>
                </>
                : <>
                    {loadState === "loading" ? "Loading..." : <p className='font-bold text-red-500'>Error while trying to contact remote</p>}
                    <Button onClick={() => onClose()}>Close</Button>
                </>
}

function SettingsEditorForm({values, defaultTournamentName, onValuesChanged, onChangeIsValid}) {

    let fields = [
        {
            key: "public_name",
            type: "text",
            displayName: "Public Name",
            required: true
        },
        {
            key: "list_publicly",
            type: "bool",
            displayName: "List Publicly",
            required: true
        },
        {
            key: "show_participants",
            type: "bool",
            displayName: "Show Participants",
            required: true
        },
        {
            key: "show_motions",
            type: "bool",
            displayName: "Show Motions",
            required: true
        },
        {
            key: "show_draws",
            type: "bool",
            displayName: "Show Draws",
            required: true
        },
        {
            key: "show_tab",
            type: "bool",
            displayName: "Show Tab",
            required: true
        },
        {
            key: "start_date",
            type: "datetime",
            displayName: "Start Date",
            required: false
        },
        {
            key: "end_date",
            type: "datetime",
            displayName: "End Date",
            required: false
        },
        {
            key: "location",
            type: "text",
            displayName: "Location",
            required: false
        },
        {
            key: "image",
            type: "image",
            displayName: "Thumbnail Image",
        }
    ];

    return <div>
        <div>
            { values["list_publicly"] && !(values["start_date"] && values["end_date"]) && <p className='text-sm text-red-500 max-w-64'>If you want your tournament to appear on the front page, you need to specify a start and end date for your tournament.</p> }
        </div>
        <Form
            fields={fields}
            values={values}
            onValuesChanged={(values) => {
                onValuesChanged(values, !validateForm(values, fields).hasErrors);
            }}
        />
    </div>
}