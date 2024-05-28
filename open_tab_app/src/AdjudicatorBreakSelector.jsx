import { useState } from "react";
import Button from "./UI/Modal";
import { useView } from "./View";
import { SortableTable } from "./SortableTable";

export function AdjudicatorBreakSelector({ nodeId, onSuccess, onAbort, ...props }) {
    let adjudicators = useView(
        { type: "AdjudicatorBreakCandidates", node_uuid: nodeId },
        {
            adjudicators: []
        }
    ).adjudicators;

    let [breakingAdjs, setBreakingAdjs] = useState(new Set());

    return <div className="flex flex-col">
        <h1>Adjudicator Break</h1>

        <div className="flex-1 min-h-0">
            <SortableTable columns={[
                { key: "name", header: "Name" },
                {
                    key: "should_break", header: "Break?", cellFactory: (val, rowIdx, idx, row) => {
                        return <td><input type="checkbox" checked={val} onChange={(e) => {
                            let newBreakingAdjs = new Set(breakingAdjs);
                            if (e.target.checked) {
                                newBreakingAdjs.add(row.uuid);
                            }
                            else {
                                newBreakingAdjs.delete(row.uuid);
                            }

                            setBreakingAdjs(newBreakingAdjs);
                        }} /></td>;
                    }
                }
            ]} data={adjudicators} rowId={"uuid"} rowStyler={(rowIdx) => {
                let outClasses = "";
                if (!adjudicators[rowIdx].is_in_previous_break) {
                    outClasses += "text-gray-600 line-through";
                }
                if (adjudicators[rowIdx].clash_state == "NoClashes") {
                    outClasses += " bg-green-500";
                }
                else if (adjudicators[rowIdx].clash_state == "SomeClashes") {
                    outClasses += " bg-yellow-500";
                }
                else if (adjudicators[rowIdx].clash_state == "FullyClashed") {
                    outClasses += " bg-red-500";
                }
                return outClasses;
            }} alternateRowColors={false} />
        </div>

        <div className="pt-2">
            <p className="text-xs">
                Colors indicate number of clashes with breaking teams.
                <span className="text-green-500">No Clashes </span>

                <span className="text-yellow-500">Some Clashes </span>

                <span className="text-red-500">Fully Clashed </span>
            </p>
            <Button role="secondary" onClick={onAbort}>Abort</Button>
            <Button role="primary" onClick={() => {
                onSuccess(
                    [...breakingAdjs]
                );
            }}>Set Break</Button>

        </div>
    </div>;
}
