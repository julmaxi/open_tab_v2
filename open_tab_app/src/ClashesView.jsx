import { useContext } from "react";
import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { SortableTable } from "./SortableTable";
import { TabGroup, Tab } from "./TabGroup";
import Button from "./UI/Button";
import { executeAction } from "./Action";

function ClashList({clashes, canIgnore, onUpdate, emptyText}) {
    return <SortableTable class="w-full" columns={[
        { key: "declaring_participant_name", header: "Declared by" },
        { key: "target_name", header: "Towards", cellFactory: (val, rowIdx, idx, row) => {
            return <td>
                <span style={
                row.is_retracted ? {
                    "text-decoration": "line-through",
                } : {}
            }>{row["target_name"]}</span> {row.is_retracted ? " (Remove)" : ""} </td>;
        }},
        { key: "actions", header: "Actions", cellFactory: (val, rowIdx, idx, row) => {

            return <td className="w-52">
                {
                    row["is_retracted"] ?
                        <Button
                            className={"rounded-none rounded-l bg-red-500"}
                            onClick={() => onUpdate({
                                "type": row["type"],
                                "target_id": row["uuid"],
                                "action": "remove",
                                "declaration_id": row["declaration_id"],
                                "declaring_participant_id": row["declaring_participant_uuid"],
                            })}
                        >
                            Remove Clash
                        </Button>
                        :
                        <Button
                            className={"rounded-none rounded-l bg-green-500"}
                            onClick={() => onUpdate({
                                "type": row["type"],
                                "target_id": row["uuid"],
                                "action": "add",
                                "declaration_id": row["declaration_id"],
                                "declaring_participant_id": row["declaring_participant_uuid"],
                            })}
                        >
                            Add Clash
                        </Button>
                }
                {
                    canIgnore ?
                        <Button
                            role="ignore"
                            className={"rounded-none rounded-r"}
                            onClick={() => onUpdate({
                                "type": row["type"],
                                "target_id": row["uuid"],
                                "action": "ignore",
                                "declaration_id": row["declaration_id"],
                                "declaring_participant_id": row["declaring_participant_uuid"],
                            })}
                        >
                            Ignore
                        </Button>
                        :
                        []
                }
            </td>;
        }}
    ]} data={clashes} rowId={"uuid"} row_id={"declaration_id"} selectedRowId={-1} emptyText={emptyText} />;
}

export default function ClashesView() {
    let { uuid } = useContext(TournamentContext);
    let clashes = useView(
        {
            "type": "Clashes",
            "tournament_uuid": uuid,
        },
        {
            pending_clashes: [],
            approved_clashes: [],
            rejected_clashes: [],
        }
    );

    function updateClashApproval({type, declaring_participant_id, target_id, action, declaration_id}) {
        let actionParams = {
            tournament_id: uuid,
        };

        console.log(type);
        if (type === "Institution") {
            actionParams.institution_read_declarations = [declaration_id];
        }
        else {
            actionParams.clash_read_declarations = [declaration_id];
        }

        if (action === "remove") {
            if (type === "Institution") {
                actionParams.deleted_institution_clashes = [
                    [declaring_participant_id, target_id]
                ];
            }
            else {
                actionParams.deleted_participant_clashes = [
                    [declaring_participant_id, target_id]
                ];
            }
        }
        else if (action === "add") {
            if (type === "Institution") {
                actionParams.added_institution_clashes = [
                    [declaring_participant_id, target_id]
                ];
            }
            else {
                actionParams.added_participant_clashes = [
                    [declaring_participant_id, target_id]
                ];
            }
        }
        executeAction(
            "UpdateClashes",
            actionParams,
        )    
    }

    return (
        <div className="w-full">

        <TabGroup initialActiveTab={0}>
            <Tab name={`Pending${clashes.pending_clashes.length > 0 ? " (" + clashes.pending_clashes.length + ")" : ""}`} autoScroll={false}>
                <ClashList
                    clashes={clashes.pending_clashes}
                    canAccept={true}
                    canIgnore={true}
                    onUpdate={updateClashApproval}
                    emptyText="There are no pending clashes. If you allow participants to declare clashes, they will appear here for you to approve or reject. Participants will not be able to see your decision."
                />
            </Tab>
            <Tab name="Rejected" autoScroll={false}>
                <ClashList
                    clashes={clashes.rejected_clashes}
                    canAccept={true}
                    canReject={false}
                    onUpdate={updateClashApproval}
                />
            </Tab>
      </TabGroup>
        </div>
    );
}