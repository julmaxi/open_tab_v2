import React, { useState } from "react";
import _ from "lodash";
import { open } from '@tauri-apps/plugin-dialog';
import { readFile, BaseDirectory } from '@tauri-apps/plugin-fs';


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

function BinaryRadioField(props) {
    let id = _.uniqueId("binary_radio");
    return <div className="flex items-center">
        <input id={`${id}-yes`} className="mr-1" type="radio" onChange={() => props.onChange(true)} checked={props.value == true} />
        <label htmlFor={`${id}-yes`} className="mr-4">Yes</label>
        <input id={`${id}-no`} className="mr-1" type="radio" onChange={() => props.onChange(false)} checked={!props.value} />
        <label htmlFor={`${id}-no`}>No</label>
    </div>;
}

function DateTimeField(props) {
    return <input
        key={props.key}
        className="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
        type="datetime-local"
        placeholder={props.def.required ? "Required" : ""}
        value={props.value ?? ""} onChange={(event) => {
            props.onChange(event.target.value);
        }} />;
}

function guessImageType(data) {
    if (data[0] == 0xFF && data[1] == 0xD8) {
        return "image/jpeg";
    }
    if (data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47) {
        return "image/png";
    }
    return null;
}

function ImageSelectorField(props) {
    return <div>
        <div className="border rounded shadow-inner w-16 h-16 flex items-center p-1 cursor-pointer" onClick={async () => {
            let file = await open({ accept: "image/*" });
            if (file) {
                let content = await readFile(file);
                let type = guessImageType(content);
                let blob = new Blob([content], { type: type });
                let reader = new FileReader();
                reader.onload = (event) => {
                    props.onChange(event.target.result);
                }
                reader.readAsDataURL(blob);
            }
        }}>
            {props.value ? <img className="w-full h-full object-contain" src={props.value} /> : <span className="text-gray-400 text-sm">No image</span>}
        </div>
        {props.value && <button className="text-sm text-gray-400" onClick={() => props.onChange(null)}>Clear</button>}
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
        case "bool":
            return BinaryRadioField;
        case "datetime":
            return DateTimeField;
        case "image":
            return ImageSelectorField;
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
        if (fieldDef.required && (values[fieldDef.key] === null || values[fieldDef.key] === "")) {
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