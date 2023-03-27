//@ts-check

import React, { useCallback, useContext } from "react";
import { useState, useMemo } from "react";
import { executeAction } from "./Action";
import { getPath, useView } from "./View";
import { open } from '@tauri-apps/api/dialog';
import { TournamentContext } from "./TournamentContext";


function EditableCell(props) {
    let [edit, setEdit] = useState(false);

    let [localValue, setLocalValue] = useState(null);

    return <td onDoubleClick={() => {
        if (!edit) {
            setEdit(true);
        }
    }}>
        {edit ? <input type="text" autoFocus value={localValue !== null ? localValue : props.value} onChange={
            (event) => {
                setLocalValue(event.target.value);
            }
        } onKeyDown={
            (event) => {
                if (event.key === "Enter") {
                    setLocalValue(null);
                    setEdit(false);
                    props.onChange(localValue);
                    event.preventDefault()
                }
                else if (event.key == "Escape") {
                    setLocalValue(null);
                    setEdit(false);
                    event.preventDefault();
                }
            }
        } onBlur= {
            (event) => {
                let value = event.target.value;
                setLocalValue(null);
                setEdit(false);
                props.onChange(value);
            }
        } onFocus = {
            (event) => {
                event.target.select();
            }
        }/> : props.value}
    </td>
}

function SortableTable(props) {
    let [sortOrder, setSortOrder] = useState(null);
    let [selectedRow, setSelectedRow] = useState(null);

    let {orderedRows, groups} = useMemo(
        () => {
            let orderedRows = [...props.data];
            if (sortOrder !== null) {
                orderedRows.sort((a, b) => (a[sortOrder.key] > b[sortOrder.key] ? 1 : -1) * sortOrder.direction);
            }

            let colGroups = new Map();
            for (let col of props.columns) {
                if (col.group) {
                    colGroups.set(col, [])
                }
            }

            for (let i = 0; i < orderedRows.length; i++) {
                let row = orderedRows[i];
                for (let [col, groups] of colGroups) {
                    let currentGroup = groups[groups.length - 1];
                    if (currentGroup === undefined || currentGroup.val !== row[col.key]) {
                        currentGroup = {val: row[col.key], size: 1, start: i};
                        groups.push(currentGroup);
                    }
                    else {
                        currentGroup.size += 1;
                        groups.push(currentGroup);
                    }
                }
            }
            
            return {orderedRows, groups: colGroups};
        }, [props.data, sortOrder]
    );

    function handleSort(column_key) {
        return (event) => {
            if (sortOrder !== null && column_key == sortOrder.key) {
                setSortOrder({ key: column_key, direction: -sortOrder.direction });
            }
            else {
                setSortOrder({ key: column_key, direction: 1 });
            }
        }
    }

    return <table className="w-full">
        <thead>
            <tr className="text-left">
                {props.columns.map((column, idx) => {
                    return <th key={idx} className="" onClick={handleSort(column.key)}>{column.header}</th>
                })}
            </tr>
        </thead>
        <tbody>
            {orderedRows.map((row, rowIdx) => {
                let className = [selectedRow === rowIdx ? "bg-sky-500" : (rowIdx % 2 == 0 ? "bg-gray-100" : "bg-white")];

                return <tr key={row[props.row_id]} className={className} onClick={() => setSelectedRow(rowIdx)}>
                    {
                        props.columns.filter(col => !col.group || groups.get(col)[rowIdx].start == rowIdx).map(
                            (column, idx) => {
                                let val = row[column.key];
                                let rowSpan = groups.get(column)?.[rowIdx]?.size ?? 1;

                                return column.cellFactory !== undefined ? column.cellFactory(val, rowIdx, idx, row) : <td rowSpan={rowSpan} key={idx} className="">{val}</td>
                            }
                        )
                    }
                </tr>
            })}
        </tbody>
    </table>
}


function ParticipantTable(props) {
    let flatTable = Object.entries(props.participants.teams).flatMap(([team_uuid, team]) => {
        return Object.entries(team.members).map(([speaker_uuid, speaker]) => {
            return {
                "uuid": speaker.uuid,
                "role": team.name,
                "name": speaker.name,
                "institutions": speaker.institutions,
                "path": ["teams", team_uuid, "members", speaker_uuid]
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
                "path": ["adjudicators", adjudicator_uuid]
            }
        }
    ))

    flatTable = flatTable.map((r) => {
        let row = {...r};
        row.institutions = row.institutions.map((i) => i.name).join(", ");
        return row;
    });

    return <SortableTable data={flatTable} row_id="uuid" columns={
        [
            { "key": "role", "header": "Role", "group": true },
            { "key": "name", "header": "Name",  cellFactory: (value, rowIdx, colIdx, rowValue) => {
                return <EditableCell key={colIdx} value={value} onChange={
                    (newName) => {
                        console.log(rowValue);
                        let newParticipant = {... getPath(props.participants, rowValue.path)};
                        console.log(newParticipant);
                        newParticipant.name = newName;
                        executeAction("UpdateParticipants", {updated_participants: [newParticipant], tournament_id: "00000000-0000-0000-0000-000000000001"})
                    }
                } />
            } },
            { "key": "institutions", "header": "Institutions" }
        ]
    } />
}


function DialogWindow(props) {
    return <div className="bg-white p-8">
        {props.children}
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
            key: "name",
            displayName: "Name",
            required: true,
            options: [
                {
                    "displayName": "Full Name",
                    "key": "full_name",
                    "fields": [
                        {
                            "key": "full_name",
                            type: "number",
                            required: true
                        },
                    ]
                },
                {
                    "displayName": "First and Last Name",
                    "key": "first_last_name",
                    "fields": [
                        {
                            "key": "first_name",
                            type: "number",
                            "displayName": "First Name",
                            required: true
                        },
                        {
                            "key": "last_name",
                            type: "number",
                            "displayName": "Last Name",
                            required: true
                        },
                    ]
                },
            ]
        },
        {type: "number", required: true, key: "institution", displayName: "Institution"},
        {type: "number", required: false, key: "clashes", displayName: "Clashes"},
    ];

    let [values, setValues] = useState({});

    let {hasErrors} = validateForm(values, csvConfigFields);

    return <DialogWindow>
        <h1>Select CSV Fields</h1>
        <Form fields={csvConfigFields} values={values} onValuesChanged={(values) => {
            setValues(values);
        }} />
        <div className="w-full flex justify-right justify-end">
            <Button onClick={props.onAbort}>Abort</Button>
            <Button onClick={() => props.onSubmit(values)} disabled={hasErrors} role="primary">Import</Button>
        </div>
        
    </DialogWindow>   
}


function Button(props) {
    let baseStyle = "ml-1 p-1 text-white rounded";

    let bgColor = "bg-gray-500";
    if (props.role == "primary") {
        bgColor = "bg-blue-500";
    }
    else if (props.role == "secondary") {
        bgColor = "bg-gray-500";
    }

    if (props.disabled) {
        bgColor = "bg-gray-300";
    }

    return <button className={`${baseStyle} ${bgColor}`} disabled={props.disabled} onClick={props.onClick}>{props.children}</button>
}


function ModalOverlay(props) {
    
}


export function ParticipantOverview() {
    let tournamentContext = useContext(TournamentContext);
    let currentView = {type: "ParticipantsList", tournament_uuid: tournamentContext.uuid};

    let participants = useView(currentView, {"teams": {}, "adjudicators": {}});
    console.log(participants);

    let [importDialogState, setImportDialogState] = useState(true);

    let openImportDialog = useCallback(async () => {
        const selected = await open({
            multiple: false,
            filters: [{
                name: 'csv',
                extensions: ['csv']
            }]
        });
        if (selected !== null) {
            setImportDialogState({
                file: selected[0]
            });
        }
    }, []);

    return <div className="flex flex-col h-full" onKeyDown={(e) => {
        if (e.nativeEvent.key == "Escape") {
            setImportDialogState(null);
        }
    }}>
        {
            importDialogState ?
            <div className="fixed top-0 left-0 z-50 w-full overflow-x-hidden overflow-y-hidden inset-0 h-full grid place-items-center">
                <div tabIndex={-1} className={"absolute top-0 left-0 w-full h-full bg-opacity-50 bg-black"} onClick={() => setImportDialogState(null)} />                
                <div className="z-10">
                    <CSVImportDialog onAbort={() => setImportDialogState(null)} onSubmit={
                        (values) => console.log(values)
                    } />    
                </div>
            </div>
            :
            []
        }
        <div className="flex-1 overflow-scroll">
            <ParticipantTable participants={participants} />
        </div>
        <div className="flex-none w-full h-12 bg-gray-200">
            <button onClick={openImportDialog} className="h-full">Import…</button>
        </div>
    </div>
}