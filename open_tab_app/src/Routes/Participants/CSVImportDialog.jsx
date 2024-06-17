import React from "react";
import { useState } from "react";
import Button from "../../UI/Button";
import { Form, validateForm } from "../../UI/Form";


export function CSVImportDialog(props) {
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
        { type: "", required: true, key: "role_column", displayName: "Role" },
        { type: "number", required: true, key: "institutions_column", displayName: "Institution" },
        { type: "number", required: false, key: "clashes_column", displayName: "Clashes" },
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
