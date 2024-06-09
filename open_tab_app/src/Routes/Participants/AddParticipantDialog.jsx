import { useContext, useState } from "react";
import { TournamentContext } from "../../TournamentContext";

import { useView } from "../../View";

import { Form, validateForm } from "./Form";

import Button from "../../UI/Button";

export default function AddParticipantDialog(props) {
    let tournamentContext = useContext(TournamentContext);
    let { teams } = useView({type: "ParticipantsList", tournament_uuid: tournamentContext.uuid}, { "teams": {}, "adjudicators": {} });

    let teamList = Object.values(teams).map(
        team => ({ key: team.uuid, displayName: team.name })
    );

    let csvConfigFields = [
        { type: "text", required: true, key: "name", displayName: "Name" },
        {
            type: "either_or",
            key: "role",
            displayName: "Role",
            required: true,
            options: [
                {
                    "displayName": "Adjudicator",
                    "key": "adjudicator",
                    "fields": [
                        {
                            "key": "chair_skill",
                            type: "number",
                            "displayName": "Chair Skill",
                            required: true
                        },
                        {
                            "key": "panel_skill",
                            type: "number",
                            "displayName": "Panel Skill",
                            required: true
                        },
                    ]
                },
                {
                    "displayName": "Speaker",
                    "key": "speaker",
                    "fields": [
                        {
                            "key": "team",
                            type: "multiple_choice",
                            required: true,
                            options: teamList
                        },
                    ]
                },
            ]
        },
    ];

    let [values, setValues] = useState(props.initialConfig || {});

    let { hasErrors } = validateForm(values, csvConfigFields);

    return <div>
        <h1>Select CSV Fields</h1>
        <Form fields={csvConfigFields} values={values} onValuesChanged={(values) => {
            setValues(values);
        }} />
        <div className="w-full flex justify-right justify-end">
            <Button onClick={props.onAbort}>Abort</Button>
            <Button onClick={() => props.onSubmit(values)} disabled={hasErrors} role="primary">Import</Button>
        </div>
    </div>;
}
