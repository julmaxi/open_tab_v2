import React from "react";
import { useState, useMemo } from "react";

export function SortableTable({ selectedRowId, onSelectRow, ...props }) {
    let [sortOrder, setSortOrder] = useState(null);

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
                    let className = [selectedRowId === row[props.row_id] ? "bg-sky-500" : (rowIdx % 2 == 0 ? "bg-gray-100" : "bg-white")].join(" ");

                    return <tr key={row[props.row_id]} className={className} onClick={() => onSelectRow(row[props.row_id])}>
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
