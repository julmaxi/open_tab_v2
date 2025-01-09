import React from "react";
import { useState, useMemo } from "react";

export function SortableTable({ selectedRowId, selectedRowIds, onSelectRow, rowId, rowStyler, alternateRowColors: alternateRowColors = true, allowMultiSelect: allowMultiSelect = false, emptyText, ...props }) {
    let [sortOrder, setSortOrder] = useState(null);

    let realSelectedRowIds;
    if (allowMultiSelect) {
        realSelectedRowIds = useMemo(() => {
            return new Set(selectedRowIds);
        }, [selectedRowIds]);
    }
    else {
        realSelectedRowIds = useMemo(() => {
            let realSelectedRowIds = new Set();
            realSelectedRowIds.add(selectedRowId);
            return realSelectedRowIds;
        }, [selectedRowId]);
    }

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
                        currentGroup = { val: row[col.key], size: 1, start: i, isHighlighted: realSelectedRowIds.has(row[rowId]) };
                    }
                    else {
                        currentGroup.size += 1;
                        currentGroup.isHighlighted = currentGroup.isHighlighted || realSelectedRowIds.has(row[rowId]);
                    }
                    groups.push(currentGroup);
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

    return <div className="h-full overflow-auto">
        <table className="w-full select-none">
            <thead className="bg-white sticky top-0">
                <tr className="text-left">
                    {props.columns.map((column, idx) => {
                        return <th key={idx} className="" onClick={handleSort(column.key)}>{column.header}</th>;
                    })}
                </tr>
            </thead>
            <tbody>
                {orderedRows.map((row, rowIdx) => {
                    let className = [];
                    if (realSelectedRowIds.has(row[rowId])) {
                        className.push("bg-blue-300 ");
                    }
                    
                    if (alternateRowColors && !realSelectedRowIds.has(row[rowId])) {
                        className.push(rowIdx % 2 == 0 ? "bg-gray-100" : "bg-white");
                    }

                    className.push(rowStylerFn(rowIdx, row));
                    return <tr key={row[rowId]} className={className.join(" ")} onClick={(e) => {
                        if (onSelectRow) {
                            if (!allowMultiSelect) {
                                onSelectRow(row[rowId]);
                            }
                            else {
                                if (e.shiftKey) {
                                    let newSelection = new Set(realSelectedRowIds);
                                    if (realSelectedRowIds.has(row[rowId])) {
                                        newSelection.delete(row[rowId]);
                                    }
                                    else {
                                        newSelection.add(row[rowId]);
                                    }
                                    onSelectRow(newSelection);
                                }
                                else {
                                    onSelectRow(new Set([row[rowId]]));
                                }
                            }
                        }
                        e.stopPropagation();
                    }}>
                        {props.columns.filter(col => !col.group || groups.get(col)[rowIdx].start == rowIdx).map(
                            (column, idx) => {
                                let val = row[column.key];
                                val = (column.transform || ((val) => val))(val);
                                let rowSpan = groups.get(column)?.[rowIdx]?.size ?? 1;

                                return column.cellFactory !== undefined ? column.cellFactory(val, rowIdx, idx, row) : <td rowSpan={rowSpan} key={idx} className={groups.get(column)?.[rowIdx]?.isHighlighted ? "bg-blue-300" : ""}>{val}</td>;
                            }
                        )}
                    </tr>;
                })}
            </tbody>
        </table>
        {orderedRows.length == 0 && emptyText ? <p className="text-gray-500">{emptyText}</p> : []}
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
