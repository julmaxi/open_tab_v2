import React, { useState } from "react";

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

function TextField(props) {
    return <input
        key={props.key}
        className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
        type="text"
        placeholder={props.def.required ? "Required" : ""}
        value={props.value ?? ""} onChange={(event) => {
            props.onChange(event.target.value);
        }} />;
}

function MultipleChoiceField(props) {
    return <select
        key={props.key}
        className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
        value={props.value}
        onChange={(event) => {
            props.onChange(event.target.value);
        }}>
        <option value="">Select...</option>
        {
            props.def.options.map((option, idx) => {
                return <option key={idx} value={option.key}>{option.displayName}</option>;
            })
        }
    </select>;
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
        case "text":
            return TextField;
        case "multiple_choice":
            return MultipleChoiceField;
        default:
            return () => <div>Unknown field type</div>;
    }
}

function Label(props) {
    return <label className="text-gray-700 text-sm font-bold">
        {props.children}
    </label>;
}

export function Form(props) {
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
export function validateForm(values, formDef) {
    let errors = {};
    let hasErrors = false;
    for (let fieldDef of formDef) {
        if (fieldDef.type == "either_or") {
            let validationResult = validateEitherOrField(values[fieldDef.key], fieldDef);
            errors[fieldDef.key] = validationResult.errors;
            hasErrors = hasErrors || validationResult.hasErrors;
        }
        if (fieldDef.required && (values[fieldDef.key] == null || values[fieldDef.key] == "")) {
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