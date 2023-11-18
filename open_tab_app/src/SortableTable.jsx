import React from "react";
import { useState, useMemo } from "react";

export function SortableTable({ selectedRowId, onSelectRow, rowId, rowStyler, ...props }) {
    let [sortOrder, setSortOrder] = useState(null);

    let rowStylerFn = rowStyler ?? (() => "");

    let { orderedRows, groups } = useMemo(
        () => {
            let orderedRows = [...props.data];
            if (sortOrder !== null) {
                orderedRows.sort((a, b) => (a[sortOrder.key] > b[sortOrder.key] ? 1 : -1) * sortOrder.direction);
            }

            let colGroups = new Map();
            for (let col of props.columns) {
                if (col.group) {
                    colGroups.set(col, []);
                }
            }

            for (let i = 0; i < orderedRows.length; i++) {
                let row = orderedRows[i];
                for (let [col, groups] of colGroups) {
                    let currentGroup = groups[groups.length - 1];
                    if (currentGroup === undefined || currentGroup.val !== row[col.key]) {
                        currentGroup = { val: row[col.key], size: 1, start: i };
                        groups.push(currentGroup);
                    }
                    else {
                        currentGroup.size += 1;
                        groups.push(currentGroup);
                    }
                }
            }

            return { orderedRows, groups: colGroups };
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
        };
    }

    return <div className="h-full overflow-scroll">
        <table className="w-full">
            <thead className="bg-white sticky top-0">
                <tr className="text-left">
                    {props.columns.map((column, idx) => {
                        return <th key={idx} className="" onClick={handleSort(column.key)}>{column.header}</th>;
                    })}
                </tr>
            </thead>
            <tbody>
                {orderedRows.map((row, rowIdx) => {
                    let className = [selectedRowId === row[rowId] ? "bg-sky-500" : (rowIdx % 2 == 0 ? "bg-gray-100" : "bg-white")].join(" ");
                    className += " " + rowStylerFn(rowIdx, row);

                    return <tr key={row[rowId]} className={className} onClick={() => onSelectRow && onSelectRow(row[rowId])}>
                        {props.columns.filter(col => !col.group || groups.get(col)[rowIdx].start == rowIdx).map(
                            (column, idx) => {
                                let val = row[column.key];
                                let rowSpan = groups.get(column)?.[rowIdx]?.size ?? 1;

                                return column.cellFactory !== undefined ? column.cellFactory(val, rowIdx, idx, row) : <td rowSpan={rowSpan} key={idx} className="">{val}</td>;
                            }
                        )}
                    </tr>;
                })}
            </tbody>
        </table>
    </div>;
}


export function EditableCell(props) {
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
