import { Outlet, useNavigate, useParams } from "react-router"
import { TournamentContext } from "./TournamentContext";
import { useContext, useMemo, useState } from 'react';
import { useView } from "./View";
import ContentView from "./ContentView";
import { SortableTable } from "./SortableTable";

function get_cell_value(value) {
    switch (value.type) {
        case "Average":
            return value.avg.toFixed(2);
        case "Percentage":
            return (value.percentage * 100).toFixed(2) + "%";
        default:
            return "-"
    }
}

export function FeedbackOverviewTable() {
    let context = useContext(TournamentContext);
    let feedback_overview = useView({type: "FeedbackOverview", tournament_uuid: context.uuid}, null);

    let [selectedParticipantIds, setSelectedParticipantIds] = useState([]);
    let flatData = useMemo(() => {
        return feedback_overview && feedback_overview.participant_entries.map((participant) => {
            let result = {
                participant_name: participant.participant_name,
                participant_id: participant.participant_id,
            };
            for (let column of feedback_overview.summary_columns) {
                result[column.question_id] = get_cell_value(participant.score_summaries[column.question_id]);
            }
            return result;
        })
    }, [feedback_overview]);

    if (feedback_overview == null) {
        return <div>Loading...</div>
    }

    return <div className="w-full h-full bla">
        <ContentView defaultDrawerWidth={400}  forceOpen={selectedParticipantIds.size > 0}>
        <ContentView.Content>
            <SortableTable columns={[{
                header: "Participant",
                key: "participant_name",
            }, ...feedback_overview.summary_columns.map((column) => ({
                header: column.title,
                key: column.question_id,
            }) )]} data={flatData} rowId={"participant_id"} allowMultiSelect={true} selectedRowIds={selectedParticipantIds} onSelectRow={(newIds => {
                setSelectedParticipantIds(newIds);
            })} />
        </ContentView.Content>
        <ContentView.Drawer>
            <div className="w-full h-full overflow-auto flex flex-col">
                {
                    [...selectedParticipantIds].map((participantId) => {
                        return <div className="flex-1"><FeedbackDetailView participantId={participantId} /></div>
                    })
                }
            </div>
        </ContentView.Drawer>
        </ContentView>
    </div>
}

export function FeedbackOverviewRoute(props) {
    return <FeedbackOverviewTable />
}

function ValCell({value}) {
    if (value === null || value === undefined) return <td></td>;
    switch (value.type) {
        case "Int":
            return <td className="border text-center">{value.val}</td>
        case "Bool":
            return <td className="border text-center">{value.val ? "✔️": "❌"}</td>
        default:
            console.log(value);
    }
}

function FeedbackResponseDetails(props) {
    let { response } = props;
    let table_values = response.values.filter((value) => value.value.type != "String");
    table_values.sort((a, b) => a.question_short_name.localeCompare(b.question_short_name));
    let string_values = response.values.filter((value) => value.value.type == "String");

    return <div className="p-1 pb-2 w-full border-b-4 border-gray-400 flex justify-center flex-col">
        <h1 className="font-bold">{response.author_name} in {response.round_name}</h1>
        <table className="border bg-white">
            <thead>
                <tr>
                    {table_values.map(
                        (value, idx) => <th className="border" key={idx}>{value.question_short_name}</th>
                    )}
                </tr>
            </thead>
            <tbody>
                <tr>
                    {table_values.map(
                        (value, idx) => <ValCell key={idx} value={value.value} />
                    )}
                </tr>
            </tbody>
        </table>
        
        {
            string_values.map((value, idx) => <div key={idx}>
                <h2 className="font-bold border-t">{value.question_short_name}</h2>
                <p>{value.value.val}</p>
            </div>)
        }
    </div>
}

export function FeedbackDetailView({participantId}) {
    let responses = useView({type: "FeedbackDetail", participant_id: participantId}, null);

    if (responses == null) {
        return <div>Loading...</div>
    }

    return <div className="w-full overflow-auto">
        <h1 className="font-bold">{responses.participant_name}</h1>
        {responses.responses.map((response, idx) => <FeedbackResponseDetails response={response} key={idx} />)}
    </div>

}
