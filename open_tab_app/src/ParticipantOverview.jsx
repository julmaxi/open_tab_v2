//@ts-check

import React from "react";
import { useState, useMemo } from "react";
import { useView } from "./View";


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
                setLocalValue(null);
                setEdit(false);
                props.onChange(localValue);
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
                    console.log(groups, currentGroup, row[col.key])
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
    console.log("ABC", orderedRows, groups);

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

                                return column.isEditable ? <EditableCell key={idx} value={val} /> : <td rowSpan={rowSpan} key={idx} className="">{val}</td>
                            }
                        )
                    }
                </tr>
            })}
        </tbody>
    </table>
}


function ParticipantTable(props) {
    let flatTable = [];
    for (let team of props.participants.teams) {
        for (let speaker of team.members) {
            flatTable.push({
                "uuid": speaker.uuid,
                "role": team.name,
                "name": speaker.name,
                "institutions": speaker.institutions
            });
        }
    }

    for (let adjudicator of props.participants.adjudicators) {
        flatTable.push({
            "uuid": adjudicator.uuid,
            "role": "Adjudicator",
            "name": adjudicator.name,
            "institutions": adjudicator.institutions
        });
    }

    return <SortableTable data={flatTable} row_id="uuid" columns={
        [
            { "key": "role", "header": "Role", "group": true },
            { "key": "name", "header": "Name", isEditable: true },
            { "key": "institutions", "header": "Institutions" }
        ]
    } />
}


export function ParticipantOverview() {
    let currentView = {type: "ParticipantsList", tournament_uuid: "00000000-0000-0000-0000-000000000001"};

    let participants = useView(currentView, {"teams": [], "adjudicators": []});

    return <div>
        <ParticipantTable participants={participants} />
    </div>
}