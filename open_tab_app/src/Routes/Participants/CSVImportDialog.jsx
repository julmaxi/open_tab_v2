import React from "react";
import { useState } from "react";
import Button from "../../UI/Button";

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
        }} />;
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
                    <input className="mr-1" type="radio" onChange={() => {
                        setSelectionIndex(idx);
                        props.onChange({});
                    }} checked={selectionIndex == idx} />
                    <Label>{option.displayName}</Label>
                </div>;
            })}
        </div>
        {<Form fields={selectedOption.fields} values={props.value ?? {}} onValuesChanged={(values) => {
            props.onChange({ ...values, type: props.def.options[selectionIndex].key });
        }} />}
    </div>;
}
function getFieldFactoryFromType(type) {
    switch (type) {
        case "number":
            return NumberField;
        case "either_or":
            return EitherOrField;
        default:
            return () => <div>Unknown field type</div>;
    }
}
function Label(props) {
    return <label className="text-gray-700 text-sm font-bold">
        {props.children}
    </label>;
}
function Form(props) {
    return <div>
        {props.fields.map((fieldDef, fieldIdx) => {
            let factory = getFieldFactoryFromType(fieldDef.type);
            let field = factory({
                key: fieldIdx,
                value: props.values[fieldDef.key],
                def: fieldDef,
                onChange: (value) => {
                    props.onValuesChanged({ ...props.values, [fieldDef.key]: value });
                }
            });
            return <div key={fieldIdx} className="mb-2">
                <Label>
                    {fieldDef.displayName}
                </Label>
                {field}
            </div>;
        })}
    </div>;
}
function validateForm(values, formDef) {
    let errors = {};
    let hasErrors = false;
    for (let fieldDef of formDef) {
        if (fieldDef.type == "either_or") {
            let validationResult = validateEitherOrField(values[fieldDef.key], fieldDef);
            errors[fieldDef.key] = validationResult.errors;
            hasErrors = hasErrors || validationResult.hasErrors;
        }
        if (fieldDef.required && values[fieldDef.key] == null) {
            errors[fieldDef.key] = "Required";
            hasErrors = true;
        }
    }
    return { errors, hasErrors };
}
function validateEitherOrField(values, fieldDef) {
    if (values == null) {
        return { errors: { "type": "Required" }, hasErrors: true };
    }
    let errors = {};
    let selectedField = fieldDef.options.find(option => option.key == values.type);
    if (selectedField == null) {
        errors["type"] = "Required";
    }
    else {
        let optionErrors = validateForm(values, selectedField.fields);
        for (let key in optionErrors.errors) {
            errors[key] = optionErrors.errors[key];
        }
    }

    return { errors, hasErrors: Object.keys(errors).length > 0 };
}
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
        { type: "number", required: true, key: "role_column", displayName: "Role" },
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
