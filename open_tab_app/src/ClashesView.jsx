import { useContext } from "react";
import { useView } from "./View";
import { TournamentContext } from "./TournamentContext";
import { SortableTable } from "./SortableTable";
import { TabGroup, Tab } from "./TabGroup";
import Button from "./UI/Button";
import { executeAction } from "./Action";

function ClashList({clashes, canAccept, canReject, onUpdate, emptyText}) {
    return <SortableTable class="w-full" columns={[
        { key: "declaring_participant_name", header: "Declared by" },
        { key: "target_participant_name", header: "Towards" },
        { key: "actions", header: "Actions", cellFactory: (val, rowIdx, idx, row) => {
            return <td className="w-52">
                {(row.is_user_declared && canAccept) ? <Button role="approve" className={canReject ? "rounded-none rounded-l" : []} onClick={() => onUpdate({"clash_id": row["clash_id"], "is_accepted": true})}>Accept</Button> : []}
                {(row.is_user_declared && canReject) ? <Button role="danger" className={canAccept ? "rounded-none rounded-r" : []} onClick={() => onUpdate({"clash_id": row["clash_id"], "is_accepted": false})}>Reject</Button> : []}
                {!row.is_user_declared ? <Button role="danger" onClick={() => onUpdate({"clash_id": row["clash_id"], "delete_clash": true})}>Delete</Button> : []}
            </td>;
        }}
    ]} data={clashes} rowId={"uuid"} row_id={"clash_id"} selectedRowId={-1} emptyText={emptyText} />;
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

    function updateClashApproval({clash_id, is_accepted, delete_clash}) {
        if (delete_clash) {
            confirm("Are you sure you want to delete this clash? You can not undo this.").then((result) => {
                if (result === true) {
                    executeAction(
                        "UpdateClashes",
                        {
                            tournament_id: uuid,
                            deleted_clashes: [clash_id]
                        },
                    )    
    }
            });
        }
        else {
            executeAction(
                "UpdateClashes",
                {
                    tournament_id: uuid,
                    updated_clashes: [
                        {
                            clash_id: clash_id,
                            approve: is_accepted,
                        }
                    ]
                },
            )    
        }
    }

    return (
        <div className="w-full">

        <TabGroup initialActiveTab={clashes.pending_clashes.length > 0 ? 0 : 1}>
            <Tab name={`Pending${clashes.pending_clashes.length > 0 ? " (" + clashes.pending_clashes.length + ")" : ""}`} autoScroll={false}>
                <ClashList
                    clashes={clashes.pending_clashes}
                    canAccept={true}
                    canReject={true}
                    onUpdate={updateClashApproval}
                    emptyText="There are no pending clashes. If you allow participants to declare clashes, they will appear here for you to approve or reject. Participants will not be able to see your decision."
                />
            </Tab>
            <Tab name="Accepted">
                <ClashList
                    clashes={clashes.approved_clashes}
                    canAccept={false}
                    canReject={true}
                    onUpdate={updateClashApproval}
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