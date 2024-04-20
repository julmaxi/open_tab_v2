//@ts-check

import React, { useCallback, useContext } from "react";
import { useState } from "react";
import { executeAction } from "./Action";
import { getPath, useView } from "./View";
import { TournamentContext } from "./TournamentContext";

import ModalOverlay from "./Modal";
import Button from "./Button";
import { SortableTable, EditableCell } from "./SortableTable";
import ComboBox from "./ComboBox";
import { useEffect } from "react";
import _ from "lodash";
import { confirm } from '@tauri-apps/api/dialog';

import {
    BrowserRouter as Router,
} from "react-router-dom";
import { openImportDialog } from "./openImportDialog";
import { ErrorHandlingContext } from "./Action";


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
    return <label className="text-gray-700 text-sm font-bold">
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
        {type: "number", required: false, key: "anonymity_column", displayName: "Anonymity"},
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

export function ParticipantImportDialogButton({buttonFactory, buttonProps: buttonProps = {}}) {
    
    let [importDialogState, setImportDialogState] = useState(null);
    let tournamentContext = useContext(TournamentContext);
    let errorContext = useContext(ErrorHandlingContext);

    let ButtonFactory = "button";
    if (buttonFactory != null) {
        ButtonFactory = buttonFactory;
    }

    return <>
        <ButtonFactory {...buttonProps} onClick={() => {
            openImportDialog().then((result) => {
                if (result !== null) {
                    setImportDialogState(result);
                }
            });
        }} className="h-full">Import…</ButtonFactory>        


        <ModalOverlay open={importDialogState !== null}>
            {
                importDialogState !== null ? <CSVImportDialog onAbort={() => setImportDialogState(null)} onSubmit={
                    (values) => {
                        executeAction(
                            "UploadParticipantsList", {
                                tournament_id: tournamentContext.uuid,
                                path: importDialogState.file,
                                parser_config: values
                            },
                            errorContext.handleError
                        );
                        setImportDialogState(null);
                    }
                } initialConfig={importDialogState.proposedConfig} /> : []
            }
        </ModalOverlay>
    </>;

}

export function ParticipantOverview() {
    let tournamentContext = useContext(TournamentContext);
    let currentView = {type: "ParticipantsList", tournament_uuid: tournamentContext.uuid};

    let participants = useView(currentView, {"teams": {}, "adjudicators": {}});


    return <div className="flex flex-col h-full w-full">
        
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
        </div>
    </div>
}